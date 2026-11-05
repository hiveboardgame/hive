#!/usr/bin/env python3
"""Regression tests for the fair-play detector.

    py -3 -m unittest discover -s scripts/fair-play -p 'test_*.py' -v

Two of these exist because the bug actually happened during development and would
have produced accusation files about the CHEATER'S VICTIMS:
    test_victim_is_never_the_subject
    test_unattributable_pair_implicates_nobody
Do not delete them.

Everything under "found by adversarial review" is a bug that shipped. Mutation testing
had 100% coverage of the rules and caught none of them, because every one is either a
boundary or an interaction between two rules that are individually fine. Tests that
assert a rule EXISTS are cheap; these assert the rules are RIGHT and that they compose.
"""
from __future__ import annotations

import contextlib
import hashlib
import io
import collections
import itertools
import json
import os
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mirror_scan as MS          # noqa: E402
import registry as REG            # noqa: E402
from registry import Registry     # noqa: E402

_ids = itertools.count(1)


def mkgame(gid, w, b, *, shared=(), wbot=False, bbot=False, gt="MLP",
           speed="Correspondence", rated=True, tour=None, start=0.0, end=1000.0,
           wr=1800.0, br=1800.0, prefix_len=12, timing_trusted=True):
    """A game record shaped exactly like mirror_scan.load() emits.

    `shared` are the hashes placed at ply >= MIN_PLY; everything before is unique
    filler so the opening never contributes.
    """
    hashes = [next(_ids) * 1_000_003 for _ in range(prefix_len)] + list(shared)
    return {
        "gid": gid, "w": w, "b": b,
        "wname": str(w), "bname": str(b),
        "wbot": wbot, "bbot": bbot,
        "wrc": 0.0, "brc": 0.0, "wr": wr, "br": br,
        "gt": gt, "speed": speed, "rated": rated, "tour": tour,
        "status": "Finished", "conclusion": "Finished",
        "start": start, "end": end,
        "end_src": "last_interaction", "timing_trusted": timing_trusted,
        "plies": len(hashes), "hashes": hashes,
    }


def filler(n=300):
    """Background corpus: unique positions, so it only sets the surprisal scale."""
    return [mkgame(f"filler{i}", f"u{i}a", f"u{i}b",
                   shared=[next(_ids) * 7_000_003 for _ in range(20)])
            for i in range(n)]


SHARED = [900_000_000 + i for i in range(20)]      # 20 positions, df=2 -> ample bits


class DetectionTests(unittest.TestCase):
    def scan(self, *games):
        return MS.scan(list(games) + filler())

    def test_detects_full_mirror_against_a_bot(self):
        played = mkgame("g1", "cheater", "victim", shared=SHARED)
        oracle = mkgame("g2", "cheater", "botacct", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1, "the canonical mirror must be detected")
        real, orac = MS.classify(s[0])
        self.assertEqual(real["gid"], "g1")
        self.assertEqual(orac["gid"], "g2")

    def test_grades_a_whole_game_mirror_as_top_priority(self):
        played = mkgame("g1", "cheater", "victim", shared=[900_000_000 + i for i in range(45)])
        oracle = mkgame("g2", "cheater", "botacct", bbot=True, start=10.0, end=900.0,
                        shared=[900_000_000 + i for i in range(45)])
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1)
        tier, _why = MS.grade(s[0])
        self.assertEqual(tier, "p1")

    def test_no_tier_claims_certainty(self):
        """Relaying and using a bot as a live analysis board are identical in every
        recorded field. A tier called `conclusive` invites a reader to skip the
        caveat saying we cannot tell them apart."""
        self.assertNotIn("conclusive", MS.TIERS)

    def test_p1_needs_a_strictly_unbroken_run_not_a_gappy_chain(self):
        """Found by external review. `chain` tolerates gaps up to RUN_GAP (6) — right
        for the evidence score, wrong for the word 'unbroken'. grade() used `chain`, so
        a run of 40 positions with several 3-ply gaps was graded P1 and printed as
        'one unbroken run of 40'. P1 must gate on a strict max_gap=1 run.

        Two colour-parity-preserving gaps (even offsets stay even) inside an otherwise
        long shared line: the pair is real evidence (P2), but not a whole-game mirror."""
        # Both games share block A then block B, but the oracle has 4 extra plies
        # BETWEEN them that the real game does not. So A matches at offset 0 and B at
        # offset +4: a 4-ply jump the RUN_GAP=6 chain swallows into one run of 40, while
        # a strict max_gap=1 run sees two runs of 20.
        blockA = [970_000_000 + i for i in range(20)]
        blockB = [971_000_000 + i for i in range(20)]
        midonly = [999_000_000 + i for i in range(4)]      # oracle-only, never matches
        played = mkgame("g1", "cheater", "victim", shared=blockA + blockB,
                        start=0.0, end=1000.0)
        oracle = mkgame("g2", "cheater", "botacct", shared=blockA + midonly + blockB,
                        bbot=True, start=10.0, end=900.0)
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1)
        p = s[0]
        strict = max((len(r) for r in MS.runs_of(p["pts"], max_gap=1)), default=0)
        chain = p["chain"]
        self.assertGreaterEqual(chain, MS.P1_MIN_RUN,
                                "the gappy chain clears the P1 bar (that is the bug)")
        self.assertLess(strict, MS.P1_MIN_RUN,
                        "but the strict contiguous run does not")
        tier, why = MS.grade(p)
        self.assertNotEqual(tier, "p1", "a gappy chain must not be graded P1")
        self.assertNotIn("unbroken run of 40", why)

    # -- exemptions ------------------------------------------------------
    def test_rematch_between_the_same_two_players_is_ignored(self):
        a = mkgame("g1", "p1", "p2", shared=SHARED)
        b = mkgame("g2", "p2", "p1", shared=SHARED, start=10.0)
        self.assertEqual(len(self.scan(a, b)[0]), 0)

    def test_bot_versus_bot_is_out_of_scope(self):
        a = mkgame("g1", "b1", "b2", shared=SHARED, wbot=True, bbot=True)
        b = mkgame("g2", "b3", "b4", shared=SHARED, wbot=True, bbot=True, start=10.0)
        self.assertEqual(len(self.scan(a, b)[0]), 0)

    def test_games_that_never_overlapped_are_ignored(self):
        a = mkgame("g1", "cheater", "victim", shared=SHARED, start=0.0, end=100.0)
        b = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True,
                   start=5000.0, end=6000.0)
        self.assertEqual(len(self.scan(a, b)[0]), 0)

    def test_game_type_mismatch_is_ignored(self):
        # the canonical hash omits game_type, so Base and Base+MLP positions collide
        a = mkgame("g1", "cheater", "victim", shared=SHARED, gt="Base")
        b = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True, gt="MLP", start=10.0)
        self.assertEqual(len(self.scan(a, b)[0]), 0)

    def test_a_players_own_repertoire_is_not_evidence(self):
        played = mkgame("g1", "cheater", "victim", shared=SHARED)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True, start=10.0)
        # the same person reached these positions in three earlier games of their own
        own = [mkgame(f"own{i}", "cheater", f"other{i}", shared=SHARED,
                      start=-10000.0, end=-9000.0) for i in range(3)]
        s, _ = MS.scan([played, oracle] + own + filler())
        self.assertEqual(len(s), 0, "a pet line played repeatedly must not flag")

    # -- evidence floor --------------------------------------------------
    def test_positions_common_across_many_games_do_not_clear_the_floor(self):
        common = [800_000_000 + i for i in range(3)]
        crowd = [mkgame(f"crowd{i}", f"c{i}a", f"c{i}b", shared=common, start=10.0)
                 for i in range(7)]
        oracle = mkgame("oracle", "c0a", "bot", shared=common, bbot=True, start=20.0)
        s, dropped = MS.scan(crowd + [oracle] + filler())
        self.assertEqual(len(s), 0)

    def test_enough_rare_positions_do_clear_the_floor(self):
        rare = [910_000_000 + i for i in range(10)]
        played = mkgame("g1", "cheater", "victim", shared=rare)
        oracle = mkgame("g2", "cheater", "bot", shared=rare, bbot=True, start=10.0)
        self.assertEqual(len(self.scan(played, oracle)[0]), 1)

    # -- live window -----------------------------------------------------
    def test_oracle_opened_late_is_treated_as_post_game_analysis(self):
        played = mkgame("g1", "player", "opponent", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "player", "bot", shared=SHARED, bbot=True,
                        start=700.0, end=1000.0)      # 70% in
        self.assertEqual(len(self.scan(played, oracle)[0]), 0)

    def test_an_oracle_that_covered_little_of_the_game_is_dropped(self):
        """The live-window test asks 'was the oracle open while the game was being
        played', i.e. overlap / REAL duration. It used to ask 'did the oracle run
        inside the real game', i.e. overlap / ORACLE duration, which was both
        exploitable and backwards -- see MIN_COVERAGE."""
        played = mkgame("g1", "player", "opponent", shared=SHARED, start=0.0, end=1000.0)
        # opens inside the window but closes almost immediately: covers 5%
        oracle = mkgame("g2", "player", "bot", shared=SHARED, bbot=True,
                        start=100.0, end=150.0)
        self.assertEqual(len(self.scan(played, oracle)[0]), 0)

    def test_an_oracle_open_for_the_whole_game_is_the_suspicious_shape(self):
        """The old statistic penalised this hardest: an oracle spanning the entire
        real game had a low overlap/oracle-duration ratio and was dropped as
        'analysis'. It is the most suspicious shape there is."""
        played = mkgame("g1", "player", "opponent", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "player", "bot", shared=SHARED, bbot=True,
                        start=20.0, end=1200.0)
        self.assertEqual(len(self.scan(played, oracle)[0]), 1)

    # -- oracle must be a source of strength -----------------------------
    def test_human_oracle_no_stronger_than_subject_is_rejected(self):
        played = mkgame("g1", "subject", "opp", shared=SHARED, wr=1800.0, br=1800.0)
        oracle = mkgame("g2", "subject", "peer", shared=SHARED, start=10.0,
                        wr=1800.0, br=1810.0)
        self.assertEqual(len(self.scan(played, oracle)[0]), 0)

    def test_human_oracle_in_a_faster_format_is_rejected(self):
        played = mkgame("g1", "subject", "opp", shared=SHARED,
                        speed="Correspondence", wr=1800.0, br=1800.0)
        oracle = mkgame("g2", "subject", "gm", shared=SHARED, start=10.0,
                        speed="Blitz", wr=1800.0, br=2600.0)
        self.assertEqual(len(self.scan(played, oracle)[0]), 0)

    def test_strong_slow_human_oracle_is_kept(self):
        played = mkgame("g1", "subject", "opp", shared=SHARED,
                        speed="Correspondence", wr=1800.0, br=1800.0)
        oracle = mkgame("g2", "subject", "gm", shared=SHARED, start=10.0,
                        speed="Correspondence", wr=1800.0, br=2600.0)
        self.assertEqual(len(self.scan(played, oracle)[0]), 1)

    # -- attribution: the bugs that actually happened --------------------
    def test_victim_is_never_the_subject(self):
        """REGRESSION: an early build wrote accusation files about the victim."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True, start=10.0)
        s, _ = self.scan(played, oracle)
        real, orac = MS.classify(s[0])
        subject = ({real["w"], real["b"]} & {orac["w"], orac["b"]}) - {None}
        self.assertEqual(subject, {"cheater"})
        self.assertNotIn("victim", subject)

    def test_unattributable_pair_implicates_nobody(self):
        """REGRESSION: alt-account pairs must not name the real game's players.

        Nothing in position data says which of the two is the beneficiary, and one
        of them is somebody's victim."""
        played = mkgame("g1", "mainacct", "victim", shared=SHARED)
        oracle = mkgame("g2", "altacct", "bot", shared=SHARED, bbot=True, start=10.0)
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1)
        real, orac = MS.classify(s[0])
        shared_acct = ({real["w"], real["b"]} & {orac["w"], orac["b"]}) - {None}
        self.assertEqual(shared_acct, set(), "no account is in both games")
        tier, _ = MS.grade(s[0])
        self.assertEqual(tier, "p3", "an unlinked pair is never top priority")

    def test_later_game_is_treated_as_the_oracle(self):
        first = mkgame("g1", "subject", "opp", shared=SHARED, start=0.0, end=1000.0,
                       wr=1800.0, br=1800.0)
        later = mkgame("g2", "subject", "gm", shared=SHARED, start=50.0, end=900.0,
                       speed="Correspondence", wr=1800.0, br=2600.0)
        s, _ = self.scan(first, later)
        real, orac = MS.classify(s[0])
        self.assertEqual(real["gid"], "g1")
        self.assertEqual(orac["gid"], "g2", "you cannot mirror a game that started later")


