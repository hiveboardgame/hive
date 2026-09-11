#!/usr/bin/env python3
"""Rank every account the fair-play signals implicate, with the reason for each.

Signals, in descending evidential strength:

  S1 SELF-MIRROR   the account is in BOTH games of a matched pair -- a rated game
                   against a human and a concurrent game against a bot carrying the
                   same positions. Implicates nobody but the account itself.
  S2 ORACLE SHAPE  many games, ZERO human opponents ever. Not wrong on its own
                   (people do play only bots) but it is what a relay terminal looks
                   like, and it is computable without any position matching.
  S3 LINKED        the account sat in the ORACLE game -- the bot game -- of a matched
                   pair whose real game is on a DIFFERENT account. It demonstrably ran
                   the oracle, and it is NOT the real game's player.
  S4 SEAT          the real-game seat that `seat_correspondence` resolves for such a
                   pair: the bot's colour in the oracle pins which of the two seats
                   received the moves, and positively EXCLUDES the other. An inference
                   about a SEAT, which is an account, which is not a person -- linking
                   it needs the database. Where the seat does not resolve, or two
                   oracles resolve it two ways, neither real-game player is named.
  S5 ELO SPLIT     rating gain concentrated in flagged games vs the account's own
                   baseline. Weak, and confounded -- see the caveats in the review
                   file. The detector never saw results, but selection still
                   correlates with outcome through rating deviation and game length.

Victims -- accounts that only ever appear as somebody's opponent -- are excluded by
construction and counted at the end.

ATTRIBUTION IS NOT RE-DERIVED HERE. This file used to compute it inline: an oracle-human
set spelled out by hand, `subj = rp & oh`, and no call to `seat_correspondence` at all.
The consequences were exactly what `subject_of`'s docstring warns about. The hand-spelled
set was a second copy of a rule whose two versions have already produced accusation files
about victims twice; and skipping the seat inference meant this tool reported 5 pairs for
the account `mirror_scan.py` attributes 25 to, while printing that the real game's players
"are NOT attributable and are not named" -- directly contradicting the case file
`mirror_scan.py` had just written naming one of them. Two entry points, two answers, one
account. Call `MS.subject_of`, `MS.oracle_humans` and `MS.seat_correspondence`; do not
re-implement them.

Note on `rated`: the site's own bot button creates UNRATED games
(apis/src/pages/challenge_bot.rs:48, "Play an unrated game vs our bot"). Counting only
rated games therefore made S2 -- the one signal that needs no position matching --
unable to fire for exactly the accounts it exists to find. Every game counts toward
the shape; only the rating columns are restricted to rated games.
"""
from __future__ import annotations
import argparse, collections, json
import mirror_scan as MS


