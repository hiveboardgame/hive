#!/usr/bin/env python3
"""Second-stage engine check: how closely does an account track the NET's policy?

This is NOT a screen. `mirror_scan.py` is the screen -- it runs over 76k games in
12 seconds and hands back a handful of accounts. This looks at those accounts only,
and it exists for the one cheating shape the mirror detector structurally cannot
see: somebody running an engine off-site, where there is no second game to match
positions against.

    # 1. start the net's eval server (SEE THE WARNING BELOW)
    py -3 tools/az_eval_server.py data/nets/<net>.pt data/az/_serve --serve-port 8899

    # 2. label a corpus of KNOWN engine and KNOWN human play, and fit the threshold
    py -3 scripts/fair-play/engine_check.py --archive archive.jsonl --calibrate

    # 3. score the accounts the screen produced
    py -3 scripts/fair-play/engine_check.py --archive archive.jsonl --uid <uid> ...

WARNING, from StockBee's own audit (docs/attic/audit-2026-07-18.md): the eval server
shares the only local GPU with everything else on that host, and when it is starved
the seated bot silently falls back to NNUE *mid-rated-game*. Do not start this while
the live bot is playing unless you have the GPU lock in place. On a box with a spare
GPU, or against a cluster node, none of that applies.

Why the transformer and not NNUE
--------------------------------
NNUE is beatable at human top level, so "did not match NNUE" says nothing. The
6.6M HiveFormer is the production brain and plays at a strength no human on the
site reaches, so its policy is a meaningful reference. It also gives the thing the
archive could not: a per-position DIFFICULTY measure. Policy entropy over the legal
moves says how many reasonable choices existed, which is exactly the conditioning
variable that makes match rate discriminating rather than a proxy for rating --
see the failed archive-only attempt documented in README.md.

Cost
----
One `analyze sims 1` is ONE forward pass. Measured serving throughput is ~15.4k
leaves/s on the local RTX 5070 and 65.6k on an A100, so:

    one 60-ply game        ~4 ms
    one account, 3k moves  ~0.2 s
    the whole 2.18M-move human corpus  ~142 s

The cost objection to engine-based checking does not survive contact with an AZ-style
policy head: you are not searching, you are doing one inference per position, and the
policy head ranks every legal move in that single pass.

Calibration without confirmed cheaters
--------------------------------------
The blocker on every engine-based check so far has been that there is no ground
truth, so no threshold can be set. There IS ground truth for a *different* question,
and it is already in the archive: the `bot` flag. 31,247 games have a bot seat and
45,268 are human-vs-human, which labels ~765k engine moves and ~2.18M human moves.

So the model is trained to answer "were these moves produced by an engine or by a
human", not "is this person cheating". That is a question with real labels, a held-out
split, and therefore a MEASURABLE false-positive rate -- the number that decides
whether anything may be shown to a moderator.

It still does not measure recall against cheaters, and it never will without
confirmed cases. What it does give is the thing the difficulty experiment lacked: a
threshold with an error bar instead of a guess.
"""
from __future__ import annotations

import argparse
import collections
import json
import math
import os
import queue
import statistics
import subprocess
import sys
import threading

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mirror_scan as MS  # noqa: E402

DEFAULT_ENGINE = os.path.join("..", "StockBee", "build_clang_strict", "stockbee.exe")


class IllegalReplay(RuntimeError):
    """The engine refused a move from the archive, so the board has diverged.

    Raised rather than swallowed: silently continuing scores every later ply against a
    position the player never faced, which produces a full, plausible, wrong result.
    """