# =====================================================================
# Found by adversarial review. Every one of these shipped.
# =====================================================================
class RegressionTests(unittest.TestCase):
    def scan(self, *games):
        return MS.scan(list(games) + filler())

    def test_bot_game_is_the_oracle_even_when_it_started_first(self):
        """F4. classify() decided roles by start order before checking for a bot seat,
        so opening the analysis board 61 seconds early cast the RATED game as a
        'human oracle', failed it on MIN_ORACLE_EDGE, and erased the pair."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "cheater", "botacct", shared=SHARED, bbot=True,
                        start=-60.0, end=1000.0)          # opened BEFORE the real game
        real, orac = MS.classify({"a": played, "b": oracle})
        self.assertEqual(real["gid"], "g1", "a bot game is never the game being played")
        self.assertEqual(orac["gid"], "g2")
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1, "opening the oracle early must not be an escape")

    def test_reported_position_count_is_the_counted_evidence(self):
        """F3. Candidacy used MAX_FANOUT=8 while novel/bits/grade used MAX_DF=50, so
        a pair with three rare positions was reported as '43 positions reproduced
        against a bot' and graded on positions already ruled too common to count."""
        rare = [960_000_000 + i for i in range(3)]
        midpop = [970_000_000 + i for i in range(40)]      # df 22: over the fan-out cap
        crowd = [mkgame(f"cr{i}", f"x{i}a", f"x{i}b", shared=midpop, start=5.0)
                 for i in range(20)]
        played = mkgame("g1", "cheater", "victim", shared=rare + midpop,
                        start=0.0, end=1000.0)
        oracle = mkgame("g2", "cheater", "botacct", shared=rare + midpop, bbot=True,
                        start=10.0, end=900.0)
        games = [played, oracle] + crowd + filler()
        index = MS.build_index(games)
        s, _ = MS.find_pairs(games, index)
        for p in s:
            for h in p["novel"]:
                self.assertTrue(MS._eligible(index, h),
                                "a position too common to detect on is too common to score")
            self.assertLessEqual(len(p["novel"]), len(rare))

    def test_a_later_game_cannot_retroactively_erase_evidence(self):
        """F7. rep was a corpus-wide count with no time filter, so ONE game played a
        year afterwards deleted the pair on the next scan -- a free, permanent,
        published exemption."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        later = [mkgame(f"after{i}", "cheater", f"someone{i}", shared=SHARED,
                        start=9_000_000.0 + i, end=9_100_000.0 + i) for i in range(3)]
        s, _ = MS.scan([played, oracle] + later + filler())
        self.assertEqual(len(s), 1, "games played AFTER the pair are not a repertoire")

    def test_the_victims_repertoire_does_not_exonerate_the_subject(self):
        """F7. The repertoire check ran over `pa | pb`, so targeting an opponent with
        a well-known pet defence made the relay invisible."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        vic = [mkgame(f"v{i}", "victim", f"other{i}", shared=SHARED,
                      start=-99000.0, end=-98000.0) for i in range(3)]
        s, _ = MS.scan([played, oracle] + vic + filler())
        self.assertEqual(len(s), 1, "the opponent's openings say nothing about the subject")

    def test_one_shared_line_does_not_become_a_clique_of_accusations(self):
        """F2. k games on one line yield C(k,2) pairs. Eight games produced 28
        'survivors', inflating the headline and hubbing the moderator queue."""
        line = [990_000_000 + i for i in range(10)]
        cluster = [mkgame(f"h{i}", f"pa{i}", f"pb{i}", shared=line, start=0.0, end=1000.0)
                   for i in range(7)]
        cluster.append(mkgame("hub", "hubacct", "nokamute", shared=line, bbot=True,
                              start=10.0, end=900.0))
        # alpha pinned to the sensitivity this mechanism test was written for: it checks
        # the CLUSTER split, and must reach it rather than have pairs dropped earlier by
        # the higher shipped floor (now 1e-3).
        s, dropped = MS.scan(cluster + filler(), 0.01)
        self.assertEqual(len(s), 0, "a shared opening line is not eight accusations")
        self.assertTrue(any("cluster" in k for k in dropped), dict(dropped))

    def test_queue_is_ordered_by_evidence_not_by_pair_count(self):
        """F1. grade() had no caller at all, and main() sorted by len(pairs), so the
        account sitting in a popular line's bot game outranked a whole-game mirror."""
        big = [950_000_000 + i for i in range(60)]
        real = mkgame("R", "cheat", "vic", shared=big, start=0.0, end=1000.0)
        orac = mkgame("O", "cheat", "bot", shared=big, bbot=True, start=10.0, end=900.0)
        s, _ = MS.scan([real, orac] + filler())
        self.assertEqual(len(s), 1)
        weak = dict(s[0])
        weak["bits"] = 1.0
        weak["chain"] = 1
        ranked = MS.rank_accounts({"noisy": [weak, weak, weak, weak], "cheat": [s[0]]})
        self.assertEqual(ranked[0][0], "cheat",
                         "one strong pair outranks four weak ones")

    def test_tournament_identity_is_the_id_not_the_snapshot(self):
        """F11. `tour` is a whole TournamentAbstractResponse carrying games_played and
        a player HashSet. Comparing the objects compared mutable counters captured at
        scrape time, so the exemption silently never fired."""
        snap_a = {"id": "T-1", "games_played": 4, "status": "InProgress"}
        snap_b = {"id": "T-1", "games_played": 9, "status": "Finished"}
        self.assertEqual(MS._tour_id(snap_a), MS._tour_id(snap_b))
        a = mkgame("g1", "p1", "p2", shared=SHARED, tour=MS._tour_id(snap_a))
        b = mkgame("g2", "p1", "p3", shared=SHARED, tour=MS._tour_id(snap_b), start=10.0)
        s, dropped = self.scan(a, b)
        self.assertEqual(len(s), 0)
        self.assertTrue(any("tournament" in k for k in dropped), dict(dropped))

    def test_every_caller_agrees_on_who_the_subject_is(self):
        """main() excluded bot seats when deciding who a pair implicates; grade(),
        write_case(), record_evidence() and _self_attributable() did not.

        They agree only while no account is ever flagged `bot` in one row and not in
        another -- true of all 5,756 accounts in the current archive, and an assumption
        about the data rather than something the code enforces. Where they diverged, a
        pair main() had ruled unattributable would be graded p2, written into a review
        file with BOTH real-game usernames printed, and logged to the registry as
        self-attributable. That is the shape that has produced accusation files about
        victims twice."""
        # seat inference is opt-in (D1); this test exercises it
        MS.SEAT_ENABLED = True
        self.addCleanup(setattr, MS, "SEAT_ENABLED", False)
        # `ghost` sits in the real game as a human and in the oracle game as the bot
        played = mkgame("g1", "ghost", "victim", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "altacct", "ghost", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        real, orac = MS.classify({"a": played, "b": oracle})
        self.assertEqual(MS.subject_of(real, orac), set(),
                         "a bot seat is never the account a pair implicates")
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1)
        tier, _why = MS.grade(s[0])
        self.assertEqual(tier, "p3", "an unattributable pair is never promoted")
        self.assertEqual(MS._self_attributable(s), 0)
        games = [played, oracle] + filler()
        rows = MS.account_stats(games, "altacct", set())
        fd, path = tempfile.mkstemp(suffix=".md")
        os.close(fd)
        try:
            MS.write_case(path, "altacct", rows, s, games)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
        finally:
            os.remove(path)
        # It is handled as the unattributable shape, so it goes through seat
        # correspondence — which states who is excluded — rather than being printed
        # as "X vs Y", which is what a self-attributable pair gets.
        self.assertIn("Excluded by this analysis", text)
        self.assertNotIn("ghost vs victim", text,
                         "it must not be rendered as a self-attributable pair")

    def test_untrusted_timing_fails_closed(self):
        """F6. updated_at is a row-mutation timestamp; for a timeout-finished game it
        is the DETECTION time. Inflating the real game's duration drives start_pct to
        zero and overlap to one, so the live-window test failed OPEN."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED, start=0.0, end=1000.0,
                        timing_trusted=False)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        s, dropped = self.scan(played, oracle)
        self.assertEqual(len(s), 0, "an untrustworthy end time must drop, not admit")
        self.assertTrue(any("timing" in k for k in dropped), dict(dropped))

    def test_load_prefers_last_interaction_and_quarantines_absurd_durations(self):
        """F6. last_interaction means 'when was the last move made' and is on the
        archive response; updated_at is not the game's end."""
        rows = [
            {"game_id": "ok", "created_at": "2026-01-01T00:00:00Z",
             "last_interaction": "2026-01-01T02:00:00Z",
             "updated_at": "2026-03-01T00:00:00Z", "speed": "Rapid",
             "game_start": "Moves",
             "white_player": {"uid": "a"}, "black_player": {"uid": "b"},
             "hashes": list(range(1, 40)), "history": [["x", "y"]] * 39},
            {"game_id": "no_li", "created_at": "2026-01-01T00:00:00Z",
             "updated_at": "2026-03-01T00:00:00Z", "speed": "Blitz",
             "game_start": "Moves",
             "white_player": {"uid": "c"}, "black_player": {"uid": "d"},
             "hashes": list(range(100, 140)), "history": [["x", "y"]] * 40},
        ]
        fd, path = tempfile.mkstemp(suffix=".jsonl")
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as fh:
                for r in rows:
                    fh.write(json.dumps(r) + "\n")
            games, rejected = MS.load(path)
        finally:
            os.remove(path)
        by = {g["gid"]: g for g in games}
        self.assertEqual(by["ok"]["end_src"], "last_interaction")
        self.assertEqual(by["ok"]["end"] - by["ok"]["start"], 7200.0,
                         "end must be the last move, not the last row update")
        self.assertTrue(by["ok"]["timing_trusted"])
        # 59 days of "Blitz" is a corrupt duration, and it came from updated_at
        self.assertFalse(by["no_li"]["timing_trusted"])
        self.assertTrue(any("implausible" in k for k in rejected), dict(rejected))

    def test_unrated_real_game_is_not_an_accusation(self):
        """F5. scan() ignored `rated` while account_stats() required it, so an unrated
        real game produced a file saying 'No flagged games' with the opponent named in
        the table underneath."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED, rated=False)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True, start=10.0)
        s, dropped = self.scan(played, oracle)
        self.assertEqual(len(s), 0)
        self.assertTrue(any("unrated" in k for k in dropped), dict(dropped))

    def test_unrated_bot_games_still_count_towards_the_account_picture(self):
        """F5. The site's own bot button creates UNRATED games, so filtering on
        `rated` emptied the vs-bots bucket and printed a row of zeroes directly above
        a table naming that very bot game."""
        games = [mkgame("g1", "subj", "victim", shared=SHARED, rated=True),
                 mkgame("g2", "subj", "bot", shared=SHARED, bbot=True, rated=False)]
        rows = MS.account_stats(games, "subj", {"g1"})
        self.assertEqual(len([r for r in rows if r["bot"]]), 1,
                         "an unrated bot game is still a bot game")

    def test_missing_opponent_rating_is_not_a_rating_of_zero(self):
        """F8. `opp_r or 0` fed sentinel zeroes to a median that the review file
        printed as evidence the account beat stronger opposition."""
        rows = [{"rated": True, "rc": 1.0, "plies": 40, "opp_rating": r, "bot": False}
                for r in (1866, 1870, 1860, None, None)]
        agg = MS._agg(rows)
        self.assertEqual(agg["med_opp"], 1866)
        self.assertEqual(agg["n_opp"], 3, "the report must say how many were recorded")

    def test_report_names_at_most_the_seat_that_received_the_moves(self):
        """F10, RESTATED — do not weaken this one either.

        The original bug printed BOTH real-game usernames in the alt-account shape, so
        one of the two people named was somebody's victim. The fix was to withhold
        both. Seat correspondence has since been turned on, so the report now names
        ONE of them — the seat the bot's colour maps to — and explicitly EXCLUDES the
        other.

        The guarantee F10 protects is unchanged, and it is what this asserts: the seat
        the analysis clears must be *stated as cleared*, never presented as the
        subject. Losing that is how this system accuses a victim.

        Here the bot is Black in the oracle, so the real game's Black seat received the
        moves and the White seat is cleared."""
        # seat inference is opt-in (D1); this test exercises it
        MS.SEAT_ENABLED = True
        self.addCleanup(setattr, MS, "SEAT_ENABLED", False)
        played = mkgame("g1", "mainacct", "victim_alice", shared=SHARED,
                        start=0.0, end=1000.0)
        oracle = mkgame("g2", "altacct", "bot", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        games = [played, oracle] + filler()
        s, _ = MS.scan(games)
        self.assertEqual(len(s), 1)
        recv_uid, _rn, other_uid, other_name = MS.seat_correspondence(s[0])
        self.assertEqual(recv_uid, "victim_alice")
        self.assertEqual(other_uid, "mainacct")
        rows = MS.account_stats(games, "altacct", set())
        fd, path = tempfile.mkstemp(suffix=".md")
        os.close(fd)
        try:
            MS.write_case(path, "altacct", rows, s, games)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
        finally:
            os.remove(path)
        self.assertIn("Excluded by this analysis", text,
                      "the cleared seat must be stated, not merely omitted")
        self.assertIn(other_name, text.split("Excluded by this analysis")[1],
                      "and must appear on the excluded side of the table")
        self.assertIn("g1", text, "the game id is still there for a database lookup")
        self.assertEqual(MS.grade(s[0])[0], "p3",
                         "resolving a seat does not make the pair more certain")

    def test_a_pair_whose_seat_cannot_be_resolved_still_names_nobody(self):
        """The fallback F10 leaves behind. With no bot seat to map, or ply offsets
        that are not all even, neither real-game player is named at all."""
        played = mkgame("g1", "mainacct", "victim_alice", shared=SHARED,
                        start=0.0, end=1000.0)
        oracle = mkgame("g2", "altacct", "bot", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        games = [played, oracle] + filler()
        s, _ = MS.scan(games)
        broken = dict(s[0])
        broken["pts"] = [(a, b + 1) for (a, b) in broken["pts"]]     # odd offset
        self.assertIsNone(MS.seat_correspondence(broken))
        rows = MS.account_stats(games, "altacct", set())
        fd, path = tempfile.mkstemp(suffix=".md")
        os.close(fd)
        try:
            MS.write_case(path, "altacct", rows, [broken], games)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
        finally:
            os.remove(path)
        self.assertNotIn("victim_alice", text)
        self.assertNotIn("mainacct", text)
        self.assertIn("players withheld", text)

    def test_report_states_the_thresholds_the_code_actually_uses(self):
        """F13. write_case() told admins 'only pairs with 6+ rare shared positions are
        visible here' while MIN_SHARED was 3."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True, start=10.0)
        games = [played, oracle] + filler()
        s, _ = MS.scan(games)
        rows = MS.account_stats(games, "cheater", {"g1"})
        fd, path = tempfile.mkstemp(suffix=".md")
        os.close(fd)
        try:
            MS.write_case(path, "cheater", rows, s, games)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
        finally:
            os.remove(path)
        self.assertIn(str(MS.MIN_SHARED), text)
        self.assertNotIn("6+ rare shared positions", text)

    # -- scoring ---------------------------------------------------------
    def test_a_contiguous_run_is_one_coincidence_not_many(self):
        """F9. Summing log2(N/df) over a run counted one coincidence once per ply: a
        111-position mirror scored 1689 bits where the conjunction is worth ~15."""
        index = MS.build_index(filler(400))
        contiguous = [[(20 + i, 20 + i) for i in range(20)]]
        scattered = [[(20 + 7 * i, 20 + 7 * i)] for i in range(20)]
        hashes = {p: 900_000_000 + p for p in range(200)}
        for h in hashes.values():
            index["df"][h] = 2
        run_bits = MS.score_runs(index, contiguous, hashes)
        scat_bits = MS.score_runs(index, scattered, hashes)
        self.assertLess(run_bits, scat_bits,
                        "20 consecutive plies are weaker evidence than 20 unrelated hits")
        self.assertLess(run_bits, 0.5 * scat_bits, "and by a wide margin")

    def test_the_scoring_model_is_documented_as_non_monotone(self):
        """A KNOWN, UNFIXED defect, pinned so it cannot be forgotten or rediscovered.

        Every run is charged a full log2(N/df) entry term, so splitting one run in two
        adds a whole entry term while REMOVING matched positions: deleting evidence
        raises the score. See the note above RUN_GAP in mirror_scan.py for why no
        constant-sized repair works and why it was left rather than guessed at.

        This test asserts the CURRENT, WRONG behaviour on purpose. When the evidence
        model is replaced it must be inverted into the property it should have:
            self.assertLessEqual(holed, full)
        """
        index = MS.build_index(filler(400))
        hashes = {p: 900_000_000 + p for p in range(400)}
        for h in hashes.values():
            index["df"][h] = 2
        whole = [(20 + i, 20 + i) for i in range(40)]
        full = MS.score_runs(index, MS.runs_of(whole), hashes)
        holed = [x for x in whole if not (30 <= x[0] < 36)]      # punch 6 out of the middle
        split = MS.score_runs(index, MS.runs_of(holed), hashes)
        self.assertEqual(len(MS.runs_of(holed)), 2, "the gap must split the run")
        self.assertGreater(split, full,
                           "documenting the defect: deleting six matched positions "
                           "currently RAISES the score")

    def test_floor_is_scale_free_for_positions_that_are_not_near_unique(self):
        """F14. equiv = sum(log2(N/df)) / log2(N/2) is exactly 1.0 per position when
        df=2, for ANY N -- an algebraic identity. The old test used df=2 positions, so
        it pinned the one input on which the property could not fail. At df>2 the same
        evidence was worth 14% more in a larger corpus."""
        def verdict(n_filler):
            rare = [920_000_000 + i for i in range(6)]
            crowd = [mkgame(f"c{n_filler}_{i}", f"q{i}a", f"q{i}b", shared=rare,
                            start=5.0) for i in range(4)]        # df = 6, not 2
            played = mkgame("g1", "cheater", "victim", shared=rare)
            oracle = mkgame("g2", "cheater", "bot", shared=rare, bbot=True, start=10.0)
            # alpha pinned: this tests the scale-freeness of the floor, not the shipped
            # threshold (now 1e-3), so it fixes the sensitivity it was written for.
            s, _ = MS.scan([played, oracle] + crowd + filler(n_filler), 0.01)
            return len(s)
        self.assertEqual(verdict(300), verdict(6000),
                         "identical evidence must not change verdict with corpus size")

    def test_unrelated_games_do_not_change_a_pairs_verdict(self):
        """Non-manipulability. This is the property no unit test in the old suite
        expressed: a verdict about two people must not depend on games involving
        neither of them. Both `rep` and the score normalisation violated it."""
        played = mkgame("g1", "cheater", "victim", shared=SHARED)
        oracle = mkgame("g2", "cheater", "bot", shared=SHARED, bbot=True, start=10.0)
        base, _ = MS.scan([played, oracle] + filler(300))
        more, _ = MS.scan([played, oracle] + filler(4000))
        self.assertEqual(len(base), len(more))
        self.assertEqual(MS.grade(base[0])[0], MS.grade(more[0])[0])

    # -- the negative control --------------------------------------------
    def test_epoch_control_sees_pairs_the_scan_deliberately_drops(self):
        """The disjoint-epoch control measures what innocent position sharing scores.
        It has to bypass the temporal filters to do that: with them on, no surviving
        pair can be a year apart, so the control silently returns zero and reads as
        'no innocent sharing at this level' when it in fact measured nothing."""
        year = 400 * 86400.0
        a = mkgame("g1", "p1", "p2", shared=SHARED, start=0.0, end=1000.0)
        b = mkgame("g2", "p3", "bot", shared=SHARED, bbot=True,
                   start=year, end=year + 1000.0)
        games = [a, b] + filler()
        index = MS.build_index(games)
        normal, _ = MS.find_pairs(games, index)
        self.assertEqual(len(normal), 0, "a year apart is never a relay")
        ec = MS.epoch_control(games, index)
        self.assertEqual(ec["n"], 1, "but the control must still be able to score it")
        self.assertGreater(ec["max"], 0)

    def test_permutation_preserves_everything_except_concurrency(self):
        """The permutation null is only a valid baseline if the shuffle leaves the
        position evidence untouched and changes nothing but when games were played."""
        import random
        games = [mkgame("g1", "a", "b", shared=SHARED, speed="Blitz",
                        start=0.0, end=1000.0),
                 mkgame("g2", "c", "d", shared=SHARED, speed="Blitz",
                        start=50.0, end=800.0)]
        shuffled = MS.permute_times(games, random.Random(1))
        self.assertEqual([g["hashes"] for g in games], [g["hashes"] for g in shuffled])
        self.assertEqual(sorted(g["start"] for g in games),
                         sorted(g["start"] for g in shuffled))
        for a, b in zip(games, shuffled):
            self.assertEqual(a["end"] - a["start"], b["end"] - b["start"],
                             "durations must survive the shuffle")


