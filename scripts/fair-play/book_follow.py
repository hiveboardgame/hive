#!/usr/bin/env python3
"""How often does an account play the BOOK move, past the opening?

The opening explorer is queried by position and ranks continuations by how often they
were played. Anyone can paste a position from their own live game into it and play
whatever comes top. That needs no engine, no second account and no bot, and it leaves
NO TRACE in the archive -- the lookup never becomes a game. Nothing else in this
directory can see it, because everything else works by pairing two games together and
here there is only one.

What CAN be measured after the fact is the consequence: an account that reads the
explorer mid-game plays the most-popular continuation more often than its peers, and
specifically past the point where "book" is still common knowledge.

    py -3 scripts/fair-play/book_follow.py --archive archive.jsonl

THIS IS A SUSPICION MARK, NOT EVIDENCE. Read the caveats at the bottom of the output
before doing anything with it. People do play from memory, strong players converge on
popular moves for the same reason those moves are popular, and neither of those is
cheating. It is a number that says "look here", in a directory whose entire posture is
that a number never decides anything.

Three design decisions carry the whole thing
--------------------------------------------
1. THE BOOK EXCLUDES THE PLAYER'S OWN GAMES. Without that, an account with a pet line
   "follows the book" that its own earlier games created, and the heaviest repertoire
   players -- the most ordinary thing in the game -- come out top. This is the same
   trap `_repertoire` exists for in mirror_scan.py, and it would have inverted the
   result here.

2. ONLY POSITIONS WHERE A BOOK ACTUALLY EXISTS COUNT. A continuation needs
   MIN_BOOK_PLAYERS other players behind it before "the popular move" means anything.
   Past ply 12, 96% of positions in the archive were reached by exactly one game, so
   the denominator is small by nature -- that is the measurement working, not failing.

3. BASELINE IS PER RATING BAND, and the comparison that matters is WITHIN an account:
   its book-following past the opening against its own book-following inside it.
   Strong players play popular moves because popular moves are good; banding removes
   most of that, and the within-account ratio removes the rest of the "this person
   just knows theory" objection -- somebody reading the explorer mid-game should be
   unusually booked DEEP relative to how booked they are SHALLOW.
"""
from __future__ import annotations

import argparse
import collections
import json
import math
import os
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mirror_scan as MS  # noqa: E402

OPENING_PLY = 8        # "outside the opening" starts here; book below this is common knowledge
MIN_BOOK_PLAYERS = 3   # other distinct players needed before a position HAS a book
MIN_COVERED = 40       # an account needs this many judgeable moves to be scored at all
MAX_FALSE = 1.0        # tolerated EXPECTED number of false flags across the whole scan
ALPHA = MAX_FALSE / 100.0   # kept for callers that read it; see MAX_FALSE at the cut


def mover_of(p):
    """Which SEAT played the move being judged at table index `p`. 0 = White.

    The move judged from position `hashes[p]` is `history[p + 1]`, and White moves at
    EVEN history indices -- verified on the archive: over 4,000 games, index%2==0 is a
    `w` piece 23,888 times against 12, and index%2==1 is a `b` piece 23,792 against 64.
    So the mover is White exactly when `p + 1` is even, i.e. when p is ODD.

    This was written as `w if p % 2 == 0 else b`, which is the opposite, and every
    judged move was credited to the player who did not make it. Nothing crashed and
    nothing looked wrong: the output was a complete, plausible, correctly-formatted
    table about the wrong people. A single named function now decides it, in one place,
    so the two call sites cannot drift apart again.
    """
    return (p + 1) % 2


MIN_BAND_N = 8         # accounts needed before a band median means anything
BAND_WIDTHS = (200, 400, 600)


