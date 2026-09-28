#!/usr/bin/env python3
"""MIRROR scan + automatic case-file generation for hivegame.com fair play.

Detects one game's positions being reproduced in another game that was live at the
same time -- the "played the same game against a bot to get its moves" pattern --
and writes a ready-to-read evidence bundle for each flagged account.

    py -3 scripts/fair-play/mirror_scan.py --archive <raw.jsonl> --out logs/fair-play/

Everything runs offline against the public archive. Nothing here decides anything:
the output is a review file for a human admin. See "What this does NOT establish"
in every generated report.

Design notes
------------
Position identity comes from hivegame.com's own canonical hash (engine/src/hasher.rs),
which is invariant to rotation, reflection, translation, move-order transposition and
interchangeable piece labels. Two games sharing a deep position sequence therefore
really are playing the same game, not merely a similar one.

game_type is NOT part of that hash, so Base and Base+MLP positions collide; every
comparison filters on game_type, exactly as the site's own explorer queries do.

Failure posture
---------------
Every ambiguity resolves AGAINST raising a case. Where a signal cannot be trusted the
pair is dropped, not admitted: unreliable timestamps, implausible durations, position
clusters, unrated real games. The detector's job is to lose recall rather than to
manufacture an accusation, and each such drop is counted and printed so the loss is
visible rather than silent.
"""
from __future__ import annotations

import argparse
import collections
import json
import math
import os
import random
import statistics
import sys
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import registry as REG  # noqa: E402


def _utf8_stdout():
    """Don't lose a completed scan to a console codepage.

    Review files are always written as UTF-8, but progress output goes to the
    terminal, and on a Windows console still set to cp1252/cp437 a single em dash
    raises UnicodeEncodeError -- after the expensive part has already run.
    """
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, OSError, ValueError):
        pass

# --- what counts as a position worth comparing ----------------------------------
MIN_PLY = 12          # openings carry no information: everyone plays them
MAX_FANOUT = 8        # a position in >8 games is shared theory, not a private line
MIN_SHARED = 3        # cheap prefilter; the real bar is the bit floor below
REP_MIN_GAMES = 3     # prior games that make a line YOURS rather than evidence
EMPTY_HASH = 0

# A single popularity threshold, applied in exactly one place (`_eligible`).
# There used to be two -- MAX_DF=50 for scoring and MAX_FANOUT=8 for candidacy --
# and the scored set was therefore a superset of the detected set: a pair whose real
# evidence was three rare positions could be reported to an admin as "43 positions
# reproduced against a bot" and graded conclusive on the strength of forty positions
# the detector had already ruled too common to count. One predicate, one place.

# --- evidence floor --------------------------------------------------------------
# Matched positions inside one game are NOT independent observations. They are
# consecutive states of a single trajectory: a game containing the ply-45 position
# necessarily contained the ply-44 one, so summing log2(N/df) over a contiguous run
# counts one coincidence once per ply. Under the old estimator a 111-position mirror
# scored 1689 bits; the conjunction is worth ~15. The error grew with run length,
# i.e. it was largest exactly where the tool was most confident.
#
# What is scored now: each maximal RUN contributes the surprisal of reaching its
# entry position, plus one branching term per step taken inside it. Runs at different
# points in the game are genuinely independent and DO multiply, so they are summed.
#
# The branching factor is measured, not assumed -- but only from positions the corpus
# can actually see branch (df >= BRANCH_MIN_DF). That estimate is censored downward
# (a position seen in 20 games reveals at most 20 successors, against a true legal
# move count several times higher), so the resulting score UNDERSTATES the evidence.
# For a tool that generates accusations, understating is the correct direction.
BRANCH_MIN_DF = 20        # only estimate branching where the corpus can observe it
BRANCH_FALLBACK = 2.0     # used when the corpus is too small to measure it at all
RUN_GAP = 6               # plies either game may skip while still being "the same run"

# KNOWN DEFECT, left in deliberately and unfixed. Read this before touching score_runs.
#
# Every run is charged a full log2(N/df) entry term, which treats each run as an
# independent meeting of the two games. On the live corpus (N=75,934, df=2, measured
# b=2.0, floor 20.00 bits) that has two visible consequences:
#
#   3 matched positions, CONTIGUOUS -> 1 run  -> 15.21 + 2*1.00 = 17.21  DROPPED
#   the same 3 positions, SCATTERED -> 3 runs -> 3 * 15.21      = 45.64  ADMITTED
#
# and, worse because it is simply incoherent, the score is NOT MONOTONE in the
# evidence: deleting six consecutive matched positions splits one run in two and
# RAISES the total, so a corpus defect that punches a hole in a hash array increases
# the reported evidence for the pair it damaged.
#
# It is not fixed here because every constant-sized repair fails. A re-entry cost must
# be > 0, or twenty scattered coincidences score the same as one and F9's property
# (`test_a_contiguous_run_is_one_coincidence_not_many`) inverts. It must also be <=
# the per-step branching term, or splitting a run is still rewarded -- and that term
# is a measured median floored at 1.0, so it can legitimately be 0 bits. No constant
# satisfies both. The repair is a different evidence model, and choosing one with no
# ground truth, during a validation pass, on a tool that names real people, is worse
# than leaving a documented defect.
#
# What bounds the damage in the meantime, measured rather than assumed at the shipped
# default: all but one of the real survivors are a single run, so the per-run entry term
# is charged exactly once and there is nothing for the non-monotonicity to act on. The
# one exception holds 2 runs and clears the floor comfortably either way, and the
# permutation null puts the expected false-pair count for the whole scan near zero. The
# defect is latent on this corpus, not active.

# The floor is multiplicity-corrected against the number of pairs actually tested,
# which the scan already knows (len(cand)). An absolute bit total ignores that a scan
# of 76k games examines billions of candidate pairings, and a fixed count of
# "equivalent near-unique positions" silently means something different at every
# corpus size.
# Default 1e-3, not 1e-2. At 1e-2 the floor is exactly 20.0 bits — the same value as the
# strongest year-apart epoch-control pair, i.e. the weakest admitted findings sit right on
# top of the benign-pair ceiling. 1e-3 lifts the floor to ~23.3 bits and drops the 5
# weakest survivors (all p3, no self-attributable pair, both p1 and all four p2 kept),
# which is the precision-first default this project's own volunteer-moderation constraint
# argues for. `--alpha 0.01` remains available for investigative/research scans.
ALPHA = 0.001             # tolerated probability of one false pair across the scan

# --- position clusters -----------------------------------------------------------
# A relay produces an ISOLATED pair: two games, nobody else. A shared opening line
# produces a clique -- one line played by k games yields C(k,2) pairs, all of which
# used to be counted in the headline and one of which would put whoever happened to
# sit in the bot game at the top of the moderator queue. Components larger than this
# are reported as a cluster, not as an accusation against anybody.
MAX_COMPONENT_GAMES = 6

# --- live-window test ------------------------------------------------------------
# Overlapping at all is far too weak: a multi-day correspondence game replayed
# against a bot the next day still "overlaps". To be a relay rather than analysis,
# the oracle game has to have been open early enough to actually answer questions
# while the real game was still being played. Both bounds are fractions of the real
# game's own duration, so they behave the same for a 3-minute blitz game and a
# six-week correspondence game. Deliberately conservative: a borderline pair is
# dropped rather than reviewed.
MAX_START_PCT = 40.0   # oracle must open within the first 40% of the real game
MIN_COVERAGE = 0.50    # oracle must be open for >=50% of the REAL game's span

# MIN_COVERAGE replaces a statistic that was both exploitable and backwards.
#
# The old test was `overlap / ORACLE duration >= 0.80`: at least 80% of the oracle's
# own span had to fall inside the real game. Lengthening the oracle shrinks that ratio
# and changes nothing else, so the most discriminating filter in the tool was defeated
# by not resigning the bot game. Measured by injecting a relay into the real archive:
# on a median 72.5h correspondence game, playing the bot game 15.9h longer than the
# real game is still caught and 18.1h longer is invisible. `end` is `last_interaction`,
# the last MOVE, so the cost of that evasion is a few extra moves against a bot.
#
# It was also the wrong question. An oracle open for the WHOLE of the real game is the
# most suspicious shape there is, and normalising by the oracle's own duration
# penalised it hardest. The question that matters is "was the oracle open while the
# game was being played", i.e. overlap / REAL duration. That is immune to the evasion
# by construction, because overlap is capped by the real game's span.
#
# Measured on the real archive (survivors / self-attributable / null mean):
#     coverage off (0.00)   29 / 6 / 0.400        0.60   26 / 6 / 0.160
#     0.25                  28 / 6 / 0.200        0.75   22 / 4 / 0.120
#     0.40                  27 / 6 / 0.160        0.90   17 / 3 / 0.040
#     0.50                  26 / 6 / 0.160
# The evasion is caught at EVERY row including 0.00 -- the repair is the change of
# statistic, not the constant. 0.50 is chosen as the plain reading of "open for most
# of the game it was supposedly advising"; the corpus cannot separate it from 0.60
# (no survivor has coverage between 0.421 and 0.649). The self-attributable set is
# unchanged from 0.00 through 0.60, and both p1 pairs sit at coverage 0.981 and 0.998,
# so no value anywhere in this range turns on any particular account.

# --- is the oracle actually a source of strength? --------------------------------
# A bot seat always is. A HUMAN seat only is when the opponent is meaningfully
# stronger AND the oracle game is not faster than the game being played -- you
# cannot consult a blitz game for advice in a correspondence game; the moves come
# from a human playing at speed, and there is nothing to query. Without this,
# "same player, same opening, two concurrent games" trips the detector every time,
# which is the single largest false-positive family in the human-oracle class.
MIN_ORACLE_EDGE = 100.0     # oracle opponent must out-rate the subject by this much
SPEED_RANK = {"Bullet": 0, "Blitz": 1, "Rapid": 2, "Classic": 3,
              "Correspondence": 4, "Untimed": 5}

# SPEED_RANK is also the scan's speed WHITELIST, enforced in load(). Everything that
# reads `speed` defaults open when it does not recognise a value:
#   MAX_PLAUSIBLE_HOURS.get(speed, None) -> no cap    -> no duration quarantine
#   SPEED_RANK.get(orac, 9) < SPEED_RANK.get(real, 0) -> never true -> test passes
# so one unrecognised string quietly disables two exemptions at once. That is the
# same fail-silent shape as the tournament-identity bug, and it is live today:
# shared_types/src/game_speed.rs has a `Puzzle` variant that appears in neither dict
# and in neither pull_archive.py's SPEEDS list. Puzzles are out of scope -- they are
# not two humans competing for rating -- but the point is not to enumerate Puzzle,
# it is that an unknown speed class must fail CLOSED. A detector whose source is
# public should not have a rule that switches off when it meets a value from its
# own repository.

# --- timing trust -----------------------------------------------------------------
# `updated_at` is a row-mutation timestamp, not a game-end timestamp. Every write
# path in db/src/models/game.rs sets it to now(), including finish_timeout(), the
# game-control concatenation and any admin tweak (see apis/src/websocket/ws_hub.rs:57
# -- "admin DB tweak, etc. Measured against game.updated_at"). A correspondence game
# abandoned on day 10 and timed out on day 40 records an end of day 40, which
# inflates the real game's duration fourfold and drives start_pct toward zero and
# overlap toward one -- the live-window test, the most discriminating filter here,
# fails OPEN and admits oracles opened after real play had already stopped.
#
# `last_interaction` is the field that means "when was the last move made", and the
# archive response carries it (apis/src/responses/game.rs:67). Use it. Where it is
# missing, fall back to updated_at but mark the game's timing UNTRUSTED, and drop
# rather than admit any pair that depends on it.
MAX_PLAUSIBLE_HOURS = {          # beyond this a duration is a corrupt row, not a game
    "Bullet": 6.0, "Blitz": 24.0, "Rapid": 24.0 * 7,
    "Classic": 24.0 * 30, "Correspondence": 24.0 * 400, "Untimed": None,
}