FIXTURE = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                       "fixtures", "archive_shape.jsonl")


def _emitted_signals():
    """Every string literal mirror_scan assigns to `signal` or passes as `signal=`.

    Read out of the SOURCE with `ast`, deliberately. The migration map's job is to hold
    the sentences this project has actually shipped, so a test that gets the sentences
    from the map itself proves nothing — which is exactly how the first version of that
    test came to be unfailable.
    """
    import ast
    src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)),
                            "mirror_scan.py"), encoding="utf-8").read()
    out = set()

    def literal(node):
        """A str constant, or an implicitly-concatenated one — how these are written."""
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            return node.value
        return None

    for node in ast.walk(ast.parse(src)):
        if isinstance(node, ast.Assign):
            for t in node.targets:
                if isinstance(t, ast.Name) and t.id == "signal":
                    v = literal(node.value)
                    if v:
                        out.add(v)
        elif isinstance(node, ast.Call):
            for kw in node.keywords:
                if kw.arg == "signal":
                    v = literal(kw.value)
                    if v:
                        out.add(v)
    return sorted(out)


def uid(name):
    """A stable 32-hex uid for a test account, the shape a real one has.

    Deterministic on purpose: str.hash() is salted per process, and a test that
    asserts "the registry contains exactly these uids" has to be able to name them.
    """
    return hashlib.md5(str(name).encode()).hexdigest()