class Engine:
    """A stockbee.exe speaking UHP, replayed forward one move at a time.

    Replaying incrementally matters: a game is `newgame` once plus one `play` per
    ply, not a fresh replay per position. That turns O(n^2) move applications into
    O(n) and is the difference between this being usable and not.
    """

    def __init__(self, path, port=None, variant="Base+MLP", timeout=90.0):
        args = [path]
        if port:
            args += ["--transformer-port", str(port)]
        self.p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, text=True, bufsize=1)
        self.variant = variant
        self.timeout = timeout
        # A reader thread so a wedged engine raises instead of blocking the process
        # for ever. The GPU eval server has a documented failure mode where the
        # worker thread dies and every request then hangs with no timeout
        # (StockBee docs/attic/audit-2026-07-18.md), and `analyze` deliberately has
        # NO NNUE fallback, so "wait for stdout" can genuinely never return.
        self._q = queue.Queue()
        self._dead = False
        self._t = threading.Thread(target=self._pump, daemon=True)
        self._t.start()
        self._read_to_ok()

    def _pump(self):
        try:
            for line in self.p.stdout:
                self._q.put(line)
        except Exception:                                  # noqa: BLE001
            pass
        self._q.put(None)                                  # EOF sentinel

    def _read_to_ok(self):
        """Read until the engine's `ok`. ALWAYS until `ok`, including after `err`.

        This used to return early on an `err` line -- and UHP prints `err ...`
        followed by its own `ok`:

            > play zzz bad move
            err could not parse move: zzz bad move
            ok

        so the trailing `ok` was left in the pipe. From then on the reader was one
        response behind: every command consumed the PREVIOUS command's `ok` and
        returned instantly with the wrong lines, until eventually a read waited for
        an `ok` that had already been eaten and blocked for ever. It survives
        thousands of clean plies and then wedges the moment one command errors,
        which is exactly how it presented -- two sweeps that ran fine for a while and
        then sat at 0% GPU with the engines idle.
        """
        out = []
        while True:
            try:
                line = self._q.get(timeout=self.timeout)
            except queue.Empty:
                self._dead = True
                raise RuntimeError(
                    f"engine did not answer within {self.timeout:.0f}s "
                    f"(wedged eval server?); last lines: {out[-3:]}")
            if line is None:
                self._dead = True
                raise RuntimeError("engine exited")
            line = line.rstrip("\r\n")
            if line == "ok":
                return out
            out.append(line)

    def cmd(self, s):
        self.p.stdin.write(s + "\n")
        self.p.stdin.flush()
        return self._read_to_ok()

    def newgame(self):
        return self.cmd(f"newgame {self.variant}")

    def play(self, mv):
        return self.cmd(f"play {mv}")

    def analyze(self, sims=1, topk=10):
        """Returns [(move, prior, winprob)], best first. One forward pass at sims=1."""
        out = self.cmd(f"analyze sims {sims} topk {topk}")
        cands = []
        for line in out:
            if not line.startswith("cand "):
                continue
            parts = [x.strip() for x in line[5:].split("|")]
            mv = parts[0]
            d = {}
            for p in parts[1:]:
                k, _, v = p.partition(" ")
                d[k] = v.strip()
            try:
                cands.append((mv, float(d.get("prior", 0.0)),
                              float(d.get("winprob", 50.0))))
            except ValueError:
                continue
        return cands

    def close(self):
        try:
            self.p.stdin.write("exit\n")
            self.p.stdin.flush()
        except Exception:
            pass
        try:
            self.p.wait(timeout=5)
        except Exception:
            self.p.kill()


def uhp_moves(history):
    """The archive stores history as [(piece, destination)] pairs; UHP wants them
    joined. A pass is recorded as ('pass', '')."""
    out = []
    for mv in history:
        if not isinstance(mv, (list, tuple)) or len(mv) != 2:
            return None
        piece, dest = str(mv[0]), str(mv[1])
        out.append(piece if not dest or dest == "." else f"{piece} {dest}")
    return out