# `created_at` is when the game ROW was created, not when play began
# (db/src/models/game.rs:182). For a tournament game with game_start = "Ready" the row
# is created when the tournament is built and play starts only once both players
# declare ready, so created_at can precede the first move by days. That inflates the
# real game's duration, which drives start_pct toward zero and overlap_frac toward
# one -- the live-window test fails OPEN, exactly as it did on the `updated_at` end
# time before F6. The duration cap catches the worst of them (1,142 of 1,842 such
# games on the current archive), but 700 stay under their cap and are trusted with a
# start time we know is wrong. Quarantine on the flag rather than hoping the cap
# catches it. `Immediate` and `Moves` both start when the row is created, so they are
# unaffected -- and all 8 tournament games among the current survivors are
# `Immediate`, so this costs no recall on today's corpus.
STARTS_WHEN_CREATED = ("Moves", "Immediate")

# Prefix marking a counter entry as "kept, but its timing is not trusted" rather than
# "dropped". These are very different facts about the corpus and the run used to print
# both under the word "rejected".
QUARANTINE = "timing quarantine: "


# --- loading -----------------------------------------------------------------
def _ts(s):
    if not s:
        return None
    try:
        return datetime.fromisoformat(str(s).replace("Z", "+00:00")).timestamp()
    except (ValueError, TypeError):
        return None


def _tour_id(t):
    """Tournament identity, not the tournament SNAPSHOT.

    `tournament` on a GameResponse is a whole TournamentAbstractResponse carrying
    games_played, games_total, status and a HashSet of players. Comparing those
    objects for equality -- which is what `ga["tour"] == gb["tour"]` did -- compares
    mutable counters captured at scrape time, and the crawl is partitioned by
    (speed x result) and runs for hours, so two games of the SAME tournament are
    almost never captured at the same moment. The exemption silently never fired.
    """
    if not t:
        return None
    if isinstance(t, dict):
        for key in ("tournament_id", "id", "uuid", "nanoid"):
            v = t.get(key)
            if v:
                return str(v)
        return None
    return str(t)


def load(path, limit=0):
    """Read the raw archive JSONL into flat game records.

    Returns (games, rejected) where `rejected` counts what was dropped and why, so
    that a corpus problem shows up as a printed number rather than as a silently
    more permissive scan.
    """
    out, rejected = [], collections.Counter()
    with open(path, encoding="utf-8") as fh:
        for i, line in enumerate(fh):
            if limit and i >= limit:
                break
            try:
                g = json.loads(line)
            except json.JSONDecodeError:
                rejected["unparseable line"] += 1
                continue
            w, b = g.get("white_player") or {}, g.get("black_player") or {}
            hashes = g.get("hashes") or []
            start = _ts(g.get("created_at"))

            # end-of-play, preferring the field that actually means it
            end, end_src = _ts(g.get("last_interaction")), "last_interaction"
            if end is None:
                end, end_src = _ts(g.get("updated_at")), "updated_at"

            if len(hashes) < MIN_PLY + 4:
                rejected["too few plies to carry evidence"] += 1
                continue
            if start is None or end is None:
                rejected["no usable timestamps"] += 1
                continue

            # Ply integrity. The DB column is Array<Nullable<Int8>> and Game::hashes()
            # drops the NULLs (db/src/models/game.rs:302), which COMPACTS the array:
            # every position after a dropped element then reports the wrong ply. Ply is
            # what MIN_PLY, the run alignment and the branching bucket are all keyed on,
            # so a single gap silently misaligns a whole game against every other. The
            # two lists agree on 80,323 of 80,323 rows of the current archive, so this
            # costs nothing today and fails closed if a backfill ever leaves a gap.
            history = g.get("history") or []
            if history and len(history) != len(hashes):
                rejected["hashes/history length disagree (ply index unreliable)"] += 1
                continue

            speed = g.get("speed")
            if speed not in SPEED_RANK:
                # See the note under SPEED_RANK: an unrecognised speed disables the
                # duration quarantine and the faster-format test simultaneously.
                rejected[f"unsupported speed {speed!r} (out of scope)"] += 1
                continue

            dur_h = (end - start) / 3600.0
            cap = MAX_PLAUSIBLE_HOURS.get(speed, None)
            trusted = end_src == "last_interaction"
            if end < start:
                rejected["end before start (corrupt row)"] += 1
                continue
            if cap is not None and dur_h > cap:
                # Not necessarily corrupt -- but not a duration we can reason about.
                # Quarantine rather than let it widen somebody's live window.
                rejected[f"{QUARANTINE}implausible duration for {speed}"] += 1
                trusted = False
            if g.get("game_start") not in STARTS_WHEN_CREATED:
                # created_at is not the start of play for this game. See the note
                # under STARTS_WHEN_CREATED.
                rejected[f"{QUARANTINE}game_start={g.get('game_start')!r} "
                         f"(created_at is not the start of play)"] += 1
                trusted = False

            out.append({
                "gid": g.get("game_id"),
                "w": w.get("uid"), "b": b.get("uid"),
                "wname": str(w.get("username")), "bname": str(b.get("username")),
                "wbot": bool(w.get("bot")), "bbot": bool(b.get("bot")),
                "wrc": g.get("white_rating_change"), "brc": g.get("black_rating_change"),
                "wr": g.get("white_rating"), "br": g.get("black_rating"),
                "gt": g.get("game_type"), "speed": speed,
                "rated": bool(g.get("rated")), "tour": _tour_id(g.get("tournament")),
                "status": str(g.get("game_status")),
                "conclusion": str(g.get("conclusion")),
                "start": start, "end": end,
                "end_src": end_src, "timing_trusted": trusted,
                "plies": len(g.get("history") or []) or len(hashes),
                "hashes": hashes,
            })
    return out, rejected