def archive_row(gid, w, b, *, shared=(), wbot=False, bbot=False, gt="MLP",
                speed="Correspondence", rated=True, tour=None, start=0.0, end=1000.0,
                wr=1800.0, br=1800.0, prefix_len=12, game_start="Moves",
                last_interaction=True):
    """A row shaped like a REAL archive row, not like load()'s output.

    Every shape here was measured against the live 80,323-row archive by
    `test_load_matches_real_archive_shape` and is pinned by it: uid is a 32-char
    lowercase hex (uuid::Uuid serialises as BYTES under CBOR because ciborium's
    is_human_readable() is false, and pull_archive.jdefault() hexes them), game_id is
    a bare nanoid string, timestamps are chrono's SecondsFormat::AutoSi with a Z
    suffix, game_status is a MAP keyed by the variant name, and hashes and history are
    the same length.

    Tests that need the loader in the loop build the corpus with this and go through
    load(); tests about detection alone use mkgame() and skip it.
    """
    from datetime import datetime, timezone
    hashes = [next(_ids) * 1_000_003 for _ in range(prefix_len)] + list(shared)

    def iso(t):
        return datetime.fromtimestamp(t, timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%S.%f") + "Z"

    row = {
        "game_id": gid, "uuid": uid("game:" + gid),
        "white_player": {"uid": uid(w), "username": str(w), "bot": wbot,
                         "patreon": False, "admin": False, "deleted": False},
        "black_player": {"uid": uid(b), "username": str(b), "bot": bbot,
                         "patreon": False, "admin": False, "deleted": False},
        "white_rating": wr, "black_rating": br,
        "white_rating_change": 0.0 if rated else None,
        "black_rating_change": 0.0 if rated else None,
        "game_type": gt, "speed": speed, "game_speed": speed, "rated": rated,
        "tournament": ({"tournament_id": tour, "id": uid("t:" + str(tour)),
                        "name": "T", "games_total": 8, "games_played": 3,
                        "players": 4, "player_list": [], "seats": 4,
                        "invite_only": False, "mode": "SingleElimination",
                        "time_mode": "Correspondence", "time_base": None,
                        "time_increment": None, "band_upper": None,
                        "band_lower": None, "status": "InProgress",
                        "start_mode": "Manual", "starts_at": None, "ends_at": None,
                        "started_at": None, "updated_at": iso(start)}
                       if tour else None),
        "game_status": {"Finished": {"Winner": "White"}},
        "conclusion": "Board", "game_start": game_start,
        "created_at": iso(start), "updated_at": iso(end + 99999.0),
        "last_interaction": iso(end) if last_interaction else None,
        "hashes": hashes, "history": [["wA1", "."]] * len(hashes),
        "move_times": [1_000_000_000] * len(hashes),
        "turn": len(hashes), "finished": True,
    }
    return row


def write_archive(rows):
    fd, path = tempfile.mkstemp(suffix=".jsonl")
    with os.fdopen(fd, "w", encoding="utf-8") as fh:
        for r in rows:
            fh.write(json.dumps(r) + "\n")
    return path


class ArchiveShapeTests(unittest.TestCase):
    """Pin the shape of the REAL archive, from a scrubbed fixture cut from it.

    Every field shape the loader assumes was inferred from the Rust source and had
    never been checked against real output. That matters more here than it usually
    would, because *every* mismatch fails silently by disabling an exemption rather
    than by raising: `tournament` not matching a key returns None and the
    same-tournament exemption never fires, which is exactly how F11 worked.

    The fixture is 23 real rows with usernames, uids, game ids and position hashes
    replaced. The hashes are scrubbed too, not just the names -- the archive endpoint
    accepts a position_hash query, so a real hash is a lookup key back to a real game
    and therefore back to real people.
    """

    @classmethod
    def setUpClass(cls):
        with open(FIXTURE, encoding="utf-8") as fh:
            cls.rows = [json.loads(line) for line in fh]

    def test_fixture_exists_and_covers_the_shapes_that_matter(self):
        self.assertGreaterEqual(len(self.rows), 20)
        speeds = {r["speed"] for r in self.rows}
        self.assertEqual(speeds, set(MS.SPEED_RANK), "every speed class must appear")
        self.assertTrue(any(r["tournament"] for r in self.rows))
        self.assertTrue(any(not r["tournament"] for r in self.rows))
        self.assertTrue(any(not r["last_interaction"] for r in self.rows))
        self.assertTrue(any(not r["hashes"] for r in self.rows))
        self.assertTrue(any(r["rated"] is False for r in self.rows))
        self.assertEqual({r["game_start"] for r in self.rows},
                         {"Moves", "Ready", "Immediate"})

    def test_load_matches_real_archive_shape(self):
        """Each assertion is a field whose real shape the loader had only guessed."""
        for i, r in enumerate(self.rows):
            with self.subTest(line=i, gid=r.get("game_id")):
                # -- join keys. uuid::Uuid is serialised as BYTES under CBOR
                # (ciborium is_human_readable() == false) and hexed by
                # pull_archive.jdefault(), so a uid is 32 lowercase hex, NOT the
                # hyphenated 36-char form.
                for side in ("white_player", "black_player"):
                    uid = r[side]["uid"]
                    self.assertIsInstance(uid, str)
                    self.assertEqual(len(uid), 32)
                    self.assertRegex(uid, r"^[0-9a-f]{32}$")
                    self.assertIsInstance(r[side]["username"], str)
                    self.assertIsInstance(r[side]["bot"], bool)
                # GameId is a newtype over String, so it arrives as a bare string,
                # not as {"0": "..."}
                self.assertIsInstance(r["game_id"], str)
                self.assertTrue(r["game_id"])

                # -- enums that must stay bare strings for the dict lookups to work
                self.assertIn(r["speed"], MS.SPEED_RANK)
                self.assertIsInstance(r["game_type"], str)
                self.assertIsInstance(r["conclusion"], str)
                self.assertIn(r["game_start"], ("Moves", "Ready", "Immediate"))
                self.assertIsInstance(r["rated"], bool)

                # -- game_status is NOT a string. It is a tuple-variant enum, so it
                # arrives as a MAP keyed by the variant name. load() str()s it, which
                # yields a Python dict repr -- harmless only because nothing reads it.
                self.assertIsInstance(r["game_status"], dict)
                self.assertEqual(set(r["game_status"]), {"Finished"})

                # -- tournament identity. THE bug that shipped: `tour` was the whole
                # TournamentAbstractResponse, whose games_played/status are captured
                # at scrape time, so two games of one tournament never compared equal.
                t = r["tournament"]
                if t is not None:
                    self.assertIsInstance(t, dict)
                    self.assertIn("tournament_id", t)
                    self.assertIsInstance(t["tournament_id"], str)
                    self.assertIsNotNone(
                        MS._tour_id(t),
                        "a non-null tournament that yields no id silently disables "
                        "the same-tournament exemption")

                # -- timestamps. chrono emits SecondsFormat::AutoSi with use_z, i.e.
                # 0/3/6/9 fractional digits and a Z suffix; Postgres is microseconds.
                self.assertIsNotNone(MS._ts(r["created_at"]))
                self.assertIsNotNone(MS._ts(r["updated_at"]))
                if r["last_interaction"]:
                    self.assertIsNotNone(MS._ts(r["last_interaction"]))

                # -- ply alignment. games.hashes is Array<Nullable<Int8>> and
                # Game::hashes() drops the NULLs, which would COMPACT the array and
                # shift every ply index. On the real archive the two lists agree on
                # every row; if they ever stop, load() must drop the game.
                self.assertIsInstance(r["hashes"], list)
                self.assertIsInstance(r["history"], list)
                if r["hashes"]:
                    self.assertEqual(len(r["hashes"]), len(r["history"]))
                    self.assertTrue(all(isinstance(h, int) for h in r["hashes"]))
                    self.assertTrue(all(h != MS.EMPTY_HASH for h in r["hashes"]),
                                    "hash 0 is the sentinel the scan skips")
                for mv in r["history"]:
                    self.assertIsInstance(mv, list)
                    self.assertEqual(len(mv), 2, "history is Vec<(String, String)>")

                # -- move_times is time LEFT on the clock, not a wall-clock stamp
                # (db/src/models/game.rs:725 pushes *_time_left). It is either
                # exactly as long as the history or empty.
                self.assertIsInstance(r["move_times"], list)
                if r["move_times"]:
                    self.assertEqual(len(r["move_times"]), len(r["history"]))

    def test_loader_accepts_the_real_rows_it_should(self):
        games, rejected = MS.load(FIXTURE)
        self.assertTrue(games, "the loader must accept real archive rows")
        # everything with enough plies and a known speed survives
        expect = sum(1 for r in self.rows
                     if len(r["hashes"]) >= MS.MIN_PLY + 4 and r["speed"] in MS.SPEED_RANK)
        self.assertEqual(len(games), expect)
        for g in games:
            self.assertEqual(len(g["w"]), 32)
            self.assertEqual(len(g["b"]), 32)
            self.assertIsNotNone(g["start"])
            self.assertIsNotNone(g["end"])
            self.assertGreaterEqual(g["end"], g["start"])
            self.assertIn(g["speed"], MS.SPEED_RANK)
        # a game with a tournament must carry a comparable id, not a snapshot
        tours = [g["tour"] for g in games if g["tour"]]
        self.assertTrue(tours)
        for t in tours:
            self.assertIsInstance(t, str)

    def test_every_real_row_has_last_interaction_or_no_hashes(self):
        """On the real archive the updated_at fallback is unreachable.

        All six rows with a null last_interaction also have zero hashes, so they are
        dropped for length before the fallback runs. Recorded because the fail-closed
        timing rule was expected to cost recall and on this corpus it costs none from
        that branch -- every untrusted game comes from the duration cap instead."""
        for r in self.rows:
            if not r["last_interaction"]:
                self.assertEqual(len(r["hashes"]), 0)


class ScopeTests(unittest.TestCase):
    """Speeds the detector does not understand must fail CLOSED."""

    def test_puzzle_games_are_out_of_scope_and_dropped_explicitly(self):
        """shared_types/src/game_speed.rs has a Puzzle variant that appears in neither
        SPEED_RANK nor MAX_PLAUSIBLE_HOURS nor pull_archive's SPEEDS. It was therefore
        never duration-quarantined and never rank-compared -- excluded by accident,
        via two dictionary defaults that both point the wrong way."""
        path = write_archive([
            archive_row("p1", "a", "b", shared=SHARED, speed="Puzzle"),
            archive_row("p2", "a", "bot", shared=SHARED, bbot=True, speed="Puzzle"),
        ])
        try:
            games, rejected = MS.load(path)
        finally:
            os.remove(path)
        self.assertEqual(games, [], "a Puzzle game must not enter the corpus")
        self.assertTrue(any("Puzzle" in k for k in rejected), dict(rejected))

    def test_an_unknown_speed_class_cannot_switch_off_the_format_test(self):
        """The rule that matters is not 'exclude Puzzle', it is 'exclude anything
        unrecognised'. Before this, SPEED_RANK.get(orac, 9) < SPEED_RANK.get(real, 0)
        was never true for an unknown value, so a new enum variant would have disabled
        the faster-format exemption rather than tripping it."""
        real = mkgame("g1", "subject", "opp", shared=SHARED,
                      speed="Correspondence", wr=1800.0, br=1800.0)
        orac = mkgame("g2", "subject", "gm", shared=SHARED, start=10.0,
                      speed="SomeFutureSpeed", wr=1800.0, br=2600.0)
        ok, why = MS.oracle_is_a_source_of_strength(real, orac)
        self.assertFalse(ok, "an unrankable speed must drop the pair, not admit it")
        self.assertIn("unknown speed", why)

    def test_hashes_history_length_disagreement_drops_the_game(self):
        """games.hashes is Array<Nullable<Int8>> and Game::hashes() filter_maps the
        NULLs away, compacting the array. Ply is the index, and ply is what MIN_PLY,
        the run alignment and the branching bucket all key on, so one dropped element
        misaligns the whole game. The real archive agrees on 80,323/80,323 rows, so
        this only ever fires on a corpus problem -- and then it fails closed."""
        row = archive_row("g1", "a", "b", shared=SHARED)
        row["hashes"] = row["hashes"][:-3]          # as if three NULLs were compacted
        path = write_archive([row])
        try:
            games, rejected = MS.load(path)
        finally:
            os.remove(path)
        self.assertEqual(games, [])
        self.assertTrue(any("length disagree" in k for k in rejected), dict(rejected))

    def test_a_tournament_game_that_starts_later_than_it_was_created_is_untrusted(self):
        """created_at is when the game ROW was made (db/src/models/game.rs:182). For
        game_start = "Ready" the row exists from tournament build time and play starts
        only once both players are ready, so created_at can precede the first move by
        days. That inflates the real game's duration, which drives start_pct toward 0
        and overlap toward 1 -- the live-window test fails OPEN, the same way it did
        on updated_at before F6. 700 such games sit under their duration cap on the
        current archive and were trusted."""
        path = write_archive([
            archive_row("ready", "a", "b", shared=SHARED, game_start="Ready"),
            archive_row("moves", "c", "d", shared=SHARED, game_start="Moves"),
            archive_row("immed", "e", "f", shared=SHARED, game_start="Immediate"),
        ])
        try:
            games, _ = MS.load(path)
        finally:
            os.remove(path)
        by = {g["gid"]: g for g in games}
        self.assertFalse(by["ready"]["timing_trusted"])
        self.assertTrue(by["moves"]["timing_trusted"])
        self.assertTrue(by["immed"]["timing_trusted"],
                        "Immediate games do start when they are created")

    def test_quarantined_games_are_not_reported_as_rejected(self):
        """A quarantined game is still in the corpus -- it still contributes to df and
        can still be somebody's opponent. It used to be counted under the word
        'rejected' next to genuine drops, so the operator was told 1,144 games had
        been discarded from a corpus that still contained all of them."""
        path = write_archive([
            archive_row("keep", "a", "b", shared=SHARED, game_start="Ready"),
            archive_row("drop", "c", "d", shared=SHARED[:1], prefix_len=2),
        ])
        try:
            games, rejected = MS.load(path)
        finally:
            os.remove(path)
        quarantined = {k: v for k, v in rejected.items() if k.startswith(MS.QUARANTINE)}
        dropped = {k: v for k, v in rejected.items() if not k.startswith(MS.QUARANTINE)}
        self.assertTrue(quarantined)
        self.assertTrue(dropped)
        self.assertEqual(len(games) + sum(dropped.values()), 2,
                         "loaded + dropped must account for every row, with "
                         "quarantined games counted among the loaded")


class LinkedBotGameTests(unittest.TestCase):
    """Two humans, each playing a bot, reproducing one line between them.

    `classify` returns (None, None) when both games contain a bot and find_pairs drops
    the pair as "roles indeterminate", because a deterministic bot gives two people
    similar games for free. Measured on the real archive, that only covers SHALLOW
    agreement: across 838 such pairs the median shared-position count is 1 and p90 is
    4, while the pairs actually being dropped shared 84, 73, 46, 40, 33 and 25.

    Neither game has a human opponent, so this is not evidence anybody beat anyone.
    It is evidence two accounts are linked, which is the fact a moderator cannot get
    from position data any other way. On the real archive it found 10 pairs across two
    account pairs, one of which the relay pipeline never saw at all, against a
    permutation null of 0.035 +/- 0.184 (0 of 200 shuffles reached 10) and an epoch
    control of zero.
    """

    def corpus(self, *extra):
        return list(extra) + filler()

    def test_two_humans_relaying_between_bot_games_are_linked(self):
        line = [770_000_000 + i for i in range(30)]
        a = mkgame("A", "acct1", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "acct2", shared=line, wbot=True,
                   start=20.0, end=900.0)
        games = self.corpus(a, b)
        idx = MS.build_index(games)
        # the relay pipeline cannot express it
        surv, dropped = MS.find_pairs(games, idx)
        self.assertEqual(len(surv), 0)
        self.assertTrue(any("indeterminate" in k for k in dropped), dict(dropped))
        # this one can
        linked, _ = MS.find_linked_bot_pairs(games, idx)
        self.assertEqual(len(linked), 1)
        self.assertEqual(set(linked[0]["humans"]), {"acct1", "acct2"})

    def test_the_same_human_in_both_bot_games_is_not_a_link(self):
        """One person practising the same line against a bot twice links nobody."""
        line = [771_000_000 + i for i in range(30)]
        a = mkgame("A", "solo", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "solo", shared=line, wbot=True,
                   start=20.0, end=900.0)
        linked, dropped = MS.find_linked_bot_pairs(
            self.corpus(a, b), MS.build_index(self.corpus(a, b)))
        self.assertEqual(len(linked), 0)

    def test_a_human_vs_human_game_is_not_this_shape(self):
        """Both games must be one-human-one-bot. A rated game against a human is the
        relay pipeline's business, not this one, and must not be double-counted."""
        line = [772_000_000 + i for i in range(30)]
        a = mkgame("A", "acct1", "victim", shared=line, start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "acct2", shared=line, wbot=True,
                   start=20.0, end=900.0)
        games = self.corpus(a, b)
        linked, _ = MS.find_linked_bot_pairs(games, MS.build_index(games))
        self.assertEqual(len(linked), 0)

    def test_bot_determinism_alone_does_not_link_anyone(self):
        """The exemption this reopens exists for a real reason: shallow agreement
        between two people playing the same bot must stay exempt, and on the real
        archive that is what the shape overwhelmingly looks like — across 838 such
        pairs the median shared-position count is 1 and p90 is 4.

        Asserted against the floor the LIVE corpus produces (10,519 candidate pairs
        → 23.30 bits at the shipped alpha=1e-3), because the floor is
        multiplicity-corrected: on a 300-game synthetic corpus there is one candidate
        pair, so the floor is ~10 bits and three positions legitimately clear it. Testing
        this end-to-end on filler would pin the wrong number."""
        LIVE_CANDIDATE_PAIRS = 10_519
        floor = MS.floor_bits(LIVE_CANDIDATE_PAIRS)      # default alpha = 1e-3
        self.assertAlmostEqual(floor, 23.3, places=1)

        index = MS.build_index(filler(400))
        hashes = {p: 773_000_000 + p for p in range(400)}
        for h in hashes.values():
            index["df"][h] = 2
        index["ngames"] = 75_934                      # the live corpus size
        typical = MS.score_runs(index, MS.runs_of([(20, 20), (21, 21), (22, 22)]),
                                hashes)
        self.assertLess(typical, floor,
                        "p90 bot-determinism agreement must not clear the live floor")
        deep = MS.score_runs(
            index, MS.runs_of([(20 + i, 20 + i) for i in range(25)]), hashes)
        self.assertGreater(deep, floor,
                           "but the 25-84 position pairs actually observed must")

    def test_games_that_never_overlapped_are_not_a_link(self):
        line = [774_000_000 + i for i in range(30)]
        a = mkgame("A", "acct1", "botacct", shared=line, bbot=True,
                   start=0.0, end=100.0)
        b = mkgame("B", "botacct", "acct2", shared=line, wbot=True,
                   start=9000.0, end=9900.0)
        games = self.corpus(a, b)
        linked, _ = MS.find_linked_bot_pairs(games, MS.build_index(games))
        self.assertEqual(len(linked), 0)

    def test_no_bot_account_is_ever_named_as_linked(self):
        line = [775_000_000 + i for i in range(30)]
        a = mkgame("A", "acct1", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "acct2", shared=line, wbot=True,
                   start=20.0, end=900.0)
        games = self.corpus(a, b)
        linked, _ = MS.find_linked_bot_pairs(games, MS.build_index(games))
        self.assertNotIn("botacct", linked[0]["humans"])


class BookFollowTests(unittest.TestCase):
    """book_follow.py had no tests, and the first thing an audit found was that every
    judged move was credited to the opponent."""

    def test_the_judged_move_is_credited_to_the_player_who_made_it(self):
        """THE bug. `hashes[p]` is the position the move `history[p+1]` was played
        from, and White moves at EVEN history indices — verified on the archive: over
        4,000 games, index%2==0 carries a `w` piece 23,888 times against 12. So the
        mover is White exactly when p is ODD.

        It was written `w if p % 2 == 0 else b`, the opposite. Nothing crashed and the
        output looked entirely normal: a complete, well-formatted table about the wrong
        people. Every published number had to be withdrawn."""
        import book_follow as BF
        self.assertEqual(BF.mover_of(0), 1, "history[1] is Black's move")
        self.assertEqual(BF.mover_of(1), 0, "history[2] is White's move")
        for p in range(0, 40):
            # history[p+1] is White iff (p+1) is even
            self.assertEqual(BF.mover_of(p) == 0, (p + 1) % 2 == 0)

    def test_a_line_only_you_have_played_is_not_a_book_you_followed(self):
        """Leave-one-PLAYER-out, not leave-one-game-out. Without it the heaviest
        repertoire players top the chart by 'following' the book their own earlier
        games created — the same trap `_repertoire` exists for in mirror_scan."""
        import book_follow as BF
        counts = collections.Counter({("wA1", "x"): 50})
        slot = (counts, {"solo": collections.Counter({("wA1", "x"): 50})})
        self.assertIsNone(BF.book_move(slot, "solo"),
                          "your own 50 games are not a book about you")
        # three other players make it a book again
        for i in range(3):
            slot[1][f"other{i}"] = collections.Counter({("wA1", "x"): 1})
        slot[0][("wA1", "x")] += 3
        self.assertIsNotNone(BF.book_move(slot, "solo"))


class SeatConflictTests(unittest.TestCase):
    """Two oracles, opposite bot colours, one rated game — the seat resolves BOTH ways."""

    def setUp(self):
        # The seat inference is OPT-IN and off by default (D1). These tests exercise the
        # feature, so they turn it on and restore the default afterwards.
        MS.SEAT_ENABLED = True
        self.addCleanup(setattr, MS, "SEAT_ENABLED", False)

    def test_contradictory_seat_resolutions_withhold_both_names(self):
        """Found by audit. seat_correspondence answers from the bot's colour in ONE
        pair's oracle. One person opening two colour-swapped analysis boards against a
        bot, both mirroring the same rated game, produced two survivors whose resolved
        seats are OPPOSITE — and both real-game players got a review file naming the
        other as excluded. Third instance of the named-victim failure.

        At most one answer can be right and the position data does not say which, so
        both are withdrawn and the report goes back to withholding both names."""
        line = [700_000_000 + i for i in range(24)]
        real = mkgame("R", "alice", "bob", shared=line, start=0.0, end=1000.0)
        oa = mkgame("OA", "spec", "botacct", shared=line, bbot=True,
                    start=10.0, end=900.0)
        ob = mkgame("OB", "botacct", "spec", shared=line, wbot=True,
                    start=20.0, end=880.0)
        games = [real, oa, ob] + filler()
        surv, _ = MS.scan(games)
        pairs = [p for p in surv if "R" in (p["a"]["gid"], p["b"]["gid"])]
        self.assertGreaterEqual(len(pairs), 2, "both oracles must survive to conflict")
        seats = {MS._seat_raw(p)[0] for p in pairs if MS._seat_raw(p)}
        self.assertEqual(seats, {"alice", "bob"},
                         "the raw rule really does answer both ways here")
        for p in pairs:
            self.assertIsNone(MS.seat_correspondence(p),
                              "a contradicted seat must resolve to nothing")
        rows = MS.account_stats(games, "spec", set())
        fd, path = tempfile.mkstemp(suffix=".md")
        os.close(fd)
        try:
            MS.write_case(path, "spec", rows, pairs, games)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
        finally:
            os.remove(path)
        self.assertNotIn("alice", text)
        self.assertNotIn("bob", text)
        self.assertIn("players withheld", text)

    def test_a_single_oracle_still_resolves_normally(self):
        """The guard must not disarm the feature wherever there is no contradiction."""
        real = mkgame("R", "alice", "bob", shared=SHARED, start=0.0, end=1000.0)
        oa = mkgame("OA", "spec", "botacct", shared=SHARED, bbot=True,
                    start=10.0, end=900.0)
        surv, _ = MS.scan([real, oa] + filler())
        self.assertEqual(len(surv), 1)
        self.assertIsNotNone(MS.seat_correspondence(surv[0]))

    def test_the_report_never_states_a_validation_number_it_did_not_measure(self):
        """write_case hardcoded 'all 854 matched positions' and '6 times out of 6' —
        measured once on a different corpus and frozen into every file since. It is the
        paragraph a moderator uses to judge how far the inference is validated."""
        real = mkgame("R", "alice", "bob", shared=SHARED, start=0.0, end=1000.0)
        oa = mkgame("OA", "spec", "botacct", shared=SHARED, bbot=True,
                    start=10.0, end=900.0)
        games = [real, oa] + filler()
        surv, _ = MS.scan(games)
        rows = MS.account_stats(games, "spec", set())
        fd, path = tempfile.mkstemp(suffix=".md")
        os.close(fd)
        try:
            MS.write_case(path, "spec", rows, surv, games)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
        finally:
            os.remove(path)
        self.assertNotIn("854", text)
        self.assertNotIn("6 times out of 6", text)
        self.assertIn("on this run", text)


class SelfMirrorTests(unittest.TestCase):
    """One account, two concurrent bot games, colours swapped: the bot plays itself.

    Found by investigating a single account on request. Two games created 53 seconds
    apart, one account against the same bot with the colours swapped, both 83 plies,
    65 shared positions. Rating change about +290 and +640 (rounded) — roughly 900 points from two DRAWN
    games, because Glicko-2 pays enormously for holding a 2550 engine at a fresh RD.
    (No account names in tracked files; the scan output carries them.)

    NOT a cheating finding, by explicit policy: there is no opponent, so what it
    distorts is a rating and what it needs is a rating correction. It is reported
    separately and never enters the review queue or the registry.

    On the real archive: 17 pairs across 7 accounts, against a permutation null of
    0.105 +/- 0.338 (0 of 200 shuffles reached 17) and an epoch control of zero.
    """

    def corpus(self, *extra):
        return list(extra) + filler()

    def test_colour_swapped_concurrent_bot_games_are_detected(self):
        line = [660_000_000 + i for i in range(30)]
        a = mkgame("A", "solo", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "solo", shared=line, wbot=True,
                   start=20.0, end=900.0)
        games = self.corpus(a, b)
        idx = MS.build_index(games)
        self.assertEqual(len(MS.find_pairs(games, idx)[0]), 0,
                         "the relay pipeline cannot express it")
        self.assertEqual(len(MS.find_linked_bot_pairs(games, idx)[0]), 0,
                         "nor can the linked-account one: it is the same human")
        sm, _ = MS.find_self_mirror_pairs(games, idx)
        self.assertEqual(len(sm), 1)
        self.assertEqual(sm[0]["uid"], "solo")

    def test_same_colour_in_both_is_ordinary_practice(self):
        """The colour swap is the whole signature. Playing the same opening against a
        bot twice from the same side is repetition of a line, and 20 such pairs are in
        the corpus against 17 swapped ones."""
        line = [661_000_000 + i for i in range(30)]
        a = mkgame("A", "solo", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "solo", "botacct", shared=line, bbot=True,
                   start=20.0, end=900.0)
        games = self.corpus(a, b)
        sm, dropped = MS.find_self_mirror_pairs(games, MS.build_index(games))
        self.assertEqual(len(sm), 0)
        self.assertTrue(any("same colour" in k for k in dropped), dict(dropped))

    def test_a_draw_by_repetition_is_not_what_is_being_detected(self):
        """A repetition draw is a legitimate way to hold a stronger opponent and must
        never be the trigger. Nothing in the detector reads `conclusion`, and on the
        real archive at most ONE of each pair's 13-88 shared positions falls on a
        repeated ply."""
        line = [662_000_000 + i for i in range(30)]
        a = mkgame("A", "solo", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "solo", shared=line, wbot=True,
                   start=20.0, end=900.0)
        a["conclusion"] = b["conclusion"] = "Repetition"
        games = self.corpus(a, b)
        with_rep = MS.find_self_mirror_pairs(games, MS.build_index(games))[0]
        a["conclusion"] = b["conclusion"] = "Board"
        games2 = self.corpus(a, b)
        without = MS.find_self_mirror_pairs(games2, MS.build_index(games2))[0]
        self.assertEqual(len(with_rep), len(without),
                         "the conclusion must not change the verdict either way")

    def test_it_never_reaches_the_review_queue_or_the_registry(self):
        """Policy, not an accident: this is a rating correction, and putting it in a
        queue headed by banned accounts would mislabel it."""
        line = [663_000_000 + i for i in range(30)]
        a = mkgame("A", "solo", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "solo", shared=line, wbot=True,
                   start=20.0, end=900.0)
        games = self.corpus(a, b)
        idx = MS.build_index(games)
        self.assertEqual(len(MS.find_self_mirror_pairs(games, idx)[0]), 1)
        # nothing the queue is built from sees it
        self.assertEqual(MS.find_pairs(games, idx)[0], [])
        self.assertEqual(MS.find_linked_bot_pairs(games, idx)[0], [])

    def test_the_rating_change_is_reported_but_never_filters(self):
        """A detector that gated on profit would miss the accounts that tried it and
        lost -- 4 of the 17 real pairs belong to an account that came out 121 points
        DOWN -- and would be reasoning from outcome instead of from mechanism."""
        line = [664_000_000 + i for i in range(30)]
        a = mkgame("A", "solo", "botacct", shared=line, bbot=True,
                   start=0.0, end=1000.0)
        b = mkgame("B", "botacct", "solo", shared=line, wbot=True,
                   start=20.0, end=900.0)
        a["wrc"], b["brc"] = -50.0, -60.0          # a losing self-mirror
        games = self.corpus(a, b)
        sm, _ = MS.find_self_mirror_pairs(games, MS.build_index(games))
        self.assertEqual(len(sm), 1, "detected on mechanism, not on profit")
        self.assertLess(sm[0]["gain"], 0)


class SeatCorrespondenceTests(unittest.TestCase):
    """Which SEAT of the real game the oracle's bot corresponds to.

    The hash XORs a side-to-move term in (engine/src/hasher.rs:27), so two games
    sharing a position are at equal parity and the colour mapping between them is the
    identity. In a relay the person sits in the oracle game playing their OPPONENT's
    colour, because the bot has to produce moves for their own -- so the bot's colour
    is the colour that received the moves.
    """

    def setUp(self):
        # Seat inference is opt-in (D1); this whole class tests it, so enable it here.
        MS.SEAT_ENABLED = True
        self.addCleanup(setattr, MS, "SEAT_ENABLED", False)

    def scan(self, *games):
        return MS.scan(list(games) + filler())

    def test_the_bot_seat_maps_to_the_subject_seat(self):
        """The control that validates the whole idea: on a self-attributable pair the
        answer is already known independently, so the seat rule must reproduce it.
        On the real archive this agrees on 6 of 6 with 0 disagreements."""
        # subject plays Black in the real game, so they sit as White against the bot
        played = mkgame("g1", "victim", "cheater", shared=SHARED,
                        start=0.0, end=1000.0)
        oracle = mkgame("g2", "cheater", "botacct", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1)
        seat = MS.seat_correspondence(s[0])
        self.assertIsNotNone(seat)
        recv_uid, recv_name, other_uid, other_name = seat
        self.assertEqual(recv_uid, "cheater",
                         "the bot's colour must map to the subject's own seat")
        self.assertEqual(other_uid, "victim")

    def test_it_resolves_a_seat_in_the_unattributable_shape(self):
        played = mkgame("g1", "alice", "bob", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "altacct", "botacct", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1)
        recv_uid, _, other_uid, _ = MS.seat_correspondence(s[0])
        # bot is Black in the oracle, so the real game's Black seat received
        self.assertEqual(recv_uid, "bob")
        self.assertEqual(other_uid, "alice")

    def test_an_odd_ply_offset_fails_closed(self):
        """An odd offset would mean the side-to-move term did not survive into the
        match, and every conclusion here would be unsound. Guess nothing."""
        played = mkgame("g1", "alice", "bob", shared=SHARED)
        oracle = mkgame("g2", "altacct", "bot", shared=SHARED, bbot=True, start=10.0)
        s, _ = self.scan(played, oracle)
        p = dict(s[0])
        p["pts"] = [(a, b + 1) for (a, b) in p["pts"]]      # force an odd offset
        self.assertIsNone(MS.seat_correspondence(p))

    def test_a_human_oracle_has_no_bot_seat_to_map(self):
        played = mkgame("g1", "subject", "opp", shared=SHARED,
                        speed="Correspondence", wr=1800.0, br=1800.0)
        oracle = mkgame("g2", "subject", "gm", shared=SHARED, start=10.0,
                        speed="Correspondence", wr=1800.0, br=2600.0)
        s, _ = self.scan(played, oracle)
        self.assertEqual(len(s), 1)
        self.assertIsNone(MS.seat_correspondence(s[0]))

    def test_the_report_shows_the_derivation_not_just_a_name(self):
        """A bare name in an accusation file is worse than none. The section has to
        carry how the seat was derived, how far it has been validated, and that
        linking the two accounts still needs the database."""
        played = mkgame("g1", "alice", "bob", shared=SHARED, start=0.0, end=1000.0)
        oracle = mkgame("g2", "altacct", "botacct", shared=SHARED, bbot=True,
                        start=10.0, end=900.0)
        games = [played, oracle] + filler()
        s, _ = MS.scan(games)
        rows = MS.account_stats(games, "altacct", set())
        fd, path = tempfile.mkstemp(suffix=".md")
        os.close(fd)
        try:
            MS.write_case(path, "altacct", rows, s, games)
            with open(path, encoding="utf-8") as fh:
                text = fh.read()
        finally:
            os.remove(path)
        self.assertIn("side-to-move", text, "the derivation must be stated")
        # How far it has been validated must be MEASURED ON THIS RUN. This assertion
        # used to pin the literal string "6 times out of 6", which was a number taken
        # once on a different corpus and frozen into every file since — so the test
        # was actively holding the fabrication in place.
        self.assertIn("on this run", text)
        self.assertRegex(text, r"\d+ of \d+ matched positions in this file")
        # This used to assert the phrase "requires the database", alongside a sentence
        # telling the reviewer to check "shared address, session or payment data". None of
        # those columns exist in db/src/schema.rs — there is no IP address, no session
        # table and no login history anywhere in it. The advice had been written without
        # reading the schema, and the test was pinning it in place. It now pins the two
        # things that are actually true: what the schema HAS, and that it has no IP.
        # There IS exactly one `ip` column — `email_request_log.ip` — so the first attempt
        # at this correction ("no IP address anywhere") was also wrong. It is a
        # password-reset rate-limit log that email_cleanup.rs purges after 24 hours, which
        # is why it cannot serve a finding surfacing weeks later. The report has to say
        # that rather than either overclaim.
        self.assertIn("email_request_log.ip", text,
                      "name the one ip column rather than pretending there is none")
        self.assertIn("24 hours", text, "and say why it is unusable")
        self.assertIn("push_devices.device_token", text,
                      "it must name the field that CAN establish linkage")
        self.assertNotRegex(text, r"shared address|payment data",
                            "neither exists in this schema")
        self.assertIn("`p3`", text, "and that the grade did not move")


class RegistryWiringTests(unittest.TestCase):
    """End-to-end: a full run over a corpus containing one of every shape must leave
    exactly the intended accounts at `suspicious`, and nothing anywhere above it.

    Registry.add_evidence had no caller outside registry.py and its own tests, so the
    whole suspicious/proven_cheater pipeline was disconnected from the detector. This
    goes through main() rather than calling record_evidence directly, because the part
    that has twice been wrong is the attribution block in main(), not the writing.
    """

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.out = os.path.join(self.dir, "cases")
        self.reg = os.path.join(self.dir, "registry.json")

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def _run(self, rows):
        path = write_archive(rows)
        argv = sys.argv
        sys.argv = ["mirror_scan.py", "--archive", path, "--out", self.out,
                    "--registry", self.reg]
        try:
            with contextlib.redirect_stdout(io.StringIO()) as buf:
                MS.main()
        finally:
            sys.argv = argv
            os.remove(path)
        with open(self.reg, encoding="utf-8") as fh:
            return json.load(fh), buf.getvalue()

    @staticmethod
    def _corpus():
        """A mirror, a cluster, an alt-account pair, and victims for each."""
        rows = [archive_row(f"filler{i}", f"u{i}a", f"u{i}b",
                            shared=[next(_ids) * 7_000_003 for _ in range(20)])
                for i in range(300)]
        # 1. the canonical self-attributable mirror
        rows += [
            archive_row("mirror_real", "cheater", "victim", shared=SHARED,
                        start=0.0, end=1000.0),
            archive_row("mirror_orac", "cheater", "botacct", shared=SHARED,
                        bbot=True, start=10.0, end=900.0),
        ]
        # 2. one popular line shared by seven games plus a bot game: a cluster, which
        #    must implicate nobody at all
        line = [990_000_000 + i for i in range(12)]
        rows += [archive_row(f"clus{i}", f"clusA{i}", f"clusB{i}", shared=line,
                             start=0.0, end=1000.0) for i in range(7)]
        rows += [archive_row("clus_hub", "clushub", "botacct2", shared=line,
                             bbot=True, start=10.0, end=900.0)]
        # 3. the alt-account shape: nobody sits in both games. Seat correspondence
        #    resolves WHICH seat received the moves -- the bot is Black in the oracle,
        #    so the real game's BLACK seat is the beneficiary and White is cleared.
        #    The names below say which is which, so an assertion about them means
        #    something.
        alt = [880_000_000 + i for i in range(20)]
        rows += [
            archive_row("alt_real", "alt_victim", "alt_beneficiary", shared=alt,
                        start=0.0, end=1000.0),
            archive_row("alt_orac", "alt_buffer", "botacct3", shared=alt, bbot=True,
                        start=10.0, end=900.0),
        ]
        return rows

    def test_a_full_run_leaves_exactly_the_intended_accounts_suspicious(self):
        # seat inference is opt-in (D1); this test exercises it
        MS.SEAT_ENABLED = True
        self.addCleanup(setattr, MS, "SEAT_ENABLED", False)
        data, _ = self._run(self._corpus())
        accounts = data["accounts"]
        got = {u: a["label"] for u, a in accounts.items()}

        self.assertEqual(
            set(got),
            {uid("cheater"),          # in both games of the mirror
             uid("alt_buffer"),       # ran the oracle of the alt-shape pair
             uid("alt_beneficiary")}, # seat correspondence: received the moves
            f"unexpected accounts in the registry: "
            f"{ {accounts[u]['name']: l for u, l in got.items()} }")
        for u in got:
            self.assertEqual(got[u], "suspicious")

    def test_the_seat_derived_account_is_labelled_as_an_inference(self):
        """It is reached by inference from the bot's colour, not by being observed in
        both games. A moderator running `registry.py show` has to see that."""
        # seat inference is opt-in (D1); this test exercises it
        MS.SEAT_ENABLED = True
        self.addCleanup(setattr, MS, "SEAT_ENABLED", False)
        data, _ = self._run(self._corpus())
        sig = data["accounts"][uid("alt_beneficiary")]["signals"][0]["signal"]
        self.assertIn("seat correspondence", sig)
        self.assertIn("inference", sig)
        direct = data["accounts"][uid("cheater")]["signals"][0]["signal"]
        self.assertNotIn("inference", direct,
                         "a pair with the same account in both games is not inferred")

    def test_no_victim_and_no_clustered_player_reaches_the_registry(self):
        data, _ = self._run(self._corpus())
        accounts = data["accounts"]
        for who in ("victim",           # opponent in the self-attributable mirror
                    "alt_victim",       # the seat correspondence CLEARS
                    "clushub",
                    *[f"clusA{i}" for i in range(7)],
                    *[f"clusB{i}" for i in range(7)]):
            self.assertNotIn(uid(who), accounts,
                             f"{who} must never appear: a victim, the seat the "
                             f"analysis clears, and a clustered player are the three "
                             f"shapes this system exists to protect")

    def test_no_bot_account_ever_gets_evidence(self):
        data, _ = self._run(self._corpus())
        for bot in ("botacct", "botacct2", "botacct3"):
            self.assertNotIn(uid(bot), data["accounts"])

    def test_automation_never_writes_a_label_above_suspicious(self):
        data, _ = self._run(self._corpus())
        for u, a in data["accounts"].items():
            self.assertIn(a["label"], ("unreviewed", "suspicious"))
        for e in data["log"]:
            self.assertEqual(e["by"], "detector")
            self.assertNotIn(e["to"], REG.HUMAN_ONLY)

    def test_a_rerun_does_not_re_accumulate_the_same_evidence(self):
        """The de-dup key is (signal, detail), so nothing that moves between runs may
        go in either. Bit totals and position counts shift as the corpus grows; the
        stable identity of a finding is the unordered pair of game ids."""
        rows = self._corpus()
        self._run(rows)
        with open(self.reg, encoding="utf-8") as fh:
            first = json.load(fh)
        # rerun on a LARGER corpus: bits, need_bits and the grade all move
        data, _ = self._run(rows + [
            archive_row(f"extra{i}", f"x{i}a", f"x{i}b",
                        shared=[next(_ids) * 3_000_017 for _ in range(20)])
            for i in range(400)])
        for u, a in data["accounts"].items():
            self.assertEqual(len(a["signals"]), len(first["accounts"][u]["signals"]),
                             "a rescan must not append a duplicate signal")
            self.assertEqual(a["evidence_games"], first["accounts"][u]["evidence_games"])
        self.assertEqual(len(data["log"]), len(first["log"]),
                         "and must not append a duplicate transition")

    def test_evidence_arriving_after_a_human_clearance_resurfaces(self):
        rows = self._corpus()
        self._run(rows)
        r = Registry(self.reg)
        r.set_label(uid("cheater"), "normal", by="a_moderator",
                    why="looked at the database and cleared this account")
        r.save()

        # rescanning the SAME corpus is not new evidence and must not resurface
        data, _ = self._run(rows)
        self.assertFalse(data["accounts"][uid("cheater")]["needs_recheck"])

        # a second, different mirror by the same account is
        again = [990_111_000 + i for i in range(20)]
        data, _ = self._run(rows + [
            archive_row("mirror2_real", "cheater", "victim2", shared=again,
                        start=5_000_000.0, end=5_001_000.0),
            archive_row("mirror2_orac", "cheater", "botacct", shared=again, bbot=True,
                        start=5_000_010.0, end=5_000_900.0)])
        a = data["accounts"][uid("cheater")]
        self.assertEqual(a["label"], "normal", "automation never overwrites a human")
        self.assertTrue(a["needs_recheck"], "but a new finding must resurface")
        self.assertNotIn(uid("victim2"), data["accounts"])

    def test_a_case_file_is_named_by_the_whole_uid(self):
        """case_{uid[:12]}.md let two accounts sharing a 12-character prefix overwrite
        each other's review file, and the survivor carried the other account's name in
        its heading."""
        self._run(self._corpus())
        files = os.listdir(self.out)
        self.assertTrue(files)
        for f in files:
            self.assertRegex(f, r"^case_[0-9a-f]{32}\.md$")


class EvasionTests(unittest.TestCase):
    """What a cheater who has READ THIS FILE can do. Every threshold here is a
    published constant in a public repository.

    None of this measures recall. Recall is not measurable: there is no confirmed
    case, so there is no denominator, and a suite that constructed cheaters until it
    caught them would only be writing the cheater to match the detector. What these
    pin is the EVASION SURFACE -- where the cliffs are, and what going over one costs
    the person doing it. When somebody changes a threshold, these say what it did to
    the set of people who can walk past.

    The numbers here are on a synthetic background so they run anywhere; the same
    harness against the real 75,934-game archive is in the notes on each test.
    """

    def relay(self, run_len=30, *, oracle_start=0.02, oracle_end=0.98,
              prior=0, prior_prefix=None, speed="Correspondence", n_filler=300):
        """A faithful relay with one deviation, against a filler background."""
        shared = [700_000_000 + next(_ids) for _ in range(run_len)]
        dur = 1000.0
        real = mkgame("R", "cheat", "victim", shared=shared, speed=speed,
                      start=0.0, end=dur)
        orac = mkgame("O", "cheat", "botacct", shared=shared, bbot=True, speed=speed,
                      start=dur * oracle_start, end=dur * oracle_end)
        pres = [mkgame(f"P{i}", "cheat", f"other{i}",
                       shared=shared[:(prior_prefix if prior_prefix is not None
                                       else run_len)],
                       start=-90000.0 - i, end=-80000.0 - i) for i in range(prior)]
        games = [real, orac] + pres + filler(n_filler)
        surv, _ = MS.scan(games)
        for p in surv:
            if {p["a"]["gid"], p["b"]["gid"]} == {"R", "O"}:
                return MS.grade(p)[0]
        return None

    def test_even_a_very_short_relay_is_caught(self):
        """Sensitivity is the half that works. Against the real archive a relay of
        only 6 plies still scores 20.2 bits against a 20.0-bit floor."""
        self.assertIsNotNone(self.relay(run_len=30))
        self.assertIsNotNone(self.relay(run_len=12))
        self.assertIsNotNone(self.relay(run_len=8))

    def test_relaying_in_a_timed_format_is_caught_too(self):
        """The current real-archive survivors are all Correspondence, which is a fact
        about the corpus and not a limitation of the detector: an injected Blitz and
        an injected Rapid relay are both caught at p1."""
        for speed in ("Blitz", "Rapid", "Classic"):
            with self.subTest(speed=speed):
                self.assertIsNotNone(self.relay(run_len=30, speed=speed))

    # -- the cliffs ------------------------------------------------------
    def test_leaving_the_oracle_game_open_no_longer_evades(self):
        """CLOSED. This was the cheapest evasion found: MIN_OVERLAP normalised the
        overlap by the ORACLE's own duration, so lengthening the oracle shrank the
        ratio and nothing else changed. Against the real archive, on a median 72.5h
        correspondence game, playing the bot game 18.1h longer made the pair invisible
        -- and `end` is `last_interaction`, the last MOVE, so the cost was a few extra
        moves against a bot.

        MIN_COVERAGE normalises by the REAL game's span instead, which is immune by
        construction: overlap is capped by the real game, so extending the oracle
        cannot reduce it."""
        self.assertIsNotNone(self.relay(oracle_end=0.98), "baseline must be caught")
        for factor in (1.25, 2.0, 5.0, 50.0):
            with self.subTest(oracle_runs_x_longer=factor):
                self.assertIsNotNone(
                    self.relay(oracle_end=factor),
                    "extending the oracle must not reduce coverage of the real game")

    def test_opening_the_oracle_late_is_a_published_cliff(self):
        """MAX_START_PCT = 40. Against the real archive: 39% in is caught, 41% is
        invisible. The cost is real -- no advice for the first 41% of the game -- so
        this one is a genuine trade rather than a free pass."""
        self.assertIsNotNone(self.relay(oracle_start=0.35))
        self.assertIsNone(self.relay(oracle_start=0.45))

    def test_the_repertoire_exemption_is_not_the_free_pass_it_looks_like(self):
        """REP_MIN_GAMES = 3 looks like a two-minute permanent exemption: play the
        line in three throwaway games and it can never be evidence again.

        It is not, because a relay cannot know the line in advance -- the bot chooses
        the replies. Only a prefix the player can deterministically steer into can be
        pre-played, and the exemption is per-position, so it degrades gracefully.
        Measured against the real archive on a 60-ply relay: pre-playing the first 50
        of 60 positions is still caught (24 bits); it takes ~55 of 60 to go dark,
        i.e. knowing almost the whole game before it starts."""
        self.assertIsNotNone(self.relay(run_len=30, prior=3, prior_prefix=10),
                             "pre-playing a third of the line does not exempt it")
        self.assertIsNotNone(self.relay(run_len=30, prior=3, prior_prefix=20),
                             "nor two thirds")
        self.assertIsNone(self.relay(run_len=30, prior=3, prior_prefix=30),
                          "only pre-playing the WHOLE line exempts it, which a relay "
                          "cannot arrange in advance")
        self.assertIsNotNone(self.relay(run_len=30, prior=2, prior_prefix=30),
                             "and it takes three prior games, not two")

    def test_an_alt_account_oracle_is_detected_but_never_named(self):
        """Not an evasion of DETECTION -- it is an evasion of ATTRIBUTION, which for a
        moderator is the same thing. Most real survivors are this shape."""
        real = mkgame("R", "main", "victim", shared=SHARED, start=0.0, end=1000.0)
        orac = mkgame("O", "alt", "botacct", shared=SHARED, bbot=True,
                      start=20.0, end=900.0)
        s, _ = MS.scan([real, orac] + filler())
        self.assertEqual(len(s), 1, "the pair is detected")
        self.assertEqual(MS.grade(s[0])[0], "p3", "but never above p3")
        self.assertEqual(MS.subject_of(*MS.classify(s[0])), set(),
                         "and no account is attributable, so no name reaches anyone")


class RegistryTests(unittest.TestCase):
    def setUp(self):
        self.path = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                 "_test_registry.json")
        if os.path.exists(self.path):
            os.remove(self.path)
        self.r = Registry(self.path)
        self._now = REG._now

    def tearDown(self):
        REG._now = self._now
        if os.path.exists(self.path):
            os.remove(self.path)

    def test_detector_can_only_reach_suspicious(self):
        a = self.r.add_evidence("uid1", signal="self-mirror", detail="x", games=["g1"])
        self.assertEqual(a["label"], "suspicious")

    def test_detector_cannot_mark_anyone_proven(self):
        with self.assertRaises(SystemExit):
            self.r.set_label("uid1", "proven_cheater", by="detector", why="x" * 40)

    def test_proven_requires_a_written_reason(self):
        with self.assertRaises(SystemExit):
            self.r.set_label("uid1", "proven_cheater", by="a_moderator", why="cheater")

    def test_clearing_someone_also_requires_a_human(self):
        with self.assertRaises(SystemExit):
            self.r.set_label("uid1", "normal", by="detector", why="looked fine to me")

    def test_a_human_can_promote_with_a_reason(self):
        self.r.add_evidence("uid1", signal="self-mirror", detail="x")
        a = self.r.set_label("uid1", "proven_cheater", by="a_moderator",
                             why="111-position full-game mirror, confirmed in the database")
        self.assertEqual(a["label"], "proven_cheater")

    def test_history_is_append_only(self):
        self.r.add_evidence("uid1", signal="s", detail="d")
        self.r.set_label("uid1", "proven_cheater", by="a_moderator", why="y" * 40)
        self.r.set_label("uid1", "normal", by="a_moderator", why="z" * 40)
        self.assertEqual(len(self.r.data["log"]), 3)
        self.assertEqual(self.r.data["log"][-1]["from"], "proven_cheater")

    def test_evidence_is_not_duplicated_across_weekly_rescans(self):
        """F15. The de-duplication compared whole dicts carrying `"at": _now()`, so it
        only ever worked for two calls inside the same second -- which is exactly what
        the old test did. Identical evidence re-accumulated once per scan run, and
        `list` ranked accounts by len(signals)."""
        for stamp in ("2026-08-01T10:00:00+00:00", "2026-08-08T10:00:00+00:00",
                      "2026-08-15T10:00:00+00:00"):
            REG._now = lambda s=stamp: s
            self.r.add_evidence("uid1", signal="self-mirror", detail="same", games=["g1"])
        a = self.r.get("uid1")
        self.assertEqual(len(a["signals"]), 1, "one finding is one signal")
        self.assertEqual(a["signals"][0]["count"], 3)
        self.assertEqual(a["signals"][0]["first_at"], "2026-08-01T10:00:00+00:00")
        self.assertEqual(a["signals"][0]["last_at"], "2026-08-15T10:00:00+00:00")
        self.assertEqual(a["evidence_games"], ["g1"])

    def test_evidence_after_a_human_clearance_is_recorded_and_resurfaced(self):
        """F12. add_evidence only logged and only promoted when the label was exactly
        `unreviewed`. Once a volunteer set `normal`, every later detection -- including
        a whole-game mirror -- was swallowed: no log entry, invisible to
        `list --label suspicious`, and still exported as a clean negative example."""
        self.r.add_evidence("uid2", signal="self-mirror", detail="first")
        self.r.set_label("uid2", "normal", by="a_moderator",
                         why="checked the database, this account is fine")
        before = len(self.r.data["log"])
        a = self.r.add_evidence("uid2", signal="self-mirror", detail="111-position mirror")
        self.assertEqual(a["label"], "normal", "automation never overwrites a human")
        self.assertTrue(a["needs_recheck"], "but it must resurface for re-review")
        self.assertEqual(len(self.r.data["log"]), before + 1,
                         "and the arrival must be on the immutable log")

    def test_a_rescan_does_not_mark_every_suspicious_account_for_recheck(self):
        """Found by wiring add_evidence up to the detector for the first time.

        `suspicious` is automation's OWN label, but it fell into the same branch as a
        human label, so the second scan of an unchanged finding set needs_recheck and
        appended a log entry reading "new evidence arrived after human review" -- with
        no human having reviewed it and no evidence being new. Two runs put the whole
        suspicious list into `list --recheck`, the one surface that is supposed to
        mean a volunteer must look at something again."""
        for _ in range(3):
            self.r.add_evidence("uid9", signal="self-mirror", detail="gA + gB",
                                games=["gA", "gB"])
        a = self.r.get("uid9")
        self.assertEqual(a["label"], "suspicious")
        self.assertFalse(a["needs_recheck"],
                         "nobody has reviewed this account, so there is nothing to "
                         "re-check")
        self.assertEqual(len(self.r.data["log"]), 1,
                         "one finding, found three times, is one transition")
        self.assertEqual(a["signals"][0]["count"], 3)

    def test_genuinely_new_evidence_on_a_suspicious_account_is_logged(self):
        self.r.add_evidence("uid10", signal="self-mirror", detail="gA + gB",
                            games=["gA", "gB"])
        self.r.add_evidence("uid10", signal="self-mirror", detail="gC + gD",
                            games=["gC", "gD"])
        self.assertEqual(len(self.r.data["log"]), 2)
        self.assertFalse(self.r.get("uid10")["needs_recheck"],
                         "still nobody has reviewed it")

    def test_rescanning_a_cleared_account_does_not_resurface_it_repeatedly(self):
        self.r.add_evidence("uid11", signal="self-mirror", detail="gA + gB")
        self.r.set_label("uid11", "normal", by="a_moderator",
                         why="checked the database, this account is fine")
        n = len(self.r.data["log"])
        self.r.add_evidence("uid11", signal="self-mirror", detail="gA + gB")
        self.assertFalse(self.r.get("uid11")["needs_recheck"],
                         "the same finding re-detected is not new evidence")
        self.assertEqual(len(self.r.data["log"]), n)
        self.r.add_evidence("uid11", signal="self-mirror", detail="gC + gD")
        self.assertTrue(self.r.get("uid11")["needs_recheck"],
                        "but a genuinely new finding must resurface it")
        self.assertEqual(len(self.r.data["log"]), n + 1)

    def test_labelling_an_unknown_uid_is_refused(self):
        """A uid is 32 hex characters transcribed by hand off a case filename. One
        wrong character used to create a NEW account row and label it proven_cheater,
        while the account the reviewer meant to label stayed in the queue."""
        import subprocess
        self.r.add_evidence("a" * 32, signal="s", detail="d")
        self.r.save()
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "registry.py")
        p = subprocess.run(
            [sys.executable, script, "--registry", self.path, "label",
             "a" * 31 + "b", "proven_cheater", "--by", "a_moderator",
             "--why", "a typo in the last character of the uid"],
            capture_output=True, text=True)
        self.assertNotEqual(p.returncode, 0, p.stdout + p.stderr)
        with open(self.path, encoding="utf-8") as fh:
            data = json.load(fh)
        self.assertNotIn("a" * 31 + "b", data["accounts"],
                         "a typo must not create a phantom proven_cheater")
        self.assertEqual(len(data["accounts"]), 1)

    def test_a_human_re_deciding_clears_the_recheck_flag(self):
        self.r.add_evidence("uid3", signal="s", detail="d")
        self.r.set_label("uid3", "normal", by="a_moderator", why="cleared after a careful look")
        self.r.add_evidence("uid3", signal="s", detail="new and worse")
        self.assertTrue(self.r.get("uid3")["needs_recheck"])
        self.r.set_label("uid3", "proven_cheater", by="a_moderator",
                         why="re-reviewed with the new evidence and concluded")
        self.assertFalse(self.r.get("uid3")["needs_recheck"])

    def test_rewording_the_signal_sentence_is_not_new_evidence(self):
        """Found by audit. The de-duplication identity WAS the human-readable sentence,
        and the sentences are long explanatory prose written for moderators. Fixing a
        typo, softening a phrase, swapping an em dash — any of it minted a new signal for
        evidence already on file, which counts as `is_new`, which sets `needs_recheck`
        and logs "new evidence arrived after human review". One proof-reading pass
        invalidated every volunteer's clearance and refilled the recheck queue with
        findings they had already dismissed."""
        self.r.add_evidence("uid20", code="mirror.self", detail="gA + gB",
                            signal="mirror: same account in both games")
        self.r.set_label("uid20", "normal", by="a_moderator",
                         why="both games are this player's own practice")
        n_log = len(self.r.data["log"])
        self.r.add_evidence("uid20", code="mirror.self", detail="gA + gB",
                            signal="mirror — the same account sat in both games")
        a = self.r.get("uid20")
        self.assertEqual(len(a["signals"]), 1, "a copy-edit is not a second finding")
        self.assertFalse(a["needs_recheck"],
                         "rewording a sentence must not invalidate a human clearance")
        self.assertEqual(len(self.r.data["log"]), n_log)
        self.assertIn("the same account sat", a["signals"][0]["signal"],
                      "the freshest wording is what a moderator should read")

    def test_upgrading_a_registry_written_before_codes_is_not_new_evidence(self):
        """The fix for the prose key had the prose key's own bug in its upgrade path.

        A registry written by the previous version stores only the sentence. The new
        lookup compares against the slug, which never matches, so the FIRST RUN after
        deploying the fix mints every finding already on file as new — setting
        needs_recheck on every human-cleared account and refilling the recheck queue with
        findings volunteers had dismissed. Deploying the fix would trip it. Verified by
        running it: 1 signal became 2 and a `normal` clearance was invalidated by a rescan
        that found nothing at all.

        Every sentence this project has actually shipped must migrate silently."""
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp, True)
        # The sentences come from mirror_scan's SOURCE, never from the map. Building the
        # fixture out of `LEGACY_CODES.items()` compared the map against itself: an
        # adversarial review changed one character of a map KEY, left mirror_scan alone,
        # and all 117 tests stayed green while the bug came back.
        for i, sentence in enumerate(_emitted_signals()):
            code = REG.LEGACY_CODES.get(sentence)
            with self.subTest(sentence=sentence[:48]):
                self.assertIsNotNone(
                    code,
                    "mirror_scan emits this sentence but LEGACY_CODES has no key for it, "
                    "so upgrading a registry written by this version re-mints every "
                    "finding as new evidence")
                path = os.path.join(tmp, f"legacy_{i}.json")
                with open(path, "w", encoding="utf-8") as fh:
                    # the record shape the PRE-code writer actually produced
                    json.dump({"accounts": {"u1": {
                        "uid": "u1", "name": "P", "label": "normal",
                        "first_seen": "x", "last_seen": "x",
                        "signals": [{"signal": sentence, "detail": "gA + gB",
                                     "first_at": "x", "last_at": "x", "count": 1}],
                        "evidence_games": ["gA", "gB"], "bundles": [],
                        "needs_recheck": False}},
                        "log": [{"at": "x", "uid": "u1", "from": "suspicious",
                                 "to": "normal", "by": "a_moderator", "why": "cleared"}]}, fh)
                r = Registry(path)
                n_log = len(r.data["log"])
                r.add_evidence("u1", code=code, signal=sentence, detail="gA + gB",
                               games=["gA", "gB"])
                a = r.get("u1")
                self.assertEqual(len(a["signals"]), 1,
                                 "the upgrade duplicated a finding already on file")
                self.assertFalse(a["needs_recheck"],
                                 "the upgrade invalidated a human clearance")
                self.assertEqual(len(r.data["log"]), n_log)

    def test_every_signal_the_detector_emits_is_a_key_of_the_legacy_map(self):
        """LEGACY_CODES is a migration only if its keys ARE the sentences shipped.

        The previous version of this asserted the slug appeared in `.values()`, which says
        nothing about the keys — and the keys are the half that has to match. Reading the
        sentences out of the source with `ast` is what makes the assertion independent of
        the thing it is checking."""
        emitted = _emitted_signals()
        self.assertGreaterEqual(len(emitted), 4,
                                f"expected the four shipped signals, found {emitted}")
        for s in emitted:
            self.assertIn(s, REG.LEGACY_CODES, f"unmapped shipped sentence: {s[:70]!r}")

    def test_a_different_code_is_still_new_evidence(self):
        """The fix must not collapse genuinely different findings about one game pair."""
        self.r.add_evidence("uid21", code="mirror.self", detail="gA + gB", signal="x")
        self.r.set_label("uid21", "normal", by="a_moderator", why="looked at it, nothing doing")
        self.r.add_evidence("uid21", code="mirror.seat_inference",
                            detail="gA + gB", signal="y")
        self.assertEqual(len(self.r.get("uid21")["signals"]), 2)
        self.assertTrue(self.r.get("uid21")["needs_recheck"])

    def test_the_detector_passes_stable_codes_not_prose(self):
        """The guard is only worth anything if the production caller uses it.

        This was a source grep for the substring `code=` within 400 characters of each
        `reg.add_evidence(`, which an adversarial review showed cannot fail: `code=None`,
        or the word in a comment, satisfies it. It now RUNS record_evidence against a
        recording double and inspects the actual keyword arguments."""
        # seat inference is opt-in (D1); this test exercises it
        MS.SEAT_ENABLED = True
        self.addCleanup(setattr, MS, "SEAT_ENABLED", False)
        seen = []

        class Recorder:
            def add_evidence(self, uid, **kw):
                seen.append(kw)

        # All THREE branches, not one: a self-attributable pair, and an alt-shape pair
        # that resolves a seat. Covering only the first let a `code=None` mutation in the
        # seat-inference branch through.
        a_line = [760_000_000 + i for i in range(20)]
        b_line = [770_000_000 + i for i in range(20)]
        games = [
            mkgame("R1", "alice", "spec", shared=a_line, start=0.0, end=1000.0),
            mkgame("O1", "spec", "botacct", shared=a_line, bbot=True,
                   start=10.0, end=900.0),
            mkgame("R2", "carol", "dave", shared=b_line, start=0.0, end=1000.0),
            mkgame("O2", "buffer", "botacct", shared=b_line, bbot=True,
                   start=10.0, end=900.0),
        ] + filler()
        surv, _ = MS.scan(games)
        by_acct = collections.defaultdict(list)
        for p in surv:
            real, orac = MS.classify(p)
            subj = MS.subject_of(real, orac)
            if subj:
                for u in subj:
                    by_acct[u].append(p)
            else:
                for u in MS.oracle_humans(orac):
                    by_acct[u].append(p)
                st = MS.seat_correspondence(p)
                if st and st[0]:
                    by_acct[st[0]].append(p)
        self.assertTrue(by_acct, "need pairs to record")
        MS.record_evidence(Recorder(), by_acct, {"botacct"}, {})
        self.assertTrue(seen, "record_evidence recorded nothing")
        self.assertEqual(
            {kw.get("code") for kw in seen},
            {"mirror.self", "mirror.oracle_side", "mirror.seat_inference"},
            "all three attribution branches must be exercised, or a code=None in the "
            "branch this corpus misses goes unnoticed")
        for kw in seen:
            code = kw.get("code")
            self.assertIsInstance(code, str,
                                  f"add_evidence called with code={code!r}: the identity "
                                  f"falls back to the moderator-facing prose")
            self.assertTrue(code, "an empty code is the prose fallback again")
            self.assertLess(len(code), 32, f"{code!r} is a sentence, not a slug")
            self.assertNotIn(" ", code)
            self.assertNotEqual(code, kw.get("signal"),
                                "the code must not BE the sentence")