def band_stats(rates, rt, width, min_n=MIN_BAND_N, aligned=False):
    """(median, sd, label) over accounts within `width` of `rt`, or None if too few.

    `rates` is [(median_rating, deep_rate)] over every scored account.

    `aligned` snaps the window to a multiple of `width`, which is what the printed
    summary table wants -- fixed, shared, reproducible bands. The per-account baseline
    uses the default CENTRED window instead. Aligned bands put an account at 1599 in
    1400-1599 and an account at 1601 in 1600-1799, which is arbitrary, and worse: when
    the fallback widens an aligned band it grows in one direction, so a sparse account
    sitting near an edge never reaches the peers a few points across it. That is how
    the first version of this fix behaved -- 1520 widened to 1200-1599 and still found
    two peers while ten sat at 1750.
    """
    if rt is None:
        return None
    if aligned:
        lo = int(rt // width) * width
        hi, label = lo + width, f"{lo:.0f}-{lo + width - 1:.0f}"
    else:
        lo, hi = rt - width / 2.0, rt + width / 2.0
        label = f"{rt:.0f}±{width // 2}"
    v = [x for r, x in rates if r is not None and lo <= r < hi]
    if len(v) < min_n:
        return None
    return statistics.median(v), statistics.stdev(v), label


def baseline_for(rates, rt, min_n=MIN_BAND_N):
    """Widen the band until it has enough accounts. None means UNSCOREABLE.

    The global median used to be the fallback, silently. A 400-rated account with four
    peers on the site was then compared against the whole-site median, and the site's
    median is dominated by the crowded middle bands -- so the comparison measured "is
    this player unlike the average player", which for anyone at the edge of the rating
    distribution is true by construction and has nothing to do with the explorer. Both
    tails were scored against a baseline that does not describe them, and the tails are
    exactly where a small `n` puts people. Widening first, then refusing, keeps the
    account visible without letting it be flagged on a comparison that was never valid.
    """
    for width in BAND_WIDTHS:
        b = band_stats(rates, rt, width, min_n)
        if b:
            return b
    return None


def load(path, limit=0):
    """(games, per-position move table). Two passes so the table stays small."""
    rows = []
    with open(path, encoding="utf-8") as fh:
        for i, line in enumerate(fh):
            if limit and i >= limit:
                break
            try:
                g = json.loads(line)
            except json.JSONDecodeError:
                continue
            h, hist = g.get("hashes") or [], g.get("history") or []
            # same ply-integrity rule as mirror_scan.load(): a compacted hash array
            # would silently shift every ply, and ply is what "past the opening" means
            if len(h) != len(hist) or len(h) < OPENING_PLY + 2:
                continue
            w, b = g.get("white_player") or {}, g.get("black_player") or {}
            rows.append((h, hist, w, b, g.get("white_rating"), g.get("black_rating"),
                         bool(w.get("bot")), bool(b.get("bot"))))
    # pass 1: which positions were reached often enough to have a book at all
    seen = collections.Counter()
    for h, hist, *_ in rows:
        for p in range(len(h) - 1):
            seen[h[p]] += 1
    hot = {x for x, c in seen.items() if c > MIN_BOOK_PLAYERS}
    # pass 2: for those positions only, the continuation counts and each player's own
    # share of them. Storing the SHARE is what makes leave-one-player-out cheap: the
    # naive version rescanned every entry for every player and went quadratic on
    # popular positions -- 10 minutes without finishing on this archive. The number of
    # DISTINCT continuations from a position is small (it is the branching factor), so
    # subtracting a player's own contribution and re-taking the argmax is O(few).
    table = {}                                 # hash -> (Counter move->n, {uid: Counter})
    for h, hist, w, b, wr, br, wb, bb in rows:
        for p in range(len(h) - 1):
            key = h[p]
            if key not in hot:
                continue
            uid_side = mover_of(p)
            mover = w if uid_side == 0 else b
            uid = mover.get("uid")
            if not uid:
                continue
            slot = table.get(key)
            if slot is None:
                slot = table[key] = (collections.Counter(), {})
            mv = tuple(hist[p + 1])
            slot[0][mv] += 1
            slot[1].setdefault(uid, collections.Counter())[mv] += 1
    return rows, table


def book_move(slot, exclude_uid):
    """The most-played continuation, over everyone EXCEPT `exclude_uid`.

    A book that contains your own games is a mirror, not a book: without this an
    account with a pet line "follows" the book its own earlier games created, and the
    heaviest repertoire players come out top. Same trap `_repertoire` exists for in
    mirror_scan.py.
    """
    counts, per_player = slot
    mine = per_player.get(exclude_uid)
    others = len(per_player) - (1 if mine else 0)
    if others < MIN_BOOK_PLAYERS:
        return None
    if not mine:
        return counts.most_common(1)[0][0], others
    best, best_n = None, -1
    for mv, n in counts.items():
        n -= mine.get(mv, 0)
        if n > best_n:
            best, best_n = mv, n
    return (best, others) if best_n > 0 else None


def main():
    MS._utf8_stdout()
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", required=True)
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--min-covered", type=int, default=MIN_COVERED)
    ap.add_argument("--top", type=int, default=25)
    args = ap.parse_args()

    rows, table = load(args.archive, args.limit)
    print(f"{len(rows):,} games with usable ply indexing; "
          f"{len(table):,} positions have enough traffic to carry a book")

    # deep = past the opening, shallow = inside it. Both per account.
    deep = collections.defaultdict(lambda: [0, 0])       # uid -> [hits, covered]
    shallow = collections.defaultdict(lambda: [0, 0])
    ratings = collections.defaultdict(list)
    names, isbot = {}, {}
    for h, hist, w, b, wr, br, wb, bb in rows:
        for p in range(len(h) - 1):
            slot = table.get(h[p])
            if slot is None:
                continue
            mover, rt = (w, wr) if mover_of(p) == 0 else (b, br)
            uid = mover.get("uid")
            if not uid or mover.get("bot"):
                continue                       # bots have no explorer to read
            names[uid] = str(mover.get("username"))
            isbot[uid] = bool(mover.get("bot"))
            bm = book_move(slot, uid)
            if bm is None:
                continue
            bucket = deep if p >= OPENING_PLY else shallow
            bucket[uid][1] += 1
            if tuple(hist[p + 1]) == bm[0]:
                bucket[uid][0] += 1
            if rt:
                ratings[uid].append(rt)

    scored = [(u, d) for u, d in deep.items() if d[1] >= args.min_covered]
    print(f"{len(scored):,} account(s) have >= {args.min_covered} judgeable moves "
          f"past ply {OPENING_PLY}\n")
    if not scored:
        return

    rate_of = lambda d: 100.0 * d[0] / d[1]
    med_rating = {u: (statistics.median(ratings[u]) if ratings.get(u) else None)
                  for u, _ in scored}
    rates = [(med_rating[u], rate_of(d)) for u, d in scored]

    print("book-following past the opening, by rating band:")
    seen_bands = {}
    for u, d in scored:
        b = band_stats(rates, med_rating[u], 200, aligned=True)
        if b:
            lo = int(b[2].split("-")[0])
            seen_bands[b[2]] = (b[0], b[1],
                                sum(1 for r, _ in rates if r is not None
                                    and lo <= r < lo + 200))
    for lbl in sorted(seen_bands, key=lambda s: int(s.split("-")[0])):
        m, sd, n = seen_bands[lbl]
        print(f"   {lbl:<12} n={n:>4}  median {m:5.1f}%  sd {sd:4.1f}")

    out, unscoreable = [], []
    for u, d in scored:
        rate = rate_of(d)
        rt = med_rating[u]
        b = baseline_for(rates, rt)
        sh = shallow.get(u, [0, 0])
        sh_rate = (100.0 * sh[0] / sh[1]) if sh[1] >= 10 else None
        if b is None:
            unscoreable.append((rate, d, sh_rate, sh[1], rt, u))
            continue
        base, sd, lbl = b
        se = math.sqrt((rate / 100 * (1 - rate / 100)) / d[1]) * 100
        z = (rate - base) / math.sqrt(se ** 2 + sd ** 2)
        out.append((z, rate, base, d, sh_rate, sh[1], rt, u))
    out.sort(reverse=True)
    if unscoreable:
        print(f"\n{len(unscoreable)} account(s) have NO comparable rating band even at "
              f"600 points wide — no z, and they cannot be flagged:")
        for rate, d, shr, shn, rt, u in unscoreable:
            nm = names.get(u, u)
            print(f"   {nm[:22]:<22} {rate:5.1f}% over {d[1]:>5} moves, "
                  f"rating {(f'{rt:.0f}' if rt else '-')}")

    # multiplicity: N accounts tested, so the cut has to hold E[false flags] < MAX_FALSE.
    #
    # This was written `>= ALPHA * 100`, which with ALPHA = 0.01 is 1.0. The arithmetic was
    # right and the printed line said "< 1", but the CONSTANT it derived from is documented
    # as "tolerated probability of one false flag across the whole scan" = 0.01 — a
    # hundredfold difference between what the constant claims and what the cut enforces. A
    # reader tightening ALPHA to 0.001 would expect a much stricter cut and get a
    # proportional one, which is fine, but a reader auditing the threshold reads 0.01 and
    # concludes the tool tolerates one false accusation per hundred scans. It tolerates one
    # per scan. Named for what it is instead.
    n = len(out)
    tail = lambda z: 0.5 * math.erfc(z / math.sqrt(2))
    zcut = 3.0
    while n * tail(zcut) >= MAX_FALSE:
        zcut += 0.05
    print(f"\n{n} accounts tested; cut for expected false flags < {MAX_FALSE:g}: "
          f"z >= {zcut:.2f}")
    print(f"\n{'account':<22} {'z':>6} {'deep':>7} {'band':>6} {'covered':>8} "
          f"{'shallow':>8} {'n':>6} {'rating':>7}")
    print("-" * 78)
    for z, rate, base, d, shr, shn, rt, u in out[:args.top]:
        nm = names.get(u, u)
        nm = ("deleted:" + u[:8]) if nm.startswith("deleted_user_") else nm
        print(f"{nm[:22]:<22} {z:>+6.1f} {rate:>6.1f}% {base:>5.1f}% {d[1]:>8} "
              f"{(f'{shr:.1f}%' if shr is not None else '-'):>8} {shn:>6} "
              f"{(f'{rt:.0f}' if rt else '-'):>7}")

    flagged = [x for x in out if x[0] >= zcut]
    print(f"\n{len(flagged)} account(s) over the multiplicity-corrected cut")
    for z, rate, base, d, shr, shn, rt, u in flagged:
        nm = names.get(u, u)
        nm = ("deleted:" + u[:8]) if nm.startswith("deleted_user_") else nm
        extra = ""
        if shr is not None:
            extra = (f"; it follows book {rate - shr:+.1f}pp MORE past the opening "
                     f"than inside it")
        print(f"   {nm}: {rate:.1f}% vs band {base:.1f}% over {d[1]} moves{extra}")

    print("\nWHAT THIS IS NOT")
    print("  - not evidence. People play from memory, and strong players converge on")
    print("    popular moves because popular moves are good.")
    print("  - the deep-vs-shallow column is the one to read: somebody consulting the")
    print("    explorer mid-game should be unusually booked PAST the opening relative")
    print("    to how booked they are INSIDE it. A player who is simply well prepared")
    print("    is booked in both.")
    print("  - the book excludes each account's own games, so a pet line cannot make")
    print("    an account look like it is following anybody.")
    print("  - nothing here reaches the registry. It is a mark for a human to weigh")
    print("    next to the position evidence, which is the part that has a null.")


if __name__ == "__main__":
    main()