# --- the time-independent index ----------------------------------------------
# Split out from the scan so that the permutation null can reshuffle timestamps and
# re-search without paying to rebuild it. Nothing in here reads start/end.
def build_index(games):
    """Position index, document frequencies, and a measured branching factor."""
    ngames = max(len(games), 2)
    idx = collections.defaultdict(list)
    for gi, g in enumerate(games):
        for ply, h in enumerate(g["hashes"]):
            if ply >= MIN_PLY and h != EMPTY_HASH:
                idx[h].append((gi, ply))
    df = {h: len({gi for gi, _ in v}) for h, v in idx.items()}
    games_with = {h: {gi for gi, _ in v} for h, v in idx.items()}

    # Branching, measured only where the corpus can observe it branching at all.
    # Bucketed by ply because a position 14 plies in does not branch like one 60 in.
    succ = collections.defaultdict(set)
    for g in games:
        hs = g["hashes"]
        for p in range(MIN_PLY, len(hs) - 1):
            a, nxt = hs[p], hs[p + 1]
            if a and nxt and df.get(a, 0) >= BRANCH_MIN_DF:
                succ[a].add(nxt)
    by_bucket = collections.defaultdict(list)
    for h, nxts in succ.items():
        ply = min(p for gi, p in idx[h])
        by_bucket[ply // 10].append(len(nxts))
    branching = {k: max(statistics.median(v), 1.0) for k, v in by_bucket.items() if v}
    global_b = max(statistics.median([b for v in by_bucket.values() for b in v]), 1.0) \
        if by_bucket else BRANCH_FALLBACK

    return {"ngames": ngames, "idx": idx, "df": df, "games_with": games_with,
            "branching": branching, "global_b": global_b}


def _b_eff(index, ply):
    return index["branching"].get(ply // 10, index["global_b"])


def _eligible(index, h):
    """The ONE definition of a position worth treating as evidence."""
    d = index["df"].get(h, 0)
    return 2 <= d <= MAX_FANOUT


def _repertoire(games, index, uid, shared, cutoff, exclude):
    """Positions `uid` had already reached in >=REP_MIN_GAMES of their own games
    that FINISHED before `cutoff`.

    Three things this deliberately does that the previous version did not:
      * it is time-ordered. `rep` used to be a corpus-wide count with no time filter,
        so ONE game played at any later date -- a year afterwards -- retroactively
        deleted the evidence for a pair. That is a two-minute, permanent, published
        exemption for anybody who reads this file.
      * it excludes the pair's own two games, so ">=3 of your own prior games" means
        that rather than "one other game, ever".
      * it consults only the SUBJECT. It used to run over `pa | pb`, the union of both
        games' players, so a victim's pet defence exonerated the person relaying
        against them.
    Eligible positions appear in at most MAX_FANOUT games, so this stays cheap.
    """
    out = set()
    for h in shared:
        n = 0
        for gi in index["games_with"].get(h, ()):
            if gi in exclude:
                continue
            g = games[gi]
            if uid in (g["w"], g["b"]) and g["end"] < cutoff:
                n += 1
                if n >= REP_MIN_GAMES:
                    out.add(h)
                    break
    return out


# --- scoring ------------------------------------------------------------------
def runs_of(pts, max_gap=RUN_GAP):
    """Partition aligned (ply_a, ply_b) matches into maximal runs.

    A run is a stretch the two games traversed together. The segmentation itself is
    not new -- it was already computed and then thrown away as a display column.
    """
    out, cur = [], []
    last = None
    for (i, j) in pts:
        if last is not None and 0 < i - last[0] <= max_gap and 0 < j - last[1] <= max_gap:
            cur.append((i, j))
        else:
            if cur:
                out.append(cur)
            cur = [(i, j)]
        last = (i, j)
    if cur:
        out.append(cur)
    return out


def score_runs(index, runs, hash_at_a):
    """Bits of evidence for a set of runs.

    Per run: log2(N / df(entry position)) for arriving at all, then one branching
    term per step taken inside the run. Runs are independent of one another and so
    are summed; positions WITHIN a run are not, and are not.
    """
    n = index["ngames"]
    total = 0.0
    for run in runs:
        entry_h = hash_at_a[run[0][0]]
        d = max(index["df"].get(entry_h, 1), 1)
        bits = math.log2(n / d)
        for (ply_a, _) in run[1:]:
            bits += math.log2(max(_b_eff(index, ply_a), 1.0))
        total += bits
    return total


def floor_bits(n_tests, alpha=ALPHA):
    """Bits required before one hit across `n_tests` comparisons is surprising."""
    return math.log2(max(n_tests, 1)) + math.log2(1.0 / alpha)


# --- detection ---------------------------------------------------------------
def find_pairs(games, index, alpha=ALPHA, require_timing=True):
    """The time-dependent half of the scan. Returns (survivors, dropped).

    `require_timing=False` keeps every position-side filter and drops every temporal
    one. That is not a scanning mode -- it would accuse people of relaying into games
    played years apart -- it exists so `epoch_control` can measure what the position
    evidence alone is worth.
    """
    df, games_with = index["df"], index["games_with"]

    cand = collections.Counter()
    for h, occ in index["idx"].items():
        if not _eligible(index, h):
            continue
        gs = sorted(games_with[h])
        for i in range(len(gs)):
            for j in range(i + 1, len(gs)):
                cand[(gs[i], gs[j])] += 1

    need_bits = floor_bits(len(cand), alpha)
    survivors, dropped = [], collections.Counter()
    witnesses = []          # exempted pairs that still bear on the two guards below
    for (ia, ib), n in cand.items():
        if n < MIN_SHARED:
            dropped["too few shared positions"] += 1; continue
        ga, gb = games[ia], games[ib]
        if ga["gt"] != gb["gt"]:
            dropped["different game type"] += 1; continue
        if (ga["wbot"] and ga["bbot"]) or (gb["wbot"] and gb["bbot"]):
            dropped["bot vs bot (out of scope)"] += 1; continue
        pa, pb = {ga["w"], ga["b"]}, {gb["w"], gb["b"]}
        if pa == pb:
            dropped["same two players (rematch)"] += 1; continue
        if ga["tour"] and ga["tour"] == gb["tour"]:
            dropped["same tournament"] += 1; continue
        if require_timing and (ga["end"] < gb["start"] or gb["end"] < ga["start"]):
            dropped["games never overlapped in time"] += 1; continue

        pair = {"a": ga, "b": gb}
        real, orac = classify(pair)
        if real is None:
            # two bot games, or two human games with no shared account: we cannot
            # tell which is which, so there is nobody to attribute anything to.
            dropped["roles indeterminate"] += 1; continue

        # Timing drives every remaining decision. If we cannot trust it, drop.
        if require_timing and not (real.get("timing_trusted", True)
                                   and orac.get("timing_trusted", True)):
            dropped["timing not trustworthy (end time unreliable)"] += 1; continue

        # first occurrence at or after MIN_PLY, so a repeated position cannot
        # silently re-map a match to a later ply and corrupt the alignment
        ha, hb = {}, {}
        for p, h in enumerate(ga["hashes"]):
            if p >= MIN_PLY and h != EMPTY_HASH and h not in ha:
                ha[h] = p
        for p, h in enumerate(gb["hashes"]):
            if p >= MIN_PLY and h != EMPTY_HASH and h not in hb:
                hb[h] = p

        shared = {h for h in (ha.keys() & hb.keys()) if _eligible(index, h)}
        # WHOSE habit could explain this line? For a self-attributable pair it is the
        # account in both games. For an unattributable one -- 15 of the 21 live
        # survivors -- `subject` is empty, this loop never ran, and the exemption was
        # silently OFF for the majority of findings while the code and the case files
        # both said it applied.
        #
        # It is the ORACLE game's human seat that inherits it, and only that seat. The
        # exemption's meaning is "this account reached these positions repeatedly in its
        # own earlier games, so its game showing them again is not surprising", and for
        # an unattributable pair that is the account whose game the oracle is.
        #
        # Deliberately NOT the real game's two players. That variant was measured too:
        # it also costs nothing today, and it hands anyone who reads this file a way
        # out -- pick opponents with a known pet defence, steer into it, and the
        # exemption fires on the VICTIM's history. An exemption the accused does not
        # own is not an exemption, it is a loophole.
        #
        # Measured on the live archive: 0 of 21 survivors lose a single position to
        # this. It is a consistency fix and a guard, not a change to today's output.
        owners = subject_of(real, orac) or oracle_humans(orac)
        rep = set()
        for uid in owners:
            rep |= _repertoire(games, index, uid, shared, real["start"], {ia, ib})
        novel = shared - rep
        if len(novel) < MIN_SHARED:
            # WITNESS, not gone. The exemption says the LINE is unremarkable; it does not
            # say these two games failed to traverse it together, and both safety guards
            # below reason about exactly that co-traversal.
            #
            # They used to run over the survivor list alone, which made them a function of
            # the exemption. Two consequences, both demonstrated:
            #   * `mark_seat_conflicts` withholds both names when two pairs about one
            #     rated game resolve OPPOSITE seats. Delete one of them here and the other
            #     resolves unopposed — so the report prints a name it had been
            #     withholding, and which of the two rated-game players that is depends on
            #     which oracle-side account happened to own the line. One of those two
            #     people is somebody's victim.
            #   * `split_clusters` sizes the position component from the survivor list.
            #     Removing a pair removes its edge, so an exemption could shrink a
            #     component under MAX_COMPONENT_GAMES and admit pairs that had been
            #     dropped as shared theory.
            #
            # Deliberately only the exemption drop, not the evidence floor. A pair below
            # the floor has not established co-traversal at all — letting coincidence veto
            # a seat claim would be a different rule. An exempted pair HAS established it.
            pair["pts"] = sorted((ha[h], hb[h]) for h in shared)
            witnesses.append(pair)
            dropped["own repertoire / too common"] += 1; continue

        pos_a = {h: ha[h] for h in novel}
        pos_b = {h: hb[h] for h in novel}
        pts = sorted((pos_a[h], pos_b[h]) for h in novel)
        hash_at_a = {p: h for h, p in pos_a.items()}
        runs = runs_of(pts)
        bits = score_runs(index, runs, hash_at_a)
        if bits < need_bits:
            dropped["below evidence floor (multiplicity-corrected)"] += 1; continue

        pair.update({
            "novel": novel, "pts": pts, "runs": runs,
            "chain": max((len(r) for r in runs), default=0),
            "n_runs": len(runs), "bits": bits, "need_bits": need_bits,
            "legacy_bits": sum(math.log2(index["ngames"] / max(df.get(h, 1), 1))
                               for h in novel),
        })

        ok, why, tm = live_window(real, orac)
        pair["timing"] = tm
        if require_timing:
            if not ok:
                dropped[why] += 1; continue
            ok2, why2 = oracle_is_a_source_of_strength(real, orac)
            if not ok2:
                dropped[why2] += 1; continue
            if not real["rated"]:
                # No rating was at stake, so there is no benefit to allege. The
                # site's own bot button creates unrated games
                # (apis/src/pages/challenge_bot.rs:48), so this is about the REAL
                # game only, never the oracle.
                dropped["real game unrated (nothing gained)"] += 1; continue
        survivors.append(pair)

    # Both guards take the witnesses too, so neither is a function of the exemption.
    survivors, clustered = split_clusters(survivors, extra=witnesses)
    if clustered:
        dropped[f"position cluster (>{MAX_COMPONENT_GAMES} games share the line)"] += clustered
    # Must run on the FULL survivor set, not per pair: a contradiction is only visible
    # when two pairs about one rated game are compared to each other.
    conflicts = mark_seat_conflicts(survivors, extra=witnesses)
    if conflicts:
        dropped[f"seat correspondence contradicted itself on {conflicts} rated "
                f"game(s) — both names withheld there"] += 0
    return survivors, dropped


def scan(games, alpha=ALPHA):
    """Convenience wrapper: build the index, then search."""
    return find_pairs(games, build_index(games), alpha)


def split_clusters(survivors, max_games=MAX_COMPONENT_GAMES, extra=()):
    """Drop pairs belonging to a large shared-line component.

    A relay is an isolated two-game component. One popular line played by k games is
    a clique of C(k,2) pairs -- 8 games at the fan-out limit produce 28 "survivors",
    inflating the headline quadratically and, because the queue used to be ordered by
    pair count, putting whoever sat in that cluster's bot game above a genuine
    111-position mirror. Returns (kept, n_dropped).

    `extra` are pairs that co-traversed the line but were exempted. Their EDGES count
    toward component size even though they are never returned: how big the component is
    is a fact about the position data, and it must not shrink because an account the
    operator controls happened to own the line. Without this, an exemption could pull a
    component under `max_games` and admit pairs that had been dropped as shared theory.
    """
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(x, y):
        rx, ry = find(x), find(y)
        if rx != ry:
            parent[rx] = ry

    for p in list(survivors) + list(extra):
        union(p["a"]["gid"], p["b"]["gid"])
    size = collections.Counter(find(g) for g in parent)
    kept = [p for p in survivors if size[find(p["a"]["gid"])] <= max_games]
    for p in kept:
        root = find(p["a"]["gid"])
        p["component"] = root
        p["component_games"] = size[root]
    return kept, len(survivors) - len(kept)


def oracle_is_a_source_of_strength(real, orac):
    """A relay only makes sense if the oracle game can actually supply better moves."""
    if orac["wbot"] or orac["bbot"]:
        return True, ""                       # a bot seat always qualifies
    subject = subject_of(real, orac)
    if not subject:
        return True, ""                       # alt shape: judged elsewhere
    uid = next(iter(subject))
    subj_r = real["wr"] if real["w"] == uid else real["br"]
    opp_r = orac["br"] if orac["w"] == uid else orac["wr"]
    if subj_r is None or opp_r is None:
        return False, "human oracle with unknown ratings (cannot justify)"
    if opp_r < subj_r + MIN_ORACLE_EDGE:
        return False, "human oracle is not stronger than the subject (no advice to gain)"
    if orac["speed"] not in SPEED_RANK or real["speed"] not in SPEED_RANK:
        # Defence in depth: load() already refuses unknown speeds, but this test used
        # to default the unknown side to a rank that made the comparison pass, so a
        # single new enum variant would have switched the rule off rather than
        # tripping it. Unknown means unrankable means drop.
        return False, "unknown speed class (cannot compare formats)"
    if SPEED_RANK[orac["speed"]] < SPEED_RANK[real["speed"]]:
        return False, "human oracle game is faster than the game played (cannot be consulted)"
    return True, ""


def live_window(real, orac):
    """Was the oracle game open early enough, and for long enough, to have been
    answering questions while the real game was live? Returns (ok, reason, metrics)."""
    rdur = max(real["end"] - real["start"], 1.0)
    odur = max(orac["end"] - orac["start"], 1.0)
    start_pct = 100.0 * (orac["start"] - real["start"]) / rdur
    overlap = max(0.0, min(real["end"], orac["end"]) - max(real["start"], orac["start"]))
    # Normalised by the REAL game's span, deliberately. See MIN_COVERAGE.
    coverage = overlap / rdur
    tm = {"start_pct": start_pct, "coverage": coverage,
          "oracle_span_ratio": odur / rdur,
          "start_lag_h": (orac["start"] - real["start"]) / 3600.0,
          "end_lead_h": (real["end"] - orac["end"]) / 3600.0}
    if start_pct > MAX_START_PCT:
        return False, "oracle opened too late (looks like post-game analysis)", tm
    if coverage < MIN_COVERAGE:
        return False, "oracle was not open for enough of the real game", tm
    return True, "", tm


def oracle_humans(orac):
    """The non-bot accounts sitting in the oracle game."""
    return {u for u, isb in ((orac["w"], orac["wbot"]), (orac["b"], orac["bbot"]))
            if u and not isb}


def subject_of(real, orac):
    """THE definition of the account a pair implicates: the HUMAN in both games.

    One predicate, one place -- the same rule `_eligible` follows, and for the same
    reason. This used to be spelled out in six places in two different ways:

        main()                         {real w,b} & oracle_humans(orac)   bot-EXCLUSIVE
        grade / write_case / the rest  {real w,b} & {orac w,b}            bot-INCLUSIVE

    The two agree only while no account is ever flagged `bot` in one row and not in
    another -- true of all 5,756 accounts in the current archive, and an assumption
    about the data rather than anything the code enforces. Where they disagreed, a
    pair `main()` had classified as unattributable would have been graded `p2`,
    written into a review file with BOTH real-game usernames printed, and logged to
    the registry as self-attributable. That is the shape that has already produced
    accusation files about victims twice.
    """
    return ({real["w"], real["b"]} & oracle_humans(orac)) - {None}


def classify(pair):
    """Split the game being played from the game being used as an oracle.

    A bot game is NEVER the game being played. That is checked first and
    unconditionally. It used to be checked only when no account sat in both games,
    so for every self-attributable pair -- the only ones that implicate anybody --
    roles were decided purely by which game was created first, on the reasoning that
    "you cannot mirror a game that has not started yet". You can: nothing stops
    somebody opening the board before their own first move, and in correspondence
    created_at is challenge-acceptance, often hours earlier. When the bot game won
    that race the rated game was cast as a "human oracle", failed the
    MIN_ORACLE_EDGE rule, and the pair vanished. Sixty-one seconds of timestamp was
    the difference between a graded case and silence.

    The oracle does NOT have to be a bot game. Relaying into a game against a
    confederate -- or against a second account you control -- is the same attack.
    """
    ga, gb = pair["a"], pair["b"]
    abot, bbot = ga["wbot"] or ga["bbot"], gb["wbot"] or gb["bbot"]
    if abot and bbot:
        return None, None          # two bot games: benign convergence, out of scope
    if abot != bbot:
        return (gb, ga) if abot else (ga, gb)   # the bot game is the oracle. Always.

    shared = ({ga["w"], ga["b"]} & {gb["w"], gb["b"]}) - {None}
    if shared:
        # Two human games, same person in both. Here start order IS the only
        # available signal, and the earlier game is the one being played.
        return (ga, gb) if ga["start"] <= gb["start"] else (gb, ga)

    return None, None              # two human games, no shared account: unattributable


# --- linked bot games --------------------------------------------------------
# The shape find_pairs cannot express, and used to throw away silently.
#
# `classify` returns (None, None) when BOTH games contain a bot seat, and find_pairs
# drops the pair as "roles indeterminate". The exemption is there for a good reason: a
# bot plays the same replies to the same moves, so two people who each play nokamute
# get similar games for free, and calling that a relay would accuse everybody who ever
# practised against the site's bots.
#
# Measured, that reason only covers shallow agreement. Across the 838 pairs of games
# whose ONLY common account is the same bot and whose humans differ:
#
#       median shared positions   1
#       p90                       4
#       >= 25 positions          11 pairs
#       >= 50 positions           2 pairs
#
# The pairs actually being dropped on the live archive share 73, 46, 40, 33 and 25
# positions. Bot determinism does not produce that; it produces one.
#
# What the shape IS: two humans, each playing a bot, reproducing one trajectory
# between them. One person operating both accounts and passing the bot's replies from
# one game into the other is the obvious reading -- and note it needs no rated human
# victim at all, which is exactly why the relay pipeline cannot see it.
#
# So this is deliberately NOT forced into the real/oracle frame. It implicates nobody
# of beating a human; what it is evidence of is that two accounts are LINKED, which is
# the fact a moderator needs and cannot get from position data any other way.
def find_linked_bot_pairs(games, index, alpha=ALPHA, require_timing=True):
    """Concurrent pairs of BOT games, run by different humans, sharing a deep line.

    Returns pairs shaped like find_pairs' survivors plus `humans`: the two accounts
    the pair links. Same eligibility, same repertoire exemption and the same
    multiplicity-corrected floor -- only the role logic differs.
    """
    cand = collections.Counter()
    for h, occ in index["idx"].items():
        if not _eligible(index, h):
            continue
        gs = sorted(index["games_with"][h])
        for i in range(len(gs)):
            for j in range(i + 1, len(gs)):
                cand[(gs[i], gs[j])] += 1

    need_bits = floor_bits(len(cand), alpha)
    out, dropped = [], collections.Counter()
    for (ia, ib), n in cand.items():
        if n < MIN_SHARED:
            continue
        ga, gb = games[ia], games[ib]
        if ga["gt"] != gb["gt"]:
            continue
        # exactly one bot seat in each game, and a human in each
        ha = {u for u, isb in ((ga["w"], ga["wbot"]), (ga["b"], ga["bbot"]))
              if u and not isb}
        hb = {u for u, isb in ((gb["w"], gb["wbot"]), (gb["b"], gb["bbot"]))
              if u and not isb}
        if len(ha) != 1 or len(hb) != 1:
            continue                       # not one-human-one-bot on both sides
        if ha == hb:
            dropped["same human in both bot games"] += 1; continue
        if ga["tour"] and ga["tour"] == gb["tour"]:
            dropped["same tournament"] += 1; continue
        if require_timing:
            if ga["end"] < gb["start"] or gb["end"] < ga["start"]:
                dropped["never overlapped"] += 1; continue
            if not (ga["timing_trusted"] and gb["timing_trusted"]):
                dropped["timing not trustworthy"] += 1; continue

        ha_, hb_ = {}, {}
        for p, h in enumerate(ga["hashes"]):
            if p >= MIN_PLY and h != EMPTY_HASH and h not in ha_:
                ha_[h] = p
        for p, h in enumerate(gb["hashes"]):
            if p >= MIN_PLY and h != EMPTY_HASH and h not in hb_:
                hb_[h] = p
        shared = {h for h in (ha_.keys() & hb_.keys()) if _eligible(index, h)}
        rep = set()
        for uid in (ha | hb):
            rep |= _repertoire(games, index, uid, shared,
                               min(ga["start"], gb["start"]), {ia, ib})
        novel = shared - rep
        if len(novel) < MIN_SHARED:
            dropped["own repertoire / too common"] += 1; continue

        pos_a = {h: ha_[h] for h in novel}
        pts = sorted((pos_a[h], hb_[h]) for h in novel)
        runs = runs_of(pts)
        bits = score_runs(index, runs, {p: h for h, p in pos_a.items()})
        if bits < need_bits:
            dropped["below evidence floor"] += 1; continue
        out.append({"a": ga, "b": gb, "novel": novel, "pts": pts, "runs": runs,
                    "chain": max((len(r) for r in runs), default=0),
                    "n_runs": len(runs), "bits": bits, "need_bits": need_bits,
                    "humans": (next(iter(ha)), next(iter(hb)))})
    out, clustered = split_clusters(out)
    if clustered:
        dropped["position cluster"] += clustered
    return out, dropped


# --- self-mirror: playing a bot against itself --------------------------------
# The third shape, and the only one with no second person in it at all.
#
# ONE account opens TWO concurrent games against a bot, sitting White in one and Black
# in the other, and passes each game's replies into the other. The bot then plays
# itself. The human contributes no moves of their own and cannot lose both games; what
# they collect is a result against an engine far above their rating, at whatever
# rating deviation they happen to be carrying.
#
# Found by investigating a single account on request. Two games created 53 seconds
# apart, one account against the same bot with the colours swapped, both 83 plies,
# both drawn by repetition, 65 shared positions. Rating change: roughly +290 and +640 (rounded — the exact figures are a join key back to the account).
# (No account names in this public file; the scan output carries them.) Nine hundred
# and thirty-five points from two DRAWN games, because Glicko-2 pays enormously for
# holding a 2550 engine at a fresh RD.
#
# Neither existing detector could see it, and one of the two blind spots is mine:
#   find_pairs()            both games hold a bot -> classify() -> (None, None)
#                           -> "roles indeterminate"
#   find_linked_bot_pairs() requires the two humans to DIFFER
#
# COLOUR-SWAPPED is the whole signature and is why this needs no result-based rule.
# For two same-colour games against one bot to share a line, the player need only
# repeat an opening -- ordinary practice, and 20 such pairs are in the corpus. For a
# colour-SWAPPED pair to share a line, the human's moves in one game must equal the
# bot's moves in the other. Independent play cannot produce that; only relaying can.
def find_self_mirror_pairs(games, index, alpha=ALPHA, require_timing=True):
    """One human, two concurrent bot games, colours swapped, sharing a deep line.

    Returns pairs plus `uid` (the account) and `gain` (its summed rating change over
    the two games, for the report -- NOT used as a filter; the detector does not look
    at results to decide what to flag).
    """
    cand = collections.Counter()
    for h, occ in index["idx"].items():
        if not _eligible(index, h):
            continue
        gs = sorted(index["games_with"][h])
        for i in range(len(gs)):
            for j in range(i + 1, len(gs)):
                cand[(gs[i], gs[j])] += 1

    need_bits = floor_bits(len(cand), alpha)
    out, dropped = [], collections.Counter()
    for (ia, ib), n in cand.items():
        if n < MIN_SHARED:
            continue
        ga, gb = games[ia], games[ib]
        if ga["gt"] != gb["gt"]:
            continue
        ha = oracle_humans(ga)
        hb = oracle_humans(gb)
        if len(ha) != 1 or len(hb) != 1 or ha != hb:
            continue                       # a different human, or not one-human-one-bot
        uid = next(iter(ha))
        if (ga["w"] == uid) == (gb["w"] == uid):
            dropped["same colour in both (ordinary repeated practice)"] += 1; continue
        if require_timing:
            if ga["end"] < gb["start"] or gb["end"] < ga["start"]:
                dropped["never overlapped"] += 1; continue
            if not (ga["timing_trusted"] and gb["timing_trusted"]):
                dropped["timing not trustworthy"] += 1; continue

        ha_, hb_ = {}, {}
        for p, h in enumerate(ga["hashes"]):
            if p >= MIN_PLY and h != EMPTY_HASH and h not in ha_:
                ha_[h] = p
        for p, h in enumerate(gb["hashes"]):
            if p >= MIN_PLY and h != EMPTY_HASH and h not in hb_:
                hb_[h] = p
        shared = {h for h in (ha_.keys() & hb_.keys()) if _eligible(index, h)}
        novel = shared - _repertoire(games, index, uid, shared,
                                     min(ga["start"], gb["start"]), {ia, ib})
        if len(novel) < MIN_SHARED:
            dropped["own repertoire / too common"] += 1; continue

        pos_a = {h: ha_[h] for h in novel}
        pts = sorted((pos_a[h], hb_[h]) for h in novel)
        runs = runs_of(pts)
        bits = score_runs(index, runs, {p: h for h, p in pos_a.items()})
        if bits < need_bits:
            dropped["below evidence floor"] += 1; continue
        gain = sum((g["wrc"] if g["w"] == uid else g["brc"]) or 0.0
                   for g in (ga, gb))
        out.append({"a": ga, "b": gb, "novel": novel, "pts": pts, "runs": runs,
                    "chain": max((len(r) for r in runs), default=0),
                    "n_runs": len(runs), "bits": bits, "need_bits": need_bits,
                    "uid": uid, "gain": gain})
    out, clustered = split_clusters(out)
    if clustered:
        dropped["position cluster"] += clustered
    return out, dropped


# --- seat correspondence -----------------------------------------------------
# Which SEAT of the real game corresponds to the bot's seat in the oracle game.
#
# engine/src/hasher.rs:27 XORs a BLACK_TO_MOVE constant in on turn parity, so the
# canonical hash encodes side-to-move. Two games sharing a position are therefore at
# the same parity, and the colour mapping between them is the identity: White in one
# is White in the other. (Measured on the current real-archive calibration: every
# surviving matched position has a ply offset of exactly 0.)
#
# That is what makes the seat free. In a relay the person sits in the oracle game
# playing their OPPONENT's colour, because the bot has to produce moves for their own:
#
#     real   game:  subject = Black,   opponent = White
#     oracle game:  subject = White (replaying the opponent's moves), bot = Black
#     flow       :  opponent plays White -> subject replays it as White against the
#                   bot -> bot answers as Black -> subject plays that as Black
#
# so the BOT's colour is the colour that received moves, and the human's colour in the
# oracle game is the colour that was being copied.
#
# WIRED INTO THE REPORT. It was not, and the decision to turn it on was taken by a
# human who described the attack independently. Read all of this before changing it.
#
# Two things support the direction:
#   * The colour mapping is measured, not argued: it follows from the hasher, and every
#     matched position across the real survivors has a ply offset of exactly 0,
#     constant within every pair.
#   * On the six self-attributable pairs, where the subject is known independently,
#     the rule reproduces the right seat 6 times out of 6 with 0 disagreements.
#
# The operator's own account of the attack -- a main account playing the rated game,
# a second "buffer" account playing the bot, the opponent's moves relayed into the bot
# game and the bot's replies relayed back -- is the same mechanism, and puts the bot's
# colour on the main account's seat. That is what this returns.
#
# WHAT IT STILL DOES NOT DO. It identifies a SEAT, which is an account, which is not a
# person: the seat may itself be an alt, and linking two accounts to one human needs
# the database, not the archive. It is also conditional on the pair being a relay at
# all -- no field in the archive establishes that. So the grade stays `p3`: a moderator
# still has to do the linkage before concluding anything.
#
# The half of the output that is most reliable, and the reason turning it on protects
# people rather than exposing them, is the EXCLUSION. Before this, the report withheld
# both real-game usernames, which protected the victim and the beneficiary equally.
# Now it says which of the two the pair is not about.
def mark_seat_conflicts(survivors, extra=()):
    """Refuse seat correspondence for any real game whose seat it resolves TWO ways.

    `extra` are pairs that co-traversed the line but were dropped by the repertoire
    exemption. They still COUNT as contradictions even though they are never reported:
    the exemption says the line is unremarkable, not that the two games failed to traverse
    it together, and it is the co-traversal that makes two answers mutually exclusive.
    Without them the guard is a function of the exemption -- delete the contradicting pair
    and the survivor resolves unopposed, printing a name that had been withheld.

    seat_correspondence() reads one pair at a time and answers from the bot's colour
    in that pair's oracle. Nothing stopped two different oracles resolving OPPOSITE
    seats of the SAME rated game -- which is exactly what one person opening two
    colour-swapped analysis boards against a bot produces. Both real-game players then
    got a review file, each naming the other as "excluded by this analysis", from a
    single position component. That is the named-victim failure for the third time,
    and this time with the victim in bold.

    The two answers are mutually exclusive, so at most one is right and nothing in the
    position data says which. The pair keeps its position evidence; only the seat claim
    is withdrawn, which puts the report back to withholding both names -- the posture
    it had before seat correspondence existed.

    On the live archive no real game resolves two ways (checked: 0 of 21), so this
    costs nothing today. It is the guard for the shape that reaches it.
    """
    by_real = collections.defaultdict(set)
    for p in list(survivors) + list(extra):
        real, orac = classify(p)
        if real is None or subject_of(real, orac):
            continue
        s = _seat_raw(p)
        if s:
            by_real[real["gid"]].add(s[0])
    bad = {gid for gid, seats in by_real.items() if len(seats) > 1}
    for p in survivors:
        real, _ = classify(p)
        if real is not None and real["gid"] in bad:
            p["seat_conflict"] = True
    return len(bad)


def _seat_raw(pair):
    """seat_correspondence() without the conflict check, for the conflict check."""
    real, orac = classify(pair)
    if real is None:
        return None
    if not (orac["wbot"] or orac["bbot"]) or (orac["wbot"] and orac["bbot"]):
        return None
    offsets = {pb - pa for (pa, pb) in pair.get("pts", ())}
    if not offsets or any(o % 2 for o in offsets):
        return None
    if orac["wbot"]:
        return real["w"], real["wname"], real["b"], real["bname"]
    return real["b"], real["bname"], real["w"], real["wname"]


# Seat inference is OPT-IN and OFF by default (`--enable-seat` to turn it on). The
# colour/parity mapping is sound, but the step from "this rated seat corresponds to the
# bot's colour" to "this seat received the bot's advice" assumes the oracle was advising
# the player rather than analysing or predicting the opponent — and its 15/15 validation
# is one operation observed 15 times, not 15 independent confirmations. Off by default,
# both real-game names are withheld on every alt-account pair, exactly as before the
# inference existed. External review (D1) asked for this default.
#
# One choke point, checked inside seat_correspondence itself, so the switch reaches EVERY
# consumer at once — attribution, case files, suspects.py, the registry. There is no way
# to enable it for one account: a switch that can be aimed is an editorial decision, and
# this one is operational.
SEAT_ENABLED = False


def seat_correspondence(pair):
    """Returns (recv_uid, recv_name, copied_uid, copied_name) or None.

    `recv` is the real-game seat the oracle's bot corresponds to. `copied` is the
    other seat, which the analysis positively excludes. None when the inference is not
    enabled (SEAT_ENABLED, off unless `--enable-seat`), when the pair has no bot
    oracle, when the ply offsets are not all even -- an odd offset would mean the
    side-to-move term did not survive into the match, and every conclusion here would
    be unsound -- or when another surviving pair resolved the OPPOSITE seat of the same
    rated game (see mark_seat_conflicts). Every one of those fails closed to
    withholding both names.
    """
    if not SEAT_ENABLED or pair.get("seat_conflict"):
        return None
    return _seat_raw(pair)


# --- evidence grading --------------------------------------------------------
# Not every hit deserves the same attention. A whole game mirrored into a bot from
# the opening move is a different object from three coincident positions, and an
# admin should be able to see that before opening the file. The grade never decides
# anything -- it decides queue order and how much of the reviewer's day it deserves.
#
# There is deliberately no tier called `conclusive`. Using a bot game as a live
# analysis board and relaying moves out of one are identical in every field the
# archive records; the difference is intent, which is not in the data. A tier named
# for certainty invites a reader to skip the caveat that says we do not have it.
TIERS = ("p1", "p2", "p3", "unclassified")
P1_MIN_RUN = 40            # one contiguous run this long, not 40 scattered positions
P1_MAX_START_PCT = 15.0


def grade(pair):
    real, orac = classify(pair)
    if real is None:
        return "unclassified", "roles could not be determined"
    n = len(pair["novel"])
    # `chain` allows gaps up to RUN_GAP=6 -- that is the right tolerance for the EVIDENCE
    # model (a relay can skip a few plies and still be a relay), but it is the wrong
    # number to stand behind the word "unbroken". A 40-position `chain` can contain
    # several gaps, and P1's whole meaning is a whole game reproduced move-for-move. So
    # P1 gates on a STRICT contiguous run (max_gap=1), computed only for grading; the
    # evidence score keeps RUN_GAP=6 untouched. Found by external review, which built a
    # gappy 40-`chain` pair and watched grade() call it "one unbroken run of 40".
    strict = max((len(r) for r in runs_of(pair.get("pts", ()), max_gap=1)), default=0)
    tm = pair.get("timing") or {}
    start = tm.get("start_pct", 100.0)
    bot_oracle = orac["wbot"] or orac["bbot"]
    self_attr = bool(subject_of(real, orac))
    if self_attr and bot_oracle and strict >= P1_MIN_RUN and start <= P1_MAX_START_PCT:
        return "p1", (f"one unbroken run of {strict} positions reproduced against a "
                      f"bot, opened {start:.0f}% in, same account in both games")
    if self_attr and bot_oracle:
        return "p2", (f"{n} positions (longest unbroken run {strict}) against a bot, "
                      f"opened {start:.0f}% in, same account in both games")
    if bot_oracle:
        return "p3", (f"{n} positions against a bot, but the played game's player is "
                      f"a different account (needs linkage)")
    return "p3", f"{n} positions, human oracle"


TIER_ORDER = {"p1": 0, "p2": 1, "p3": 2, "unclassified": 3}


def rank_accounts(by_acct):
    """Moderator queue order: best grade first, then strongest single pair.

    NOT pair count. Ranking by volume put whoever happened to sit in the bot game of
    one popular opening line -- which yields C(k,2) pairs from k games -- above an
    account with a single whole-game mirror.
    """
    def key(item):
        _uid, ps = item
        return (min(TIER_ORDER[grade(p)[0]] for p in ps),
                -max(p["bits"] for p in ps))
    return sorted(by_acct.items(), key=key)


# --- per-account evidence ----------------------------------------------------
def account_stats(games, uid, flagged_gids):
    """Every game this account played, with the rating columns marked usable or not.

    `rated` is NOT a filter here. The site's own bot button creates unrated games
    (apis/src/pages/challenge_bot.rs:48, "Play an unrated game vs our bot"), so
    filtering on it emptied the vs-bots bucket entirely and printed a row of zeroes
    directly above a table naming the very bot game in question.
    """
    rows = []
    for g in games:
        for side in ("w", "b"):
            if g[side] != uid:
                continue
            other_bot = g["bbot"] if side == "w" else g["wbot"]
            rc = g["wrc"] if side == "w" else g["brc"]
            opp_rc = g["brc"] if side == "w" else g["wrc"]   # Glicko is NOT symmetric
            opp_r = g["br"] if side == "w" else g["wr"]
            rows.append({"gid": g["gid"], "rated": g["rated"],
                         "rc": rc, "bot": other_bot, "opp_rc": opp_rc,
                         "plies": g["plies"], "speed": g["speed"],
                         "start": g["start"],
                         "opp_rating": opp_r,          # may be None; never coerced to 0
                         "flag": g["gid"] in flagged_gids,
                         "oppname": g["bname"] if side == "w" else g["wname"]})
    return rows


def _med(vals):
    """Median over present values only. A missing rating is not a rating of zero.

    `opp_r or 0` used to turn every unrecorded opponent rating into a 0 and feed it
    to statistics.median, which dragged the baseline down and made the flagged games
    look like they were played against stronger opposition than they were.
    """
    vals = [v for v in vals if v is not None]
    return (statistics.median(vals), len(vals)) if vals else (None, 0)


def _agg(rows):
    rated = [r for r in rows if r["rated"] and r["rc"] is not None]
    net = sum(r["rc"] for r in rated)
    med_plies, _ = _med([r["plies"] for r in rows])
    med_opp, n_opp = _med([r["opp_rating"] for r in rows])
    return dict(n=len(rows), n_rated=len(rated), net=net,
                avg=(net / len(rated)) if rated else None,
                wins=sum(1 for r in rated if r["rc"] > 0),
                rate=(100.0 * sum(1 for r in rated if r["rc"] > 0) / len(rated))
                if rated else None,
                med_plies=med_plies, med_opp=med_opp, n_opp=n_opp)


def _fmt(v, spec="+.2f"):
    return "—" if v is None else format(v, spec)


def write_case(path, uid, rows, pairs, all_games, null=None):
    human = [r for r in rows if not r["bot"]]
    fl = [r for r in human if r["flag"]]
    un = [r for r in human if not r["flag"]]
    bots = [r for r in rows if r["bot"]]
    A, B, Bo = _agg(fl), _agg(un), _agg(bots)

    with open(path, "w", encoding="utf-8") as f:
        w = f.write
        w(f"# Fair-play review file — account `{uid}`\n\n")
        w(f"*Generated {datetime.now(timezone.utc):%Y-%m-%d %H:%M UTC} by "
          f"`scripts/fair-play/mirror_scan.py` from the public hivegame.com archive.*\n\n")
        w("**This is a review file, not a verdict.** It was produced by an automated "
          "detector whose false-positive rate has not been measured against any "
          "confirmed case. Nothing in it should result in action without human review.\n\n")
        if not SEAT_ENABLED:
            # A file that names nobody where it normally would must SAY WHY, or a reader
            # mistakes "the seat inference is off" for "the analysis could not attribute
            # this". They are different facts. Seat inference is OFF by default; a run
            # turns it on with --enable-seat.
            w("> **Seat inference is off (the default; enable with `--enable-seat`).** "
              "Alt-account pairs below withhold both real-game names because the seat "
              "inference was not run, not because the seat could not be resolved.\n\n")
        w("**The detector cannot tell relaying from analysis.** Playing your live game "
          "into a bot to see what it does, and copying the bot's replies back, leave "
          "*identical* traces in every field the archive records. The difference is "
          "intent, and intent is not in this data.\n\n---\n\n")

        if null:
            w("## Empirical false-positive baseline\n\n")
            w(f"The same scan was re-run {null['k']} times with game start times "
              "reshuffled within each speed class, holding every position, hash and "
              "player relationship fixed. That destroys real concurrency and leaves "
              "everything else intact, so it measures how many pairs this detector "
              "produces from coincidence alone.\n\n")
            w("| | observed | shuffled (mean ± sd) |\n|---|---|---|\n")
            w(f"| surviving pairs | {null['observed']} | "
              f"{null['mean']:.1f} ± {null['sd']:.1f} |\n")
            w(f"| self-attributable | {null['observed_self']} | "
              f"{null['mean_self']:.1f} ± {null['sd_self']:.1f} |\n\n")
            if null["mean"] >= null["observed"] * 0.5:
                w("**Read this before anything else: the shuffled baseline is close to "
                  "the observed count.** On this corpus the concurrency test is not "
                  "discriminating, and the pairs below should be treated as position "
                  "coincidences until something else distinguishes them.\n\n")

        w("## Matched game pairs\n\n")
        w("**Oracle opened** is how far into the rated game the other game was started, "
          "and **covered** is the fraction of the rated game during which the other "
          "game was open. A relay has to be open early enough to answer while the game "
          "is still being played, and open for enough of it to keep answering; a "
          "post-game analysis starts late, and an unrelated game covers little. Pairs "
          f"where the other game opened later than {MAX_START_PCT:.0f}% in, or covered "
          f"less than {MIN_COVERAGE * 100:.0f}% of the rated game, are discarded "
          "before this table is built.\n\n")
        w("| Grade | Positions | Longest run | Runs | Evidence | Oracle opened | "
          "Covered | Tournament | Rated game | Concurrent game |\n")
        w("|---|---|---|---|---|---|---|---|---|---|\n")
        for p in sorted(pairs, key=lambda p: (TIER_ORDER[grade(p)[0]], -p["bits"])):
            real, orac = classify(p)
            if real is None:
                continue
            tier, _ = grade(p)
            tm = p.get("timing") or {}
            when = (f"{tm.get('start_pct', 0):.0f}% in (+{tm.get('start_lag_h', 0):.1f} h)"
                    if tm else "—")
            tg = "**yes**" if real.get("tour") else "no"
            # Attribution guard: name people only in the shape where position data
            # can actually say who benefited. In the alt-account shape it cannot,
            # and one of the two players in that game is somebody's victim.
            self_attr = bool(subject_of(real, orac))
            seat = None if self_attr else seat_correspondence(p)
            if self_attr:
                rdesc = f"`{real['gid']}` {real['wname']} vs {real['bname']}"
                odesc = f"`{orac['gid']}` {orac['wname']} vs {orac['bname']}"
            elif seat:
                # The bot's colour maps to exactly one seat, so the other seat is
                # positively EXCLUDED rather than merely unnamed. See the note on
                # seat_correspondence(). The grade stays p3: this identifies an
                # account, and linking it to the oracle-side account needs the
                # database.
                _ru, rname, _ou, oname = seat
                rdesc = (f"`{real['gid']}` **{rname}** ← moves "
                         f"(*{oname} excluded*)")
                odesc = f"`{orac['gid']}` {orac['wname']} vs {orac['bname']}"
            else:
                # Two distinct reasons seat is None, and the file must not conflate
                # them: the operator disabled it (the pair WAS attributable), versus no
                # bot seat / odd ply offsets / a contradicting witness (it genuinely was
                # not). Same withholding, different truth.
                why_withheld = ("seat inference disabled" if not SEAT_ENABLED
                                else "not attributable")
                rdesc = f"`{real['gid']}` *(players withheld — {why_withheld})*"
                odesc = f"`{orac['gid']}` *(players withheld — {why_withheld})*"
            cov = (f"{100.0 * tm['coverage']:.0f}%" if "coverage" in tm else "—")
            w(f"| `{tier}` | {len(p['novel'])} | {p['chain']} | {p['n_runs']} | "
              f"{p['bits']:.0f} / {p['need_bits']:.0f} bits | {when} | {cov} | {tg} | "
              f"{rdesc} | {odesc} |\n")
        w(f"\n*Evidence is scored per contiguous run, not per position: arriving at a "
          f"run costs log2(N/df), each further step inside it costs the measured "
          f"branching factor. Positions inside one run are consecutive states of a "
          f"single trajectory and are not independent observations. The floor "
          f"({pairs[0]['need_bits']:.0f} bits) is corrected for the number of pairs "
          f"this scan actually compared.*\n" if pairs else "\n")

        w("\n## Rating record\n\n")
        if fl and A["avg"] is not None and B["avg"] is not None:
            w("**Treat this as weak, not as corroboration.** The detector selected these "
              "games from position hashes alone, but selection still correlates with "
              "outcome through several paths that this table does not close — see the "
              "caveats underneath it.\n\n")
            w("| Games | n | rated | Net Elo | Per game | Rating-positive |\n"
              "|---|---|---|---|---|---|\n")
            for label, S in (("vs humans — flagged", A), ("vs humans — all others", B),
                             ("vs bots", Bo)):
                rate = "—" if S["rate"] is None else f"{S['rate']:.0f}%"
                w(f"| {label} | {S['n']} | {S['n_rated']} | {_fmt(S['net'], '+.0f')} | "
                  f"{_fmt(S['avg'])} | {rate} |\n")
            w(f"\nDifference: {A['avg'] - B['avg']:+.2f} Elo per game.\n\n")
            w("| | Flagged | Others |\n|---|---|---|\n")
            w(f"| Median game length | {_fmt(A['med_plies'], '.0f')} plies | "
              f"{_fmt(B['med_plies'], '.0f')} plies |\n")
            w(f"| Median opponent rating | {_fmt(A['med_opp'], '.0f')} "
              f"({A['n_opp']}/{A['n']} recorded) | {_fmt(B['med_opp'], '.0f')} "
              f"({B['n_opp']}/{B['n']} recorded) |\n")
            w("\n**Why this is not an independent check.**\n\n")
            w("- *Rating deviation.* Glicko-2 moves a rating in proportion to the "
              "player's RD, which starts near 350 and decays toward 60. Flagged games "
              "are clustered in time by construction — each needs a concurrent second "
              "game — so if that window is early in the account's life the flagged "
              "games sit at high RD and the baseline at low RD, and a large positive "
              "difference appears with no cheating at all. The archive does not "
              "publish RD, so this table cannot rule that out.\n")
            w("- *Length.* Matching needs depth, so flagged games are games that were "
              "played out. Players losing badly resign early. Comparing median lengths "
              "does not close this: a resignation under pressure and a win can have the "
              "same ply count.\n")
            w("- *Opponent strength.* Under Glicko-2 a fixed win rate against stronger "
              "opponents mechanically yields a larger average gain. A higher median "
              "opponent rating in the flagged set is expected, not evidence.\n")
            w("- *Time control.* The live-window test favours games with a long "
              "wall-clock span, which within one account are the games the player took "
              "more time over.\n")
        elif fl:
            w("Flagged games exist but carry no usable rating data.\n")
        else:
            w("No flagged games for this account.\n")

        w("\n## Players affected\n\n")
        vic = collections.defaultdict(lambda: [0, 0.0])
        for r in fl:
            vic[r["oppname"]][0] += 1
            vic[r["oppname"]][1] += (r["opp_rc"] or 0.0)
        if vic:
            w("| Opponent | Flagged games | Their rating change |\n|---|---|---|\n")
            for n_, (c, e) in sorted(vic.items(), key=lambda kv: kv[1][1]):
                w(f"| {n_} | {c} | {e:+.0f} |\n")
            w("\n*Rating changes are the opponent's own recorded delta. Glicko-2 is not "
              "symmetric, so these do not simply mirror the subject's.*\n")
        else:
            w("*None — this account is not attributable as the subject of any pair here.*\n")

        seats = []
        for p in pairs:
            real, orac = classify(p)
            if real is None or subject_of(real, orac):
                continue
            s = seat_correspondence(p)
            if s:
                seats.append((real, orac, s))
        if seats:
            w("\n## Seat correspondence — which seat received the moves\n\n")
            w("In these pairs no single account sits in both games, so the pair cannot "
              "be attributed to a *person* from position data. It can be attributed to "
              "a **seat**.\n\n")
            w("The canonical hash includes a side-to-move term "
              "(`engine/src/hasher.rs:27`), so two games sharing a position are at the "
              "same parity and the colour mapping between them is the identity. In a "
              "relay the person sits in the oracle game playing their *opponent's* "
              "colour — the bot has to produce moves for their own — so the bot's "
              "colour is the colour that received moves.\n\n")
            w("| Rated game | Received the moves | Excluded by this analysis | "
              "Ran the oracle |\n|---|---|---|---|\n")
            for real, orac, (_ru, rname, _ou, oname) in seats:
                who = ", ".join(sorted(
                    n for u, n, b in ((orac["w"], orac["wname"], orac["wbot"]),
                                      (orac["b"], orac["bname"], orac["bbot"]))
                    if u and not b)) or "—"
                w(f"| `{real['gid']}` | {rname} | {oname} | {who} |\n")
            # Computed from THIS run. These were hardcoded as "854 matched positions"
            # and "6 times out of 6" -- numbers measured once, on a different corpus,
            # frozen into every review file ever generated afterwards. It is the one
            # paragraph a moderator has to judge how far the inference has been
            # validated, so a stale number there is worse than no number.
            offs = collections.Counter()
            for p2 in pairs:
                for (pa, pb) in p2.get("pts", ()):
                    offs[pb - pa] += 1
            agree = disagree = 0
            for p2 in pairs:
                r2, o2 = classify(p2)
                if r2 is None:
                    continue
                subj2 = subject_of(r2, o2)
                s2 = _seat_raw(p2)
                if subj2 and s2:
                    if s2[0] in subj2:
                        agree += 1
                    else:
                        disagree += 1
            n_off = sum(offs.values())
            n_zero = offs.get(0, 0)
            w(f"\n**How far this has been checked, on this run.** The colour mapping is "
              f"measured, not argued: {n_zero} of {n_off} matched positions in this "
              f"file have a ply offset of exactly 0"
              + (f" (offsets seen: {dict(sorted(offs.items()))})" if len(offs) > 1
                 else "") + ". ")
            if agree + disagree:
                w(f"On the {agree + disagree} pair(s) here where the subject is "
                  f"*already* known independently — the same account in both games — "
                  f"the rule reproduces the right seat {agree} time(s) and the wrong "
                  f"seat {disagree} time(s). ")
            else:
                w("This file contains no pair whose subject is known independently, so "
                  "it carries **no local check** of the rule at all. ")
            w("That is the only validation available without a confirmed case.\n\n")
            w("**What it still does not establish.** It names an *account*, not a "
              "person: that account may itself be an alt, and linking two accounts to "
              "one human is not something the archive can do. Nor, mostly, can the "
              "database. `db/src/schema.rs` has **no session table and no login "
              "history**, and its only `ip` column is `email_request_log.ip` — a "
              "rate-limit log for password-reset requests, which `email_cleanup.rs` "
              "**deletes after 24 hours**, so it is gone long before any finding here "
              "reaches a reviewer. The identity fields that survive are `users.email`, "
              "`users.created_at`, and `push_devices.device_token`; a token shared "
              "between two accounts is direct device linkage, and it exists only for "
              "accounts that enabled push notifications. This finding is also "
              "conditional on the pair being a relay at all. **The grade stays `p3` for "
              "exactly that reason.**\n\n")
            w("**The most reliable column is the third one.** It says who this pair is "
              "*not* about. Before this analysis the report withheld both names, which "
              "protected the person who benefited and the person who was played "
              "against equally.\n")

        w("\n## What this does NOT establish\n\n")
        w("- **Identity.** `soft_delete` overwrites the username, so a deleted account's "
          "owner cannot be determined from public data. Only the database can answer that.\n")
        w("- **Per-move causality.** In correspondence at days-per-move the site resets the "
          "clock before recording move times, so `move_times` is constant and carries no "
          "information. We can show the games overlapped; we cannot show from the archive "
          "that each bot move preceded the corresponding real move.\n")
        w("- **Intent.** Using a bot game as an analysis board leaves the same trace as "
          "deliberate relaying. Both are covered by the fair-play rule being proposed, but "
          "they are different accusations — and no such rule was published at the time.\n")
        w(f"- **Completeness.** Only pairs clearing {MIN_SHARED} shared positions at "
          f"fan-out ≤ {MAX_FANOUT}, and the multiplicity-corrected bit floor, are visible "
          "here. The true extent is a lower bound.\n")


def write_linked_report(path, linked, bot_uids):
    """The evidence bundle for the linked-accounts finding.

    `record_evidence` is not the only writer to the registry: main() also records every
    `find_linked_bot_pairs` hit. Those accounts therefore appeared in
    `registry.py list --label suspicious` with NO case file, because write_case is built
    around the real/oracle frame and two bot games have neither. On the live archive that
    put 6 accounts in the registry against 4 files, and the two extra are live named
    players. A durable `suspicious` record with nothing for a moderator to read is the
    worst of both: it accuses and it does not show its work.

    Deliberately a separate file rather than a case file. The finding is a different
    claim -- two accounts were operated together -- and neither game has a human
    opponent, so nobody here is alleged to have beaten anyone.
    """
    by_pair = collections.defaultdict(list)
    for p in linked:
        by_pair[tuple(sorted(p["humans"]))].append(p)
    with open(path, "w", encoding="utf-8") as fh:
        w = fh.write
        w("# Linked accounts — two accounts operated together\n\n")
        w("Two concurrent games, each one human against a bot, run by DIFFERENT humans, "
          "reproducing one line between them.\n\n")
        w("**Nobody here is alleged to have beaten anybody.** Neither game has a human "
          "opponent, which is exactly why the relay pipeline cannot see this shape. What "
          "the position evidence establishes is that the two accounts were operated "
          "together; establishing that one *person* holds both needs the database.\n\n")
        w("Two people in the same room practising against the same bot would produce "
          "this too. It is a question for a human, not an answer.\n\n")
        for (u1, u2), ps in sorted(by_pair.items(),
                                   key=lambda kv: -max(p["bits"] for p in kv[1])):
            nm = {}
            for p in ps:
                for g in (p["a"], p["b"]):
                    nm[g["w"]], nm[g["b"]] = g["wname"], g["bname"]
            flag1 = " *(declared bot)*" if u1 in bot_uids else ""
            flag2 = " *(declared bot)*" if u2 in bot_uids else ""
            w(f"## {nm.get(u1, u1)}{flag1}  ↔  {nm.get(u2, u2)}{flag2}\n\n")
            w(f"`{u1}`  ↔  `{u2}`\n\n")
            w("| Positions | Longest run | Evidence | Game A | Game B |\n")
            w("|---|---|---|---|---|\n")
            for p in sorted(ps, key=lambda q: -q["bits"]):
                w(f"| {len(p['novel'])} | {p['chain']} | "
                  f"{p['bits']:.0f} / {p['need_bits']:.0f} bits | "
                  f"`{p['a']['gid']}` {p['a']['wname']} vs {p['a']['bname']} | "
                  f"`{p['b']['gid']}` {p['b']['wname']} vs {p['b']['bname']} |\n")
            w("\n")
        w("\n## What would settle it\n\n")
        w("The database — but check what it actually holds before relying on it. "
          "`db/src/schema.rs` has **no session table and no login history**, and its "
          "only `ip` column is `email_request_log.ip`, a password-reset rate-limit log "
          "that `email_cleanup.rs` purges after 24 hours. The identity fields that "
          "survive are `users.email`, `users.created_at` and "
          "`push_devices.device_token`; a device token shared between two accounts is "
          "direct linkage and the strongest thing available, but it only exists for "
          "accounts that enabled push notifications.\n\n")
        w("If that is not enough — and it usually will not be — then this finding does "
          "not get past `suspicious`, and that is the correct outcome rather than a "
          "gap to work around.\n")


# --- the durable record ------------------------------------------------------
def record_evidence(reg, by_acct, bot_uids, bundles):
    """Attach every surviving pair to the fair-play registry, at `suspicious` and no
    higher. Returns (n_accounts, n_signals).

    ATTRIBUTION IS NOT RE-DERIVED HERE. `by_acct` is the same mapping the case files
    are written from, built by the block in main() -- a subject is an account in BOTH
    games, or the human seat of an unattributable pair's ORACLE game. A real-game
    player of an unattributable pair is not in it, and a pure opponent is not in it.
    Re-deriving the rule in a second place is how it drifts, and the two shapes it
    protects are the two that have each already produced an accusation file about a
    victim.

    `code` and `detail` are together the de-duplication key, so NOTHING that moves
    between runs may go in either. Bit totals, position counts and the grade all shift
    as the corpus grows -- putting any of them in `detail` would append a fresh signal
    on every weekly scan and make the registry's own ordering a function of how often
    the tool was run, which is the bug F15 fixed. The stable identity of a finding is
    the KIND of finding plus the unordered pair of game ids; the volatile numbers live
    in the case file, which is linked as the `bundle`.

    `signal` is the sentence a moderator reads and is free to reword -- which is
    exactly why it is no longer the key. It used to be, and rewording it counted as new
    evidence: see registry.add_evidence.
    """
    n_acct = n_sig = 0
    for uid, pairs in by_acct.items():
        if not uid or uid in bot_uids:
            # Never write evidence about a declared bot. The oracle seat of a relay is
            # the bot, and it is the one participant that certainly did nothing.
            continue
        name = None
        for p in pairs:
            real, orac = classify(p)
            if real is None:
                continue
            for g in (real, orac):
                for u, nm, isb in ((g["w"], g["wname"], g["wbot"]),
                                   (g["b"], g["bname"], g["bbot"])):
                    if u == uid and not isb:
                        name = nm
        for p in pairs:
            real, orac = classify(p)
            if real is None:
                continue
            if subject_of(real, orac):
                code = "mirror.self"
                signal = "mirror: same account in both games"
            elif uid in oracle_humans(orac):
                code = "mirror.oracle_side"
                signal = "mirror: ran the oracle game of an unattributable pair"
            else:
                # Reached only via seat correspondence, and named as such: this
                # account was not observed in the oracle game at all. The basis is
                # an inference from the bot's colour, and a moderator reading
                # `registry.py show` has to be able to see that it is.
                code = "mirror.seat_inference"
                signal = ("mirror: seat correspondence says this seat received the "
                          "moves (inference — account linkage needs the database)")
            gids = tuple(sorted((real["gid"], orac["gid"])))
            reg.add_evidence(uid, name=name, code=code, signal=signal,
                             detail=f"{gids[0]} + {gids[1]}",
                             games=list(gids), bundle=bundles.get(uid))
            n_sig += 1
        n_acct += 1
    return n_acct, n_sig


# --- permutation null --------------------------------------------------------
def permute_times(games, rng):
    """Reshuffle start times within speed class, preserving each game's duration.

    Everything the index depends on -- positions, hashes, players, game_type -- is
    untouched, so the only thing destroyed is real concurrency. Whatever survives is
    what this detector produces from coincidence.
    """
    out = [dict(g) for g in games]
    by_speed = collections.defaultdict(list)
    for i, g in enumerate(games):
        by_speed[g["speed"]].append(i)
    for _, idxs in by_speed.items():
        starts = [games[i]["start"] for i in idxs]
        rng.shuffle(starts)
        for i, s in zip(idxs, starts):
            out[i]["start"] = s
            out[i]["end"] = s + (games[i]["end"] - games[i]["start"])
    return out


def _self_attributable(pairs):
    n = 0
    for p in pairs:
        real, orac = classify(p)
        if real is None:
            continue
        if subject_of(real, orac):
            n += 1
    return n


def null_distribution(games, index, k, seed=0, alpha=ALPHA):
    """Run the pair search k times over shuffled timestamps. Reuses the index."""
    rng = random.Random(seed)
    counts, selves = [], []
    for i in range(k):
        shuffled = permute_times(games, rng)
        surv, _ = find_pairs(shuffled, index, alpha)
        counts.append(len(surv))
        selves.append(_self_attributable(surv))
        print(f"   null {i + 1}/{k}: {len(surv)} pairs, {selves[-1]} self-attributable",
              flush=True)
    sd = (lambda v: statistics.stdev(v) if len(v) > 1 else 0.0)
    return {"k": k, "mean": statistics.mean(counts), "sd": sd(counts),
            "mean_self": statistics.mean(selves), "sd_self": sd(selves)}


def epoch_control(games, index, min_gap_days=365, alpha=ALPHA):
    """Score distribution over pairs that CANNOT be relays, because the two games
    are separated by a year or more.

    Calibrates the position half of the evidence independently of the timing half.
    Whatever bit totals show up here are what innocent position sharing looks like:
    if far-apart pairs routinely score as high as the flagged ones, the score is not
    separating relays from shared theory and the timing filters are carrying the
    entire result.

    Note this deliberately runs with the temporal filters OFF -- with them on, no
    surviving pair can be a year apart, so the control would trivially return zero.
    """
    surv, _ = find_pairs(games, index, alpha, require_timing=False)
    gap = min_gap_days * 86400.0
    far = [p for p in surv if abs(p["a"]["start"] - p["b"]["start"]) >= gap]
    bits = sorted((p["bits"] for p in far), reverse=True)
    return {"n": len(far), "top": bits[:10],
            "median": statistics.median(bits) if bits else None,
            "max": bits[0] if bits else None}


# --- main --------------------------------------------------------------------
def main():
    _utf8_stdout()
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", required=True)
    ap.add_argument("--out", default="logs/fair-play")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--min-pairs", type=int, default=1,
                    help="only write a case file for accounts in >= this many pairs")
    ap.add_argument("--alpha", type=float, default=ALPHA,
                    help="tolerated probability of one false pair across the whole scan")
    ap.add_argument("--null-permutations", type=int, default=0, metavar="K",
                    help="measure the false-positive baseline over K timestamp shuffles")
    ap.add_argument("--epoch-control", action="store_true",
                    help="score pairs a year or more apart, which cannot be relays, "
                         "to calibrate the position evidence on its own")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--registry", nargs="?", const=REG.DEFAULT_PATH, default=None,
                    metavar="PATH",
                    help="also record every surviving pair in the fair-play registry, "
                         "raising each implicated account to `suspicious` and no "
                         "higher (default path: %s)" % REG.DEFAULT_PATH)
    ap.add_argument("--enable-seat", action="store_true",
                    help="turn the seat inference ON (default OFF). When off, both "
                         "real-game names are withheld on every alt-account pair, as "
                         "before the inference existed. See the D1 note in the source.")
    ap.add_argument("--disable", action="append", default=[], metavar="FAMILY",
                    choices=("linked", "self-mirror"),
                    help="skip a whole signal family for this run (repeatable). "
                         "`linked`: the linked-accounts detector. `self-mirror`: the "
                         "rating-adjustment detector. The relay core cannot be "
                         "disabled; if the relay core is wrong, do not run the tool. "
                         "(The seat inference is OFF by default; use --enable-seat.)")
    args = ap.parse_args()

    global SEAT_ENABLED
    if args.enable_seat:
        SEAT_ENABLED = True
    # A scan's output must state its own configuration, or two runs become incomparable
    # without the shell history that produced them.
    cfg = []
    cfg.append("seat inference ON" if SEAT_ENABLED else "seat inference OFF (default)")
    if args.disable:
        cfg.append("disabled: " + ", ".join(sorted(set(args.disable))))
    if args.alpha != 0.001:
        cfg.append(f"alpha={args.alpha}")
    print("config — " + "; ".join(cfg))

    games, rejected = load(args.archive, args.limit)
    print(f"loaded {len(games):,} games")
    # Dropped and quarantined are different facts and used to print under one word.
    # A quarantined game is still IN the corpus -- it still contributes to df, it can
    # still be somebody's opponent -- it just cannot be used as timing evidence.
    for k, v in rejected.most_common():
        if not k.startswith(QUARANTINE):
            print(f"   dropped — {k}: {v:,}")
    for k, v in rejected.most_common():
        if k.startswith(QUARANTINE):
            print(f"   kept, timing untrusted — {k[len(QUARANTINE):]}: {v:,}")
    untrusted = sum(1 for g in games if not g["timing_trusted"])
    if untrusted:
        print(f"   {untrusted:,} of {len(games):,} loaded game(s) have untrusted "
              f"timing — pairs using them are dropped, not admitted")

    index = build_index(games)
    survivors, dropped = find_pairs(games, index, args.alpha)
    print(f"{len(survivors)} pairs survived every exemption")
    for k, v in dropped.most_common():
        print(f"   excluded — {k}: {v:,}")

    tiers = collections.Counter(grade(p)[0] for p in survivors)
    print("   grades: " + (", ".join(f"{t}={tiers[t]}" for t in TIERS if tiers[t])
                           or "none"))
    comps = len({p["component"] for p in survivors})
    print(f"   {comps} distinct position component(s) — a relay is an isolated pair, "
          f"so this, not the pair count, is the number of independent events")

    # --- the shape the relay pipeline cannot express -------------------------
    linked = ([] if "linked" in args.disable
              else find_linked_bot_pairs(games, index, args.alpha)[0])
    if linked:
        pairs_by_accts = collections.defaultdict(list)
        for p in linked:
            pairs_by_accts[tuple(sorted(p["humans"]))].append(p)
        print(f"\n{len(linked)} pair(s) of concurrent BOT games run by DIFFERENT "
              f"humans reproduce one line — {len(pairs_by_accts)} account pair(s)")
        print("   this is evidence two accounts are LINKED, not that anybody beat a "
              "human: neither game has a human opponent")
        for (u1, u2), ps in sorted(pairs_by_accts.items(),
                                   key=lambda kv: -max(p["bits"] for p in kv[1])):
            nm = {}
            for p in ps:
                for g in (p["a"], p["b"]):
                    nm[g["w"]], nm[g["b"]] = g["wname"], g["bname"]
            print(f"   {nm.get(u1, u1)}  <->  {nm.get(u2, u2)}   "
                  f"{len(ps)} pair(s), up to {max(len(p['novel']) for p in ps)} "
                  f"positions, {max(p['bits'] for p in ps):.0f} bits")

    # --- rating adjustment, NOT a fair-play accusation -----------------------
    # Deliberately kept out of `by_acct`, out of the review queue and out of the
    # registry. There is no opponent here: one person, two games, one bot playing
    # itself. What it distorts is a rating, so what it needs is a rating correction,
    # and calling it cheating would put someone in a queue headed by banned accounts
    # for something categorically different.
    selfm = ([] if "self-mirror" in args.disable
             else find_self_mirror_pairs(games, index, args.alpha)[0])
    if selfm:
        by_uid = collections.defaultdict(list)
        for p in selfm:
            by_uid[p["uid"]].append(p)
        print(f"\n{len(selfm)} colour-swapped self-mirror pair(s) — "
              f"{len(by_uid)} account(s). NOT a cheating finding: no opponent is "
              f"involved, this is a RATING ADJUSTMENT list.")
        print("   one account, two concurrent games against a bot with the colours "
              "swapped, each game's replies fed into the other, so the bot plays "
              "itself and the account banks a result against an engine far above it")
        print(f"   {'account':<24} {'pairs':>6} {'net rating':>11} {'deepest':>8}")
        for uid, ps in sorted(by_uid.items(),
                              key=lambda kv: -sum(p["gain"] for p in kv[1])):
            nm = "?"
            for p in ps:
                for g in (p["a"], p["b"]):
                    for u, n in ((g["w"], g["wname"]), (g["b"], g["bname"])):
                        if u == uid:
                            nm = n
            print(f"   {str(nm)[:24]:<24} {len(ps):>6} "
                  f"{sum(p['gain'] for p in ps):>+11.0f} "
                  f"{max(len(p['novel']) for p in ps):>8}")
        print("   *the rating column is reported for the correction, and is not used "
              "to decide what is flagged — the signature is the colour swap*")

    null = None
    if args.null_permutations:
        print(f"\nmeasuring the false-positive baseline over "
              f"{args.null_permutations} timestamp shuffles...")
        null = null_distribution(games, index, args.null_permutations,
                                 args.seed, args.alpha)
        null["observed"] = len(survivors)
        null["observed_self"] = _self_attributable(survivors)
        print(f"\n  observed          : {null['observed']} pairs, "
              f"{null['observed_self']} self-attributable")
        print(f"  shuffled baseline : {null['mean']:.1f} ± {null['sd']:.1f} pairs, "
              f"{null['mean_self']:.1f} ± {null['sd_self']:.1f} self-attributable")
        if null["mean"] >= null["observed"] * 0.5:
            print("  ** the concurrency test is NOT discriminating on this corpus **")

    if args.epoch_control:
        print("\nscoring pairs >=1 year apart (cannot be relays)...")
        ec = epoch_control(games, index, alpha=args.alpha)
        obs = sorted((p["bits"] for p in survivors), reverse=True)
        print(f"  {ec['n']} such pair(s); bits median {_fmt(ec['median'], '.0f')}, "
              f"max {_fmt(ec['max'], '.0f')}")
        if obs:
            print(f"  flagged pairs for comparison: median "
                  f"{statistics.median(obs):.0f}, max {obs[0]:.0f} bits")
        if ec["max"] is not None and obs and ec["max"] >= max(obs):
            print("  ** innocent far-apart pairs score as high as the flagged ones — "
                  "the position score is not separating relays from shared theory **")

    # ------------------------------------------------------------------
    # ATTRIBUTION. Getting this wrong writes a "fair-play review file" about
    # somebody's VICTIM, so the rule is deliberately conservative:
    #
    #   subject  = the human who is in BOTH games (real game + oracle game).
    #              They are the only person the pair can implicate on its own.
    #   oracle   = the human seat of the bot game, when that is a different
    #              account (the alt-account shape). Recorded as linked, and
    #              given its own file only because such accounts typically have
    #              zero human opponents, which is itself the thing to review.
    #   victim   = the suspect's opponent in the real game. NEVER a subject,
    #              and in the alt shape not even NAMED -- the pairs table
    #              withholds both real-game usernames, because position data
    #              cannot say which of the two benefited and the other one is
    #              somebody's victim.
    # ------------------------------------------------------------------
    by_acct = collections.defaultdict(list)
    unattributed = []
    victims_only = set()
    for p in survivors:
        real, orac = classify(p)
        if real is None:
            continue
        real_players = {real["w"], real["b"]} - {None}
        orac_human = oracle_humans(orac)
        subject = subject_of(real, orac)
        if subject:
            for uid in subject:
                by_acct[uid].append(p)
            victims_only |= (real_players - subject)
        else:
            unattributed.append(p)
            for uid in orac_human:          # linked oracle-side account
                by_acct[uid].append(p)
            seat = seat_correspondence(p)
            if seat:
                # The bot's colour resolves WHICH seat received the moves, so the
                # real game's two players are no longer interchangeable. Without
                # this the account that actually gained the rating is invisible in
                # the queue and only the buffer account it ran the bot on shows up.
                # The seat it clears stays a victim and gets nothing.
                recv_uid, _rn, other_uid, _on = seat
                if recv_uid:
                    by_acct[recv_uid].append(p)
                victims_only |= (real_players - {recv_uid})
            else:
                victims_only |= real_players
    protected = len(victims_only - set(by_acct))
    print(f"\n{protected} account(s) appeared only as an opponent — no file written for them")
    seated = sum(1 for p in unattributed if seat_correspondence(p))
    print(f"{len(unattributed)} pair(s) have no account in both games (alt-account "
          f"shape) — of those, {seated} resolve to a SEAT, which names an account but "
          f"not a person; linking it needs the database")

    os.makedirs(args.out, exist_ok=True)
    flagged_gids = collections.defaultdict(set)
    for uid, ps in by_acct.items():
        for p in ps:
            real, _ = classify(p)
            if real and uid in (real["w"], real["b"]):
                flagged_gids[uid].add(real["gid"])

    bot_uids = set()
    for g in games:
        if g["wbot"] and g["w"]:
            bot_uids.add(g["w"])
        if g["bbot"] and g["b"]:
            bot_uids.add(g["b"])

    written, bundles, reported = 0, {}, {}
    for uid, ps in rank_accounts(by_acct):
        if len(ps) < args.min_pairs or uid in bot_uids:
            continue
        rows = account_stats(games, uid, flagged_gids[uid])
        if not rows:
            continue
        # The full uid, not a 12-character prefix. Two accounts sharing a prefix would
        # have silently overwritten each other's review file, and the file that
        # survived would have carried the other account's name in its heading.
        path = os.path.join(args.out, f"case_{uid}.md")
        write_case(path, uid, rows, ps, games, null)
        written += 1
        bundles[uid] = path
        reported[uid] = ps
        tier = min((grade(p)[0] for p in ps), key=lambda t: TIER_ORDER[t])
        print(f"  wrote {path}  [{tier}]  ({len(ps)} pairs, "
              f"{len(flagged_gids[uid])} flagged games)")
    print(f"\n{written} case files in {args.out}/")

    # The linked-accounts finding gets its own bundle, because it is recorded in the
    # registry from a different code path and used to arrive there with nothing attached.
    linked_path = None
    if linked:
        linked_path = os.path.join(args.out, "linked_accounts.md")
        write_linked_report(linked_path, linked, bot_uids)
        n_linked_accts = len({u for p in linked for u in p["humans"]
                              if u and u not in bot_uids})
        print(f"  wrote {linked_path}  ({n_linked_accts} account(s) in "
              f"{len(linked)} pair(s))")

    if args.registry:
        reg = REG.Registry(args.registry)
        n_acct, n_sig = record_evidence(reg, reported, bot_uids, bundles)
        # Linked accounts are recorded too, with a signal that says what the finding
        # actually is. Neither game has a human opponent, so nobody here is alleged to
        # have beaten anyone: the evidence is that two accounts were operated together.
        for p in linked:
            u1, u2 = p["humans"]
            nm = {}
            for g in (p["a"], p["b"]):
                nm[g["w"]], nm[g["b"]] = g["wname"], g["bname"]
            gids = tuple(sorted((p["a"]["gid"], p["b"]["gid"])))
            for me, other in ((u1, u2), (u2, u1)):
                if not me or me in bot_uids:
                    continue
                reg.add_evidence(
                    me, name=nm.get(me), code="linked.bot_games",
                    signal=("linked accounts: a concurrent bot game run by another "
                            "account reproduces this account's line (no human "
                            "opponent in either game)"),
                    detail=f"{gids[0]} + {gids[1]}", games=list(gids),
                    bundle=linked_path)
                n_sig += 1
        reg.save()
        print(f"\nregistry {args.registry}: {n_sig} finding(s) attached to "
              f"{n_acct} account(s) from the relay pipeline")
        # RECONCILE the two counts. The registry has more accounts than there are case
        # files whenever a linked-accounts finding fires, and the gap used to be silent:
        # a moderator running `list --label suspicious` saw accounts with no bundle.
        in_reg = {u for u, a in reg.data["accounts"].items() if a["signals"]}
        no_bundle = sorted(u for u in in_reg
                           if not reg.data["accounts"][u].get("bundles"))
        print(f"   {len(in_reg)} account(s) carry evidence; {written} relay case file(s)"
              + (" + linked_accounts.md" if linked else ""))
        if no_bundle:
            print(f"   ** {len(no_bundle)} account(s) in the registry have NO evidence "
                  f"bundle — a moderator cannot read what they are accused of:")
            for u in no_bundle:
                nm = reg.data["accounts"][u].get("name") or u[:12]
                print(f"      {nm}  [{u}]")
        labels = collections.Counter(a["label"] for a in reg.data["accounts"].values())
        print("   labels now: "
              + ", ".join(f"{k}={v}" for k, v in labels.most_common()))
        print("   the detector cannot set anything above `suspicious`; "
              "`normal` and `proven_cheater` need a named human and a written reason")
        recheck = [a for a in reg.data["accounts"].values() if a.get("needs_recheck")]
        if recheck:
            print(f"   {len(recheck)} account(s) have new evidence since a human "
                  f"decided — `registry.py list --recheck`")


if __name__ == "__main__":
    main()