class SuspectsAgreementTests(unittest.TestCase):
    """`suspects.py` had no tests at all, and it had drifted from mirror_scan.

    It re-derived attribution inline — a hand-spelled oracle-human set and
    `subj = rp & oh` — and never called seat_correspondence. So on the real archive it
    reported 5 pairs for the account mirror_scan attributes 25 to, while printing that the
    real game's players "are NOT attributable and are not named" — contradicting the case
    file mirror_scan had just written naming one of them. Two entry points, two answers.
    """

    def _run(self, rows, enable_seat=False):
        import subprocess
        path = write_archive(rows)
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "suspects.py")
        cmd = [sys.executable, script, "--archive", path]
        if enable_seat:
            cmd.append("--enable-seat")
        try:
            p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8",
                               errors="replace")
        finally:
            os.remove(path)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        return p.stdout

    def _alt_shape_corpus(self):
        """One rated game mirrored by a bot game run by a THIRD account.

        The bot is WHITE in the oracle, so the seat it corresponds to is the rated game's
        WHITE player. Getting this backwards is how the first draft of this test failed:
        it named the players as if the bot's colour did not decide which seat resolves,
        and the code was right while the test was wrong."""
        line = [730_000_000 + i for i in range(24)]
        rows = [
            archive_row("REAL", "receiver", "cleared", shared=line,
                        start=5_000.0, end=6_000.0),
            archive_row("ORAC", "botacct", "buffer", shared=line, wbot=True,
                        start=5_010.0, end=5_900.0),
        ]
        for i in range(300):
            rows.append(archive_row(f"f{i}", f"u{i}a", f"u{i}b",
                                    shared=[next(_ids) * 7_000_003 for _ in range(20)],
                                    start=i * 10.0, end=i * 10.0 + 50.0))
        return rows

    def test_it_names_the_seat_mirror_scan_names_and_clears_the_other(self):
        # Seat inference is opt-in (D1); with it enabled, suspects.py must name the
        # received seat and clear the other, agreeing with the case files.
        out = self._run(self._alt_shape_corpus(), enable_seat=True)
        self.assertIn("receiver", out,
                      "the seat that received the moves must appear, as it does in the "
                      "review queue")
        self.assertNotIn("cleared", out,
                         "the seat the analysis EXCLUDES is somebody's victim and must "
                         "not be listed as a suspect")
        self.assertIn("seat correspondence", out,
                      "the reason must say the attribution is an inference")
        self.assertIn("buffer", out, "the account that ran the oracle is still recorded")

    def test_with_seat_off_by_default_neither_real_seat_is_named(self):
        """The D1 default: without --enable-seat, an alt-shape pair names neither
        rated-game player. The oracle operator is still recorded (it ran the bot game)."""
        out = self._run(self._alt_shape_corpus())   # seat off (default)
        self.assertIn("seat inference OFF", out)
        self.assertNotIn("receiver", out, "no rated-game seat named with seat off")
        self.assertNotIn("cleared", out)

    def test_the_two_entry_points_use_the_same_attribution_helpers(self):
        """The functional test above covers today's shape; this one stops the two files
        drifting apart again, which is how the disagreement arose."""
        src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                "suspects.py"), encoding="utf-8").read()
        for fn in ("MS.subject_of(", "MS.oracle_humans(", "MS.seat_correspondence("):
            self.assertIn(fn, src, f"suspects.py must call {fn} rather than re-derive it")
        # CODE only. The module docstring quotes the old inline version on purpose, as the
        # record of what went wrong, and a naive substring check trips on that.
        code = src.split('"""', 2)[-1]
        self.assertNotIn('orac["wbot"]', code,
                         "a hand-spelled oracle-human set is a second copy of the rule")
        self.assertNotIn("rp & oh", code,
                         "subject_of is THE definition; do not re-implement it")