# --- move identity ------------------------------------------------------------
# A Hive move names its destination RELATIVE TO A NEIGHBOUR, and a cell with several
# occupied neighbours has several equally valid spellings. `wG1 wA1/` and `wG1 \wS1`
# are the same move whenever wA1 sits west of wS1.
#
# This was compared as a STRING against the net's candidate list. The candidates come
# out of the engine's own generator, so they carry the engine's spelling, and the
# archive carries hivegame.com's. Measured over 3,209 real plies: only 71.4% of archive
# move strings appear verbatim in the engine's `validmoves`. The other 28.6% were
# scored `rank = -1` -- counted as "did not play the net's move" -- no matter what the
# net thought of them. Every match rate this file has ever printed was depressed by
# roughly that much, and not uniformly: the aliasing rate rises with how crowded the
# board is, i.e. with ply, so it biased the late game hardest.
#
# Reading the engine's echo back does not fix it: `play` echoes the GameString with the
# move VERBATIM (measured: 0 of 1,707 re-rendered), so the engine never states a
# canonical form. Resolving both sides to absolute axial coordinates does, exactly, and
# costs no extra engine calls.
#
# Validated against the engine's own move generator over the same 3,209 plies: the
# archive move resolves to EXACTLY ONE entry of `validmoves` 3,187 times, to none 0
# times, to more than one 0 times. The other 22 are passes.
LEFT = {"-": (-1, 0), "/": (-1, 1), "\\": (0, -1)}    # glyph BEFORE the reference
RIGHT = {"-": (1, 0), "/": (1, -1), "\\": (0, 1)}     # glyph AFTER the reference


def resolve(mv, pos):
    """(piece, (q, r)) for a UHP move string, given the current piece -> cell map.

    Returns None for a pass, and for a reference to a piece not on the board -- both of
    which mean "no cell identity", never "matches anything".
    """
    mv = (mv or "").strip()
    if not mv or mv.lower() == "pass":
        return None
    parts = mv.split()
    piece = parts[0]
    if len(parts) == 1:
        return piece, (0, 0)              # opening placement defines the origin
    ref = parts[1]
    if ref[0] in LEFT:
        d, name = LEFT[ref[0]], ref[1:]
    elif ref[-1] in RIGHT:
        d, name = RIGHT[ref[-1]], ref[:-1]
    else:
        d, name = (0, 0), ref             # no glyph: climbing on top of `ref`
    base = pos.get(name)
    if base is None:
        return None
    return piece, (base[0] + d[0], base[1] + d[1])


def entropy(priors):
    tot = sum(priors) or 1.0
    e = 0.0
    for p in priors:
        q = p / tot
        if q > 0:
            e -= q * math.log2(q)
    return e


def score_game(eng, moves, want_sides, sims=1, topk=10):
    """Per-move features for the plies where a side we care about moved.

    Returns [(ply, rank, prior_of_played, entropy, n_cands)] -- rank 0 = the net's
    own first choice. `entropy` is the difficulty conditioner: low entropy means the
    net saw one obvious move and agreeing with it proves nothing.

    Passes produce no row. A pass is legal in Hive only when nothing else is, so it is
    a forced move and scoring it measures nothing about the player.
    """
    eng.newgame()
    rows = []
    pos = {}                    # piece -> axial cell, so moves compare by CELL not text
    for i, mv in enumerate(moves):
        played = resolve(mv, pos)
        if (i % 2) in want_sides and played is not None:
            cands = eng.analyze(sims=sims, topk=topk)
            if cands:
                priors = [c[1] for c in cands]
                # Compare where the move GOES, not how it is spelled. See resolve().
                cells = [resolve(c[0], pos) for c in cands]
                at = cells.index(played) if played in cells else -1
                rank = at
                pr = priors[at] if at >= 0 else 0.0
                rows.append((i, rank, pr, entropy(priors), len(cands)))
        # A REJECTED move is fatal to everything after it. The engine keeps its old
        # board, so the next analyze scores a DIFFERENT position from the one the
        # player faced -- and with the side to move flipped if the rejection was a
        # missing pass. Nothing crashes; the rows just quietly describe another game.
        # Abandon the game instead of returning numbers about a board that never
        # existed.
        if any(str(x).startswith("err") for x in eng.play(mv)):
            raise IllegalReplay(f"engine rejected ply {i}: {mv!r}")
        if played is not None:
            pos[played[0]] = played[1]
        elif (mv or "").strip().lower() != "pass":
            # Our own coordinate tracking has lost the board while the ENGINE accepted
            # the move, so every later cell is suspect. Same reasoning as above: a
            # silent skip would keep scoring, against cells that no longer mean
            # anything. A pass is the one legitimate None.
            raise IllegalReplay(f"could not resolve ply {i} to a cell: {mv!r}")
    return rows