def main():
    MS._utf8_stdout()
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", required=True)
    ap.add_argument("--min-oracle-games", type=int, default=20)
    ap.add_argument("--alpha", type=float, default=MS.ALPHA)
    ap.add_argument("--enable-seat", action="store_true",
                    help="turn the seat inference ON (default OFF), exactly as "
                         "mirror_scan --enable-seat does. The two entry points must "
                         "agree: with it off, alt-account real-game seats are not named "
                         "here either.")
    args = ap.parse_args()

    if args.enable_seat:
        MS.SEAT_ENABLED = True
    print("config — seat inference "
          + ("ON" if MS.SEAT_ENABLED else "OFF (default)"))

    raw_by_uid = collections.defaultdict(list)
    names = {}
    with open(args.archive, encoding="utf-8") as fh:
        for line in fh:
            try:
                g = json.loads(line)
            except json.JSONDecodeError:
                continue
            w, b = g.get("white_player") or {}, g.get("black_player") or {}
            for me, other, rc in ((w, b, g.get("white_rating_change")),
                                  (b, w, g.get("black_rating_change"))):
                u = me.get("uid")
                if not u:
                    continue
                names[u] = str(me.get("username"))
                raw_by_uid[u].append({
                    "gid": g.get("game_id"), "rc": rc,
                    "rated": bool(g.get("rated")),
                    "opp_bot": bool(other.get("bot")),
                    "me_bot": bool(me.get("bot")),
                    "tour": bool(g.get("tournament")),
                })

    games, rejected = MS.load(args.archive)
    for k, v in rejected.most_common():
        # A quarantined game is still in the corpus; only a dropped one is not.
        if k.startswith(MS.QUARANTINE):
            print(f"kept, timing untrusted — {k[len(MS.QUARANTINE):]}: {v:,}")
        else:
            print(f"dropped — {k}: {v:,}")
    index = MS.build_index(games)
    survivors, _ = MS.find_pairs(games, index, args.alpha)

    self_mirror = collections.Counter()
    linked = collections.Counter()
    seated = collections.Counter()
    flagged_games = collections.defaultdict(set)
    tiers = collections.defaultdict(list)
    victims = set()

    for p in survivors:
        real, orac = MS.classify(p)
        if real is None:
            continue
        rp = {real["w"], real["b"]} - {None}
        subj = MS.subject_of(real, orac)
        tier, _ = MS.grade(p)
        if subj:
            for u in subj:
                self_mirror[u] += 1
                flagged_games[u].add(real["gid"])
                tiers[u].append(tier)
            victims |= (rp - subj)
        else:
            # UNATTRIBUTED pair: no account sits in both games. The human in the bot
            # game demonstrably ran the oracle, so it is recorded.
            for u in MS.oracle_humans(orac):
                linked[u] += 1
                tiers[u].append(tier)
            # The bot's colour then resolves WHICH real-game seat received the moves.
            # The seat it clears stays a victim and gets nothing. This mirrors
            # mirror_scan.main() exactly, and must keep mirroring it: the two used to
            # disagree, and the account mirror_scan attributed most of its pairs to
            # showed up here with only a handful.
            seat = MS.seat_correspondence(p)
            if seat:
                recv = seat[0]
                if recv:
                    seated[recv] += 1
                    flagged_games[recv].add(real["gid"])
                    tiers[recv].append(tier)
                victims |= (rp - {recv})
            else:
                # Nothing resolved, or two oracles resolved the seat two ways. Both
                # real-game players are withheld, which is the posture this file had
                # before seat correspondence existed.
                victims |= rp

    # oracle-shape scan across the WHOLE corpus, independent of position matching
    oracle = {}
    for u, rows in raw_by_uid.items():
        if any(r["me_bot"] for r in rows):
            continue                                   # declared bot: not a suspect
        n = len(rows)
        humans = sum(1 for r in rows if not r["opp_bot"])
        if n >= args.min_oracle_games and humans == 0:
            oracle[u] = n

    # "plays only bots" is NOT suspicious on its own -- most such accounts are
    # beginners practising. It only carries weight together with a matched pair.
    bot_only_benign = {u: n for u, n in oracle.items()
                       if not self_mirror.get(u) and not linked.get(u)
                       and not seated.get(u)}
    suspects = (set(self_mirror) | set(linked) | set(seated)
                | (set(oracle) - set(bot_only_benign)))
    suspects = {u for u in suspects if not any(r["me_bot"] for r in raw_by_uid.get(u, []))}

    rows = []
    for u in suspects:
        rs = raw_by_uid.get(u, [])
        hum = [r for r in rs if not r["opp_bot"]]
        rated = [r for r in hum if r["rated"] and r["rc"] is not None]
        fl = [r for r in rated if r["gid"] in flagged_games[u]]
        un = [r for r in rated if r["gid"] not in flagged_games[u]]
        best = min((MS.TIER_ORDER[t] for t in tiers.get(u, [])), default=3)
        rows.append({
            "uid": u, "name": names.get(u, "?"),
            "self": self_mirror.get(u, 0), "linked": linked.get(u, 0),
            "seat": seated.get(u, 0),
            "oracle_n": oracle.get(u, 0), "tier": MS.TIERS[best],
            "games": len(rs), "humans": len(hum),
            "flagged": len(fl), "tour_flagged": sum(1 for r in fl if r["tour"]),
            "avg_f": (sum(r["rc"] for r in fl) / len(fl)) if fl else None,
            "avg_u": (sum(r["rc"] for r in un) / len(un)) if un else None,
        })

    # Queue order is evidence, not volume. Ranking by pair count put whoever happened
    # to sit in a popular opening line's bot game above a whole-game mirror.
    rows.sort(key=lambda r: (MS.TIER_ORDER[r["tier"]], -r["self"], -r["seat"],
                             -r["linked"]))

    print(f"\n{len(rows)} accounts carry a signal worth reviewing.")
    print(f"  {len(victims - suspects)} account(s) appear only as somebody's opponent — excluded.")
    print(f"  {len(bot_only_benign)} account(s) play only bots but match no pair — "
          f"normal practice behaviour, not listed.\n")
    hdr = (f"{'account':22s} {'tier':>5s} {'S1':>3s} {'S2':>4s} {'S3':>3s} {'S4':>3s} "
           f"{'games':>6s} {'vsHum':>6s} {'flag':>5s} {'tour':>5s} {'Elo/flag':>9s} "
           f"{'Elo/other':>9s}")
    print(hdr); print("-" * len(hdr))
    for r in rows:
        nm = r["name"]
        nm = ("deleted:" + r["uid"][:8]) if nm.startswith("deleted_user_") else nm[:22]
        af = f"{r['avg_f']:+.2f}" if r["avg_f"] is not None else "  -"
        au = f"{r['avg_u']:+.2f}" if r["avg_u"] is not None else "  -"
        print(f"{nm:22s} {r['tier']:>5s} {r['self']:3d} {r['oracle_n']:4d} "
              f"{r['linked']:3d} {r['seat']:3d} "
              f"{r['games']:6d} {r['humans']:6d} {r['flagged']:5d} {r['tour_flagged']:5d} "
              f"{af:>9s} {au:>9s}")

    print("\nREASONS")
    for r in rows:
        nm = ("deleted:" + r["uid"][:12]) if r["name"].startswith("deleted_user_") else r["name"]
        why = []
        if r["self"]:
            why.append(f"in BOTH games of {r['self']} matched pair(s) — self-attributable")
        if r["oracle_n"]:
            why.append(f"{r['oracle_n']} games, ZERO human opponents — relay-terminal shape")
        if r["linked"] and not r["self"]:
            why.append(f"sat in the bot game of {r['linked']} matched pair(s) whose real "
                       f"game is on another account — it ran the oracle, and it is not "
                       f"the real game's player")
        if r["seat"]:
            why.append(f"seat correspondence resolves this account as the seat that "
                       f"RECEIVED the moves in {r['seat']} pair(s) whose oracle is on "
                       f"another account — an inference from the bot's colour, naming a "
                       f"SEAT and not a person; linking it needs the database")
        if r["tour_flagged"]:
            why.append(f"{r['tour_flagged']} flagged game(s) were TOURNAMENT games")
        if r["avg_f"] is not None and r["avg_u"] is not None and r["avg_f"] - r["avg_u"] > 5:
            why.append(f"gains {r['avg_f']:+.1f} Elo/game in flagged games vs "
                       f"{r['avg_u']:+.1f} elsewhere — WEAK: flagged games are clustered "
                       f"in time, and Glicko-2 deltas scale with rating deviation")
        print(f"\n  {nm}  [{r['uid']}]  ({r['tier']})")
        for x in why:
            print(f"     - {x}")


if __name__ == "__main__":
    main()