class RepertoireOwnerTests(unittest.TestCase):
    """Whose habit is allowed to explain a shared line."""

    def test_the_oracle_operators_own_prior_line_exempts_an_unattributable_pair(self):
        """Found by audit. `for uid in subject_of(...)` never ran when subject was empty
        — 15 of the 21 live survivors at the shipped default — so the repertoire
        exemption was silently OFF for the majority of findings while the code and the
        case files both said it applied.
        """
        line = [710_000_000 + i for i in range(20)]
        real = mkgame("R", "alice", "bob", shared=line, start=5_000.0, end=6_000.0)
        orac = mkgame("O", "buffer", "botacct", shared=line, bbot=True,
                      start=5_010.0, end=5_900.0)
        # three of `buffer`'s OWN games, all finished before the real game started
        prior = [mkgame(f"P{i}", "buffer", f"opp{i}", shared=line,
                        start=0.0, end=100.0) for i in range(3)]
        surv, dropped = MS.scan([real, orac] + prior + filler())
        pairs = [p for p in surv if {"R", "O"} == {p["a"]["gid"], p["b"]["gid"]}]
        self.assertEqual(pairs, [], "the line is the oracle operator's own repertoire")
        self.assertTrue(any("repertoire" in k for k in dropped),
                        f"expected a repertoire exclusion, got {list(dropped)}")

    def test_an_exempted_pair_still_contradicts_a_seat_claim(self):
        """Found by adversarial review of the exemption fix itself.

        Both safety guards ran AFTER the exemption, so both were a function of it. Two
        colour-swapped oracles mirroring one rated game resolve OPPOSITE seats and
        `mark_seat_conflicts` withholds both names. Give one oracle-side account three
        cheap prior games containing the line and that pair is exempted — the survivor
        then resolves unopposed and the report PRINTS a name it had been withholding.
        Which of the two rated-game players gets named depends on which oracle-side
        account happened to own the line, and one of them is somebody's victim."""
        line = [740_000_000 + i for i in range(24)]
        real = mkgame("R", "alice", "bob", shared=line, start=5_000.0, end=6_000.0)
        o1 = mkgame("O1", "buf1", "botacct", shared=line, bbot=True,
                    start=5_010.0, end=5_900.0)
        o2 = mkgame("O2", "botacct", "buf2", shared=line, wbot=True,
                    start=5_020.0, end=5_880.0)
        # buf1's own prior games make (R, O1) exempt
        prior = [mkgame(f"P{i}", "buf1", f"opp{i}", shared=line, start=0.0, end=100.0)
                 for i in range(3)]
        games = [real, o1, o2] + prior + filler()
        surv, dropped = MS.scan(games)
        kept = [p for p in surv if "R" in (p["a"]["gid"], p["b"]["gid"])]
        self.assertTrue(any("repertoire" in k for k in dropped),
                        "the O1 pair must actually be exempted for this to be the case "
                        f"under test; dropped={list(dropped)}")
        self.assertTrue(kept, "the O2 pair must still survive")
        for p in kept:
            self.assertIsNone(
                MS.seat_correspondence(p),
                "an exempted pair still contradicts the seat, so both names stay "
                "withheld — otherwise three cheap games choose who gets accused")

    def test_an_exempted_pairs_edge_still_counts_toward_the_component(self):
        """Same root cause, tested at the unit rather than the corpus level.

        `split_clusters` sized the component from the survivor list, so removing a pair
        removed its edge and an exemption could pull a component under
        MAX_COMPONENT_GAMES, admitting pairs that had been dropped as shared theory.

        Tested directly, and deliberately so: I could NOT build a whole corpus where this
        fires, because the repertoire games themselves count toward `df` and MAX_FANOUT
        caps df at 8 — so by the time a component is big enough to matter, the positions
        have stopped being eligible and the pair is dropped for having no shared positions
        rather than by the exemption. The ordering was still wrong and the guard is free;
        what is NOT claimed is a demonstrated end-to-end exploit."""
        def edge(a, b):
            return {"a": {"gid": a}, "b": {"gid": b}}

        chain = [edge(f"g{i}", f"g{i + 1}") for i in range(MS.MAX_COMPONENT_GAMES)]
        kept, n_dropped = MS.split_clusters(chain[:2])
        self.assertEqual(len(kept), 2, "a small component is kept")
        self.assertEqual(kept[0]["component_games"], 3)
        # the same two survivors, with the rest of the chain present only as witnesses
        kept2, _ = MS.split_clusters(chain[:2], extra=chain[2:])
        self.assertEqual(kept2, [], "the witnesses' edges make it a cluster again")

    def test_a_victims_repertoire_does_not_exempt_a_relay_against_them(self):
        """The rejected variant. Extending the exemption to the REAL game's players also
        costs nothing on today's archive — and hands anyone reading this file a way out:
        pick opponents with a known pet defence, steer into it, and the exemption fires
        on the victim's history. An exemption the accused does not own is a loophole."""
        line = [720_000_000 + i for i in range(20)]
        real = mkgame("R", "alice", "victim", shared=line, start=5_000.0, end=6_000.0)
        orac = mkgame("O", "buffer", "botacct", shared=line, bbot=True,
                      start=5_010.0, end=5_900.0)
        prior = [mkgame(f"V{i}", "victim", f"opp{i}", shared=line,
                        start=0.0, end=100.0) for i in range(3)]
        surv, _ = MS.scan([real, orac] + prior + filler())
        pairs = [p for p in surv if {"R", "O"} == {p["a"]["gid"], p["b"]["gid"]}]
        self.assertEqual(len(pairs), 1,
                         "the victim's own pet line must not clear the relay")