def iter_games(archive, limit=0, want=None, human_only=False, bot_only=False):
    n = 0
    with open(archive, encoding="utf-8") as fh:
        for line in fh:
            try:
                g = json.loads(line)
            except json.JSONDecodeError:
                continue
            h, hist = g.get("hashes") or [], g.get("history") or []
            if len(h) != len(hist) or len(h) < 8:
                continue
            if g.get("game_type") != "MLP":
                continue
            w, b = g.get("white_player") or {}, g.get("black_player") or {}
            wb, bb = bool(w.get("bot")), bool(b.get("bot"))
            if human_only and (wb or bb):
                continue
            if bot_only and not (wb or bb):
                continue
            if want and not ({w.get("uid"), b.get("uid")} & want):
                continue
            yield g, w, b, wb, bb
            n += 1
            if limit and n >= limit:
                return


def summarise(rows, label=""):
    """Match rate overall and split by how much choice the net actually had."""
    if not rows:
        return None
    hard = [r for r in rows if r[3] >= 1.5]     # >= ~2.8 equally-weighted options
    easy = [r for r in rows if r[3] < 1.5]

    def rate(rs):
        return (100.0 * sum(1 for r in rs if r[1] == 0) / len(rs)) if rs else None
    return {"label": label, "n": len(rows), "top1": rate(rows),
            "top1_hard": rate(hard), "n_hard": len(hard),
            "top1_easy": rate(easy), "n_easy": len(easy),
            "mean_prior": statistics.mean(r[2] for r in rows),
            "median_entropy": statistics.median(r[3] for r in rows)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", required=True)
    ap.add_argument("--engine", default=DEFAULT_ENGINE)
    ap.add_argument("--port", type=int, default=8899,
                    help="the net's eval server; analyze REQUIRES it")
    ap.add_argument("--uid", action="append", default=[],
                    help="account to score (repeatable)")
    ap.add_argument("--calibrate", action="store_true",
                    help="score known-engine and known-human play to fit a threshold")
    ap.add_argument("--games", type=int, default=40)
    ap.add_argument("--bot-games", action="store_true",
                    help="also score the account's games against a bot. Off by default "
                         "because the calibration baseline is human-vs-human, but it is "
                         "the ONLY way to score a buffer account, which has no others.")
    ap.add_argument("--sims", type=int, default=1,
                    help="1 = one forward pass, the policy alone. >1 adds search.")
    ap.add_argument("--variant", default="Base+MLP")
    args = ap.parse_args()

    MS._utf8_stdout()
    if not os.path.exists(args.engine):
        raise SystemExit(f"engine not found: {args.engine}\n"
                         f"point --engine at stockbee.exe")

    eng = Engine(args.engine, port=args.port, variant=args.variant)
    try:
        if args.calibrate:
            # The labels are already in the archive: `bot` marks engine play.
            buckets = {"ENGINE (bot seats)": [], "HUMAN (human-vs-human)": []}
            abandoned = 0

            def take(bucket, mv, sides):
                nonlocal abandoned
                try:
                    buckets[bucket] += score_game(eng, mv, sides, sims=args.sims)
                except IllegalReplay as ex:
                    abandoned += 1
                    print(f"# abandoned a {bucket} game: {ex}")

            for g, w, b, wb, bb in iter_games(args.archive, limit=args.games,
                                              bot_only=True):
                mv = uhp_moves(g["history"])
                if not mv:
                    continue
                take("ENGINE (bot seats)", mv,
                     {s for s, isb in ((0, wb), (1, bb)) if isb})
            for g, w, b, wb, bb in iter_games(args.archive, limit=args.games,
                                              human_only=True):
                mv = uhp_moves(g["history"])
                if not mv:
                    continue
                take("HUMAN (human-vs-human)", mv, {0, 1})
            if abandoned:
                print(f"# {abandoned} game(s) could not be replayed and were abandoned")
            print(f"{'population':<26} {'n':>7} {'top1':>7} {'top1|hard':>10} "
                  f"{'n hard':>8} {'top1|easy':>10} {'H':>6}")
            for k, rows in buckets.items():
                s = summarise(rows, k)
                if not s:
                    continue
                f = lambda v: "-" if v is None else f"{v:.1f}%"
                print(f"{k:<26} {s['n']:>7} {f(s['top1']):>7} "
                      f"{f(s['top1_hard']):>10} {s['n_hard']:>8} "
                      f"{f(s['top1_easy']):>10} {s['median_entropy']:>6.2f}")
            print("\nthe separation between those two rows on the HARD column is the "
                  "whole signal; if it is small, no threshold on this will be safe")
        else:
            want = set(args.uid)
            if not want:
                raise SystemExit("give --uid (repeatable) or --calibrate")
            per = collections.defaultdict(list)
            # `--games` is a budget, and it used to bind on nothing here: the `break`
            # below left the `for side` loop, so the game loop ran to the end of a
            # 500MB archive regardless. With the eval server at ~4ms/game that is the
            # difference between 10 seconds and hours, and it is the flag an operator
            # reaches for precisely when they cannot afford the long run.
            # The budget counts WORK ATTEMPTED, not rows kept. Counting rows made an
            # abandoned game free against the budget while it had already spent a
            # `newgame` and one `analyze` per ply of the account's side -- so an account
            # whose games mostly fail to replay could run arbitrarily long under a flag
            # that says 40 games. `--games` says games, so games is what it counts.
            cap_games = max(args.games, 1)
            cap_plies = cap_games * 60
            attempted = abandoned = 0
            for g, w, b, wb, bb in iter_games(args.archive, limit=0, want=want,
                                              human_only=not args.bot_games):
                if attempted >= cap_games or \
                        sum(len(v) for v in per.values()) >= cap_plies:
                    print(f"# stopped at the --games budget: {attempted} game-side(s) "
                          f"attempted, {sum(len(v) for v in per.values())} plies scored")
                    break
                mv = uhp_moves(g["history"])
                if not mv:
                    continue
                for side, p in ((0, w), (1, b)):
                    if p.get("uid") in want:
                        attempted += 1
                        try:
                            per[p["uid"]] += score_game(eng, mv, {side}, sims=args.sims)
                        except IllegalReplay as ex:
                            # One unreplayable game must not take the account's whole
                            # score with it, but it must not vanish either -- a silent
                            # skip turns "we could not read 40% of this account" into a
                            # confident number over the other 60%.
                            abandoned += 1
                            print(f"# abandoned {g['game_id'][:12]}: {ex}")
            if abandoned:
                print(f"# {abandoned} game(s) could not be replayed and were abandoned")
            print(f"{'uid':<34} {'n':>7} {'top1':>7} {'top1|hard':>10} {'n hard':>8}")
            for uid, rows in per.items():
                s = summarise(rows, uid)
                if not s:
                    continue
                f = lambda v: "-" if v is None else f"{v:.1f}%"
                print(f"{uid:<34} {s['n']:>7} {f(s['top1']):>7} "
                      f"{f(s['top1_hard']):>10} {s['n_hard']:>8}")
            # SAY SO when an account produced nothing. `human_only=True` drops every game
            # with a bot seat, and the accounts the screen produces most of -- the p3
            # buffer accounts -- have ONLY bot games. They therefore score zero rows and
            # used to fall out of the table entirely, which reads as "checked, nothing
            # found" when it means "not checked". `--bot-games` opts in: those moves are
            # still the account's own moves, and if the account is relaying a bot's replies
            # they are the bot's moves, which is exactly what this file measures.
            silent = [u for u in want if not per.get(u)]
            for u in silent:
                print(f"# {u}: NOTHING SCORED. This is not a clean result — it means no "
                      f"human-vs-human game was found for the account. Buffer accounts "
                      f"typically have none; pass --bot-games to score their bot games.")
            print("\ncompare against --calibrate; a number without that baseline "
                  "means nothing")
    finally:
        eng.close()


if __name__ == "__main__":
    main()