class EngineNotationTests(unittest.TestCase):
    """A Hive move names its destination relative to a NEIGHBOUR, so one cell has
    several equally valid spellings. engine_check compared spellings."""

    class FakeEngine:
        """No GPU, no subprocess. score_game only needs these three methods."""

        def __init__(self, cands, reject=()):
            self.cands, self.reject = cands, set(reject)
            self.played = []

        def newgame(self):
            self.played = []

        def analyze(self, sims=1, topk=10):
            return list(self.cands)

        def play(self, mv):
            self.played.append(mv)
            return ["err illegal move"] if mv in self.reject else ["gamestring"]

    def test_two_spellings_of_one_cell_resolve_to_the_same_move(self):
        import engine_check as EC
        pos = {"wS1": (0, 0), "wA1": (-1, 0)}       # wA1 sits west of wS1
        self.assertEqual(EC.resolve("wG1 wA1/", pos), EC.resolve("wG1 \\wS1", pos))
        self.assertEqual(EC.resolve("wG1 /wS1", pos), EC.resolve("wG1 wA1\\", pos))
        self.assertNotEqual(EC.resolve("wG1 -wS1", pos), EC.resolve("wG1 wS1-", pos))
        self.assertEqual(EC.resolve("wS1", {}), ("wS1", (0, 0)))
        self.assertIsNone(EC.resolve("pass", pos))
        self.assertIsNone(EC.resolve("wG1 bQ-", pos), "reference not on the board")

    def test_the_played_move_matches_the_nets_top_choice_despite_the_spelling(self):
        """Measured on 3,209 real plies: only 71.4% of archive move strings appear
        verbatim in the engine's own `validmoves`. The other 28.6% scored rank = -1 —
        'did not play the net's move' — regardless of what the net thought of them.
        Every match rate this file printed was depressed by roughly that much, and the
        aliasing rate rises with how crowded the board is, so it biased late play most.
        """
        import engine_check as EC
        moves = ["wS1", "bS1 wS1-", "wA1 -wS1", "bA1 bS1-", "wG1 wA1/"]
        eng = self.FakeEngine([("wG1 \\wS1", 0.7, 55.0), ("wQ -wA1", 0.3, 45.0)])
        rows = EC.score_game(eng, moves, {0})
        by_ply = {r[0]: r for r in rows}
        self.assertIn(4, by_ply)
        self.assertEqual(by_ply[4][1], 0,
                         "`wG1 wA1/` IS `wG1 \\wS1`; the old string compare said no")
        # and the naive comparison really would have missed it
        self.assertNotIn("wG1 wA1/", [c[0] for c in eng.analyze()])

    def test_a_pass_is_not_scored(self):
        """A pass is legal in Hive only when nothing else is, so it is forced and
        scoring it measures nothing about the player."""
        import engine_check as EC
        eng = self.FakeEngine([("wG1 \\wS1", 1.0, 55.0)])
        rows = EC.score_game(eng, ["wS1", "bS1 wS1-", "pass"], {0, 1})
        self.assertNotIn(2, [r[0] for r in rows])
        self.assertEqual(eng.played, ["wS1", "bS1 wS1-", "pass"])

    def test_a_rejected_move_abandons_the_game(self):
        """The engine keeps its old board after `err`, so every later ply would be
        scored against a position the player never faced — a full, plausible, wrong
        result about another game."""
        import engine_check as EC
        eng = self.FakeEngine([("wS1", 1.0, 55.0)], reject={"bS1 wS1-"})
        with self.assertRaises(EC.IllegalReplay):
            EC.score_game(eng, ["wS1", "bS1 wS1-", "wA1 -wS1"], {0, 1})

    def test_an_unresolvable_move_abandons_the_game_too(self):
        """If our coordinate tracking loses the board while the engine accepts the move,
        every later cell is meaningless. Failing closed is the only safe answer."""
        import engine_check as EC
        eng = self.FakeEngine([("wS1", 1.0, 55.0)])
        with self.assertRaises(EC.IllegalReplay):
            EC.score_game(eng, ["wS1", "bQ bNOPE-"], {0, 1})

    def test_the_games_budget_binds_on_the_uid_path(self):
        """`--games` bound on nothing: the `break` left the inner `for side` loop, so the
        game loop ran to the end of a 500MB archive. It is the flag an operator reaches for
        precisely when they cannot afford the long run.

        This asserted the ORDER OF TWO SOURCE LINES, which an adversarial review pointed
        out never executes main() at all. It now runs main() against a real archive with a
        fake engine and counts how many games the engine actually saw."""
        import engine_check as EC

        started = []

        class CountingEngine:
            def __init__(self, *a, **kw):
                pass

            def newgame(self):
                started.append(1)

            def analyze(self, sims=1, topk=10):
                return [("wS1", 1.0, 55.0)]

            def play(self, mv):
                return ["ok"]

            def close(self):
                pass

        me = uid("budget_subject")
        rows = [archive_row(f"B{i}", "budget_subject", f"opp{i}",
                            shared=[next(_ids) * 3_000_017 for _ in range(20)],
                            start=i * 100.0, end=i * 100.0 + 90.0) for i in range(30)]
        path = write_archive(rows)
        argv, engine_cls = sys.argv, EC.Engine
        try:
            EC.Engine = CountingEngine
            sys.argv = ["engine_check.py", "--archive", path, "--engine", __file__,
                        "--uid", me, "--games", "3"]
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                EC.main()
            out = buf.getvalue()
        finally:
            EC.Engine, sys.argv = engine_cls, argv
            os.remove(path)
        self.assertLessEqual(len(started), 4,
                             f"--games 3 let the engine see {len(started)} games; the "
                             f"budget does not bind")
        self.assertLess(len(started), len(rows),
                        "the whole archive was scored despite --games 3")
        self.assertIn("--games budget", out, "the run must say it stopped early")

    def test_an_abandoned_game_still_costs_against_the_budget(self):
        """The budget counted ROWS KEPT, so a game that fails to replay was free against
        it while having already spent a newgame and one analyze per ply. An account whose
        games mostly fail could run arbitrarily long under a flag that says 3 games."""
        import engine_check as EC

        started = []

        class AlwaysRejects:
            def __init__(self, *a, **kw):
                pass

            def newgame(self):
                started.append(1)

            def analyze(self, sims=1, topk=10):
                return [("wS1", 1.0, 55.0)]

            def play(self, mv):
                return ["err illegal move"]

            def close(self):
                pass

        me = uid("budget_reject")
        rows = [archive_row(f"X{i}", "budget_reject", f"opp{i}",
                            shared=[next(_ids) * 3_000_019 for _ in range(20)],
                            start=i * 100.0, end=i * 100.0 + 90.0) for i in range(30)]
        path = write_archive(rows)
        argv, engine_cls = sys.argv, EC.Engine
        try:
            EC.Engine = AlwaysRejects
            sys.argv = ["engine_check.py", "--archive", path, "--engine", __file__,
                        "--uid", me, "--games", "3"]
            with contextlib.redirect_stdout(io.StringIO()):
                EC.main()
        finally:
            EC.Engine, sys.argv = engine_cls, argv
            os.remove(path)
        self.assertLessEqual(len(started), 4,
                             f"every game was abandoned, so zero rows were kept, and the "
                             f"engine still saw {len(started)} games under --games 3")


class BookFollowBandTests(unittest.TestCase):
    """Sparse rating bands used to fall back to the whole-site median, silently."""

    def test_a_band_with_too_few_peers_widens_before_it_answers(self):
        import book_follow as BF
        rates = [(1500 + i, 20.0) for i in range(20)]          # crowded 1500-1699
        self.assertIsNotNone(BF.band_stats(rates, 1550, 200))
        rates_sparse = [(1520, 20.0), (1530, 21.0)] + [(1750 + i, 30.0)
                                                       for i in range(10)]
        self.assertIsNone(BF.band_stats(rates_sparse, 1520, 200),
                          "two peers is not a baseline")
        wide = BF.baseline_for(rates_sparse, 1520)
        self.assertIsNotNone(wide, "±300 reaches all 12 accounts")
        self.assertEqual(wide[2], "1520±300")

    def test_the_widening_window_is_centred_not_snapped_to_a_multiple(self):
        """An aligned band grows in one direction, so a sparse account near an edge
        never reaches peers a few points across it. 1520 aligned to width 400 gives
        1200-1599 — still two accounts, while ten sit at 1750."""
        import book_follow as BF
        rates = [(1520, 20.0), (1530, 21.0)] + [(1620 + i, 30.0) for i in range(10)]
        self.assertIsNone(BF.band_stats(rates, 1520, 400, aligned=True),
                          "1200-1599 misses every peer at 1620")
        self.assertIsNotNone(BF.band_stats(rates, 1520, 400),
                             "1320-1720 reaches them")

    def test_an_account_with_no_comparable_band_is_unscoreable_not_averaged(self):
        """The global median is dominated by the crowded middle bands, so comparing a
        400-rated account to it measures 'is this player unlike the average player' —
        true by construction at either tail, and nothing to do with the explorer."""
        import book_follow as BF
        rates = [(400, 55.0)] + [(1600 + i, 20.0) for i in range(30)]
        self.assertIsNone(BF.baseline_for(rates, 400),
                          "no peers within 600 points means no comparison exists")
        self.assertIsNotNone(BF.baseline_for(rates, 1610))

    def test_an_unrated_account_is_never_banded(self):
        import book_follow as BF
        rates = [(1600 + i, 20.0) for i in range(30)]
        self.assertIsNone(BF.baseline_for(rates, None))


class DismissTests(unittest.TestCase):
    """The per-finding OFF switch. Every signal the detector emits must be something a
    human can turn off — permanently, auditably, and without muting anything new."""

    def setUp(self):
        self.path = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                 "_test_dismiss_registry.json")
        if os.path.exists(self.path):
            os.remove(self.path)
        self.r = Registry(self.path)

    def tearDown(self):
        if os.path.exists(self.path):
            os.remove(self.path)

    def _finding(self, uid="u1", detail="gA + gB"):
        self.r.add_evidence(uid, code="mirror.self",
                            signal="mirror: same account in both games",
                            detail=detail, games=list(detail.split(" + ")))

    def test_dismiss_requires_a_named_human_and_a_reason(self):
        """Automation must never dismiss its own output, or the false-positive rate
        becomes unmeasurable — the same discipline as set_label.

        The bypasses in this list were found by adversarial review of the FIRST version
        of this guard, which compared `by` to the exact string "detector" and checked
        `len(why)` without stripping. A trailing space, a capital, a tab, or a
        whitespace-only name+reason all slipped through and wrote a nameless, reasonless
        decision into the audit log that export-labels treats as ground truth."""
        self._finding()
        bad = [
            ("detector", "x" * 40),            # the literal automated identity
            ("detector ", "x" * 40),           # trailing space
            ("Detector", "x" * 40),            # capitalised
            ("\tdetector", "x" * 40),          # leading tab
            (" ", "x" * 40),                   # whitespace-only name
            ("a_moderator", "too short"),      # reason under 20 chars
            ("a_moderator", " " * 25),         # whitespace-only reason
        ]
        for by, why in bad:
            with self.subTest(by=repr(by), why=repr(why)):
                with self.assertRaises(SystemExit):
                    self.r.dismiss("u1", code="mirror.self", detail="gA + gB",
                                   by=by, why=why)
        # and the account is untouched by every rejected attempt
        self.assertFalse(self.r.get("u1")["signals"][0].get("dismissed"))

    def test_dismiss_refuses_a_finding_that_does_not_exist(self):
        """Same typo-guard reasoning as `label`: dismissing a phantom finding creates
        the illusion of review without any review."""
        self._finding()
        with self.assertRaises(SystemExit):
            self.r.dismiss("nobody", code="mirror.self", detail="gA + gB",
                           by="a_moderator", why="typo in the uid should be refused")
        with self.assertRaises(SystemExit):
            self.r.dismiss("u1", code="mirror.self", detail="gX + gY",
                           by="a_moderator", why="typo in the detail should be refused")

    def test_a_rescan_does_not_resurrect_a_dismissed_finding(self):
        """The property that makes this an OFF switch rather than a snooze: the weekly
        scan re-finding the same pair lands on the same (code, detail) record, is_new
        stays false, and the dismissal survives with no suppression list anywhere."""
        self._finding()
        self.r.dismiss("u1", code="mirror.self", detail="gA + gB",
                       by="a_moderator", why="reviewed both games; this is analysis, not relay")
        n_log = len(self.r.data["log"])
        self._finding()                      # the rescan
        a = self.r.get("u1")
        self.assertEqual(len(a["signals"]), 1)
        self.assertTrue(a["signals"][0].get("dismissed"),
                        "the rescan must not clear the dismissal")
        self.assertFalse(a["needs_recheck"],
                         "a re-found dismissed finding is not new evidence")
        self.assertEqual(len(self.r.data["log"]), n_log,
                         "and it must not spam the log")

    def test_dismissing_one_finding_never_mutes_the_next(self):
        """A different pair of games is a different detail — a NEW signal. Dismissal
        must be per-finding, not per-account, or it becomes a blanket immunity."""
        self._finding(detail="gA + gB")
        self.r.dismiss("u1", code="mirror.self", detail="gA + gB",
                       by="a_moderator", why="reviewed both games; this one is fine")
        self._finding(detail="gC + gD")
        a = self.r.get("u1")
        live = [s for s in a["signals"] if not s.get("dismissed")]
        self.assertEqual(len(live), 1, "the new finding must surface live")
        self.assertEqual(live[0]["detail"], "gC + gD")

    def test_double_dismiss_is_refused_and_the_log_survives(self):
        self._finding()
        self.r.dismiss("u1", code="mirror.self", detail="gA + gB",
                       by="a_moderator", why="reviewed both games; this is fine")
        with self.assertRaises(SystemExit):
            self.r.dismiss("u1", code="mirror.self", detail="gA + gB",
                           by="someone_else", why="should not silently overwrite a_moderator")
        last = self.r.data["log"][-1]
        self.assertIn("dismissed finding mirror.self", last["why"])
        self.assertEqual(last["by"], "a_moderator")

    def test_show_displays_the_code_that_dismiss_expects_for_legacy_signals(self):
        """Found by adversarial review. `show` read the account dict directly, skipping
        the LEGACY_CODES migration get() runs, so a pre-`code` signal printed only its
        sentence and no code — while dismiss (which does migrate) accepted only the
        never-displayed slug. The documented path ('read the code from show') was a dead
        end for exactly the historical findings the migration exists to preserve."""
        import subprocess
        with open(self.path, "w", encoding="utf-8") as fh:
            json.dump({"accounts": {"u1": {
                "uid": "u1", "name": "P", "label": "suspicious",
                "first_seen": "x", "last_seen": "x",
                "signals": [{"signal": "mirror: same account in both games",
                             "detail": "gA + gB", "first_at": "x", "last_at": "x",
                             "count": 1}],
                "evidence_games": ["gA", "gB"], "bundles": [],
                "needs_recheck": False}}, "log": []}, fh)
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "registry.py")
        p = subprocess.run([sys.executable, script, "--registry", self.path,
                            "show", "u1"], capture_output=True, text=True,
                           encoding="utf-8", errors="replace")
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIn("mirror.self", p.stdout,
                      "show must display the code dismiss --code expects")

    def test_list_ranks_live_findings_above_dismissed_ones(self):
        """An account whose every finding a human already dismissed must not keep
        heading the review queue on the strength of dead signals."""
        import subprocess
        for d in ("g1 + g2", "g3 + g4", "g5 + g6"):
            self._finding(uid="busy", detail=d)
        for d in ("g1 + g2", "g3 + g4", "g5 + g6"):
            self.r.dismiss("busy", code="mirror.self", detail=d,
                           by="a_moderator", why="reviewed all three; repertoire, not relay")
        self._finding(uid="fresh", detail="g7 + g8")
        self.r.save()
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "registry.py")
        p = subprocess.run([sys.executable, script, "--registry", self.path, "list"],
                           capture_output=True, text=True, encoding="utf-8",
                           errors="replace")
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        rows = [l for l in p.stdout.splitlines() if "[" in l and "]" in l]
        self.assertIn("fresh", rows[0],
                      f"the account with a LIVE finding must rank first:\n{p.stdout}")


class KillSwitchTests(unittest.TestCase):
    """The family-level switches: --enable-seat (seat is off by default) and
    --disable linked / --disable self-mirror.

    The requirement they exist for: every signal the tool produces must be something an
    operator can turn off wholesale if it misbehaves in the field, without patching
    code, and with the scan output stating its configuration."""

    @staticmethod
    def _corpus():
        rows = []
        # alt shape: rated receiver-vs-cleared, mirrored by a bot game on `buffer`
        line = [810_000_000 + i for i in range(24)]
        rows.append(archive_row("KSR", "receiver", "cleared", shared=line,
                                start=5_000.0, end=6_000.0))
        rows.append(archive_row("KSO", "botacct", "buffer", shared=line, wbot=True,
                                start=5_010.0, end=5_900.0))
        # linked shape: two bot games, different humans, one line
        l2 = [820_000_000 + i for i in range(24)]
        rows.append(archive_row("KL1", "linkA", "botacct", shared=l2, bbot=True,
                                start=5_000.0, end=5_800.0))
        rows.append(archive_row("KL2", "linkB", "botacct", shared=l2, bbot=True,
                                start=5_020.0, end=5_820.0))
        # self-mirror shape: one human, two colour-swapped bot games
        l3 = [830_000_000 + i for i in range(24)]
        rows.append(archive_row("KM1", "swapper", "botacct", shared=l3, bbot=True,
                                start=5_000.0, end=5_800.0, rated=True))
        rows.append(archive_row("KM2", "botacct", "swapper", shared=l3, wbot=True,
                                start=5_015.0, end=5_815.0, rated=True))
        for i in range(300):
            rows.append(archive_row(f"ksf{i}", f"ka{i}", f"kb{i}",
                                    shared=[next(_ids) * 7_000_003 for _ in range(20)],
                                    start=i * 10.0, end=i * 10.0 + 50.0))
        return rows

    @classmethod
    def _run(cls, disable=(), enable_seat=False):
        import subprocess
        arc = write_archive(cls._corpus())
        out = tempfile.mkdtemp()
        reg = os.path.join(out, "reg.json")
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                              "mirror_scan.py")
        cmd = [sys.executable, script, "--archive", arc, "--out", out,
               "--registry", reg]
        if enable_seat:
            cmd.append("--enable-seat")
        for d in disable:
            cmd += ["--disable", d]
        try:
            p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8",
                               errors="replace", timeout=300)
        finally:
            os.remove(arc)
        assert p.returncode == 0, p.stdout + p.stderr
        cases = ""
        for fn in os.listdir(out):
            if fn.endswith(".md"):
                cases += open(os.path.join(out, fn), encoding="utf-8").read()
        regdata = (json.load(open(reg, encoding="utf-8"))
                   if os.path.exists(reg) else {"accounts": {}})
        codes = {s.get("code") for a in regdata["accounts"].values()
                 for s in a["signals"]}
        shutil.rmtree(out, ignore_errors=True)
        return p.stdout, cases, codes

    def test_seat_off_by_default_withholds_both_names_everywhere(self):
        """The D1 default. Seat inference is opt-in, so a plain run withholds both
        real-game names on every alt-account pair — in every consumer at once, because
        the switch sits inside seat_correspondence itself. --enable-seat opts back in."""
        # default: seat OFF
        stdout, cases, codes = self._run()
        self.assertIn("seat inference OFF", stdout,
                      "a scan must state its own configuration")
        self.assertNotIn("mirror.seat_inference", codes,
                         "no seat-derived evidence may reach the registry by default")
        self.assertNotIn("receiver", cases, "the receiving seat's name is withheld")
        self.assertNotIn("cleared", cases, "and so is the other seat's — both withheld")

        # opt in: seat ON
        stdout, cases, codes = self._run(enable_seat=True)
        self.assertIn("seat inference ON", stdout)
        self.assertIn("mirror.seat_inference", codes,
                      "with --enable-seat the inference fires on this corpus")
        self.assertIn("receiver", cases, "and it names the receiving seat")

    def test_disable_linked_and_self_mirror_skip_those_families(self):
        stdout, cases, codes = self._run()
        self.assertIn("linked.bot_games", codes)
        self.assertIn("self-mirror", stdout)

        stdout, cases, codes = self._run(disable=("linked", "self-mirror"))
        self.assertNotIn("linked.bot_games", codes)
        self.assertNotIn("swapper", stdout,
                         "the self-mirror table must not print at all")
        self.assertNotIn("linkA", stdout)

    def test_suspects_agrees_with_the_seat_default(self):
        """Found by adversarial review. The seat switch must reach suspects.py too, or an
        operator gets alt-account seats named in the review QUEUE while the case files
        withhold them (or vice-versa). Both entry points default OFF and both opt in with
        --enable-seat — a switch one surface ignores is not a switch."""
        import subprocess
        arc = write_archive(self._corpus())
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "suspects.py")
        try:
            off = subprocess.run([sys.executable, script, "--archive", arc],
                                 capture_output=True, text=True, encoding="utf-8",
                                 errors="replace", timeout=300)
            on = subprocess.run([sys.executable, script, "--archive", arc,
                                 "--enable-seat"], capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=300)
        finally:
            os.remove(arc)
        self.assertEqual(off.returncode, 0, off.stdout + off.stderr)
        self.assertEqual(on.returncode, 0, on.stdout + on.stderr)
        self.assertIn("seat inference OFF", off.stdout)
        self.assertNotIn("receiver", off.stdout,
                         "with seat off (default), no alt-pair real-game seat is named")
        self.assertIn("receiver", on.stdout,
                      "with --enable-seat the received seat is named in the queue")

    def test_a_disabled_run_says_so_in_the_case_files_not_just_stdout(self):
        """Found by adversarial review. A case file that withholds names where it
        normally would must state WHY — 'seat inference disabled' (the pair was
        attributable) versus 'not attributable' (it wasn't). Otherwise a durable
        artifact silently conflates an operator choice with an analysis limit."""
        import subprocess
        arc = write_archive(self._corpus())
        out = tempfile.mkdtemp()
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                              "mirror_scan.py")
        try:
            # default run: seat inference off
            p = subprocess.run([sys.executable, script, "--archive", arc, "--out", out],
                               capture_output=True, text=True, encoding="utf-8",
                               errors="replace", timeout=300)
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            text = "".join(open(os.path.join(out, fn), encoding="utf-8").read()
                           for fn in os.listdir(out) if fn.endswith(".md"))
        finally:
            os.remove(arc)
            shutil.rmtree(out, ignore_errors=True)
        self.assertIn("Seat inference is off", text,
                      "the case file must record that the inference was off")
        self.assertIn("--enable-seat", text,
                      "and point at the real flag, not the removed --disable seat")
        self.assertIn("seat inference disabled", text,
                      "and the withheld rows must say why, not 'not attributable'")
        self.assertNotIn("--disable seat", text,
                         "the removed flag must not appear anywhere in a case file")

    def test_the_seat_switch_is_a_single_choke_point(self):
        """Unit-level: the module flag alone decides whether seat_correspondence resolves
        a pair — no consumer needs its own check. Default is OFF (D1)."""
        line = [840_000_000 + i for i in range(24)]
        real = mkgame("R", "alice", "bob", shared=line, start=0.0, end=1000.0)
        orac = mkgame("O", "spec", "botacct", shared=line, bbot=True,
                      start=10.0, end=900.0)
        surv, _ = MS.scan([real, orac] + filler())
        pair = next(p for p in surv if "R" in (p["a"]["gid"], p["b"]["gid"]))
        self.assertIsNone(MS.seat_correspondence(pair),
                          "off by default — no name resolved")
        try:
            MS.SEAT_ENABLED = True
            self.assertIsNotNone(MS.seat_correspondence(pair),
                                 "on when enabled")
        finally:
            MS.SEAT_ENABLED = False


if __name__ == "__main__":
    unittest.main(verbosity=2)
