#!/usr/bin/env python3
"""The fair-play registry: the durable record of what we believe about each account.

Three labels, and only three:

    normal          reviewed by a human, no action -- an explicit clearance, not a default
    suspicious      evidence exists; a human has not yet concluded
    proven_cheater  a human concluded, with a reason on the record

plus `unreviewed`, which is what every account is until somebody looks.

Two rules the code enforces, not the operator:

  1. Automation may only ever set `suspicious`, and only by attaching evidence.
     `normal` and `proven_cheater` require a named human and a written reason.
     A detector cannot label anyone, which is the whole ethic of the system and
     also the cleanest posture under GDPR Art. 22 (no solely-automated decisions
     with a significant effect).

  2. Every transition is appended to an immutable log. Labels can change; history
     cannot. The log is what lets us measure the detector later -- each human
     decision is a ground-truth label, and accumulating them is the only path to
     a supervised classifier for off-site engine use.

    py -3 tools/cheat/registry.py show <uid>
    py -3 tools/cheat/registry.py list --label suspicious
    py -3 tools/cheat/registry.py label <uid> proven_cheater --by a_moderator --why "..."
    py -3 tools/cheat/registry.py export-labels        # training set, uids + labels
"""
from __future__ import annotations

import argparse
import json
import os
from datetime import datetime, timezone

LABELS = ("unreviewed", "suspicious", "normal", "proven_cheater")
HUMAN_ONLY = ("normal", "proven_cheater")
DEFAULT_PATH = os.path.join("logs", "cheat", "registry.json")


def _human(by, why):
    """Normalise and validate a (reviewer, reason) pair. Returns (by, why) trimmed.

    Not a security boundary -- anyone with CLI or file access can write any name, and no
    string check changes that. It is a hygiene gate on the AUDIT LOG, which export-labels
    treats as ground truth. `by = "detector"` is refused because that literal is the one
    the automated path stamps (see _log call sites), and a whitespace-only `by` or `why`
    used to pass `not by` / `len(why) >= 20` and write a nameless, reasonless decision.
    Trim first, then check emptiness, so " " and "\\t" cannot masquerade as a name.
    """
    by = (by or "").strip()
    why = (why or "").strip()
    if not by or by.casefold() == "detector":
        raise SystemExit("this action requires a named human reviewer (--by)")
    if len(why) < 20:
        raise SystemExit("this action requires a written reason of >=20 characters")
    return by, why

# Signals written before `code` existed carry only the sentence. Without this map the
# FIRST RUN after that change reproduces the exact bug the change was made to fix: the
# stored sentence does not equal the new slug, so every finding already on file is minted
# as new, `needs_recheck` goes true on every human-cleared account, and the recheck queue
# refills with findings volunteers had already dismissed. Deploying the fix would trip it.
# Measured before adding this: 1 signal -> 2, and a `normal` clearance invalidated, from a
# rescan that found nothing new.
#
# Only these exact historical sentences are mapped. Anything else keeps the old
# sentence-as-identity behaviour, which is no worse than before. NEW signals must always
# pass a `code`; this map is not a place to add entries, it is a record of what was
# written before codes existed.
LEGACY_CODES = {
    "mirror: same account in both games": "mirror.self",
    "mirror: ran the oracle game of an unattributable pair": "mirror.oracle_side",
    "mirror: seat correspondence says this seat received the moves "
    "(inference — account linkage needs the database)": "mirror.seat_inference",
    "linked accounts: a concurrent bot game run by another account reproduces this "
    "account's line (no human opponent in either game)": "linked.bot_games",
}


def _now():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


class Registry:
    def __init__(self, path=DEFAULT_PATH):
        self.path = path
        self.data = {"accounts": {}, "log": []}
        if os.path.exists(path):
            with open(path, encoding="utf-8") as fh:
                self.data = json.load(fh)

    def save(self):
        os.makedirs(os.path.dirname(self.path) or ".", exist_ok=True)
        tmp = self.path + ".tmp"
        with open(tmp, "w", encoding="utf-8") as fh:
            json.dump(self.data, fh, indent=1, ensure_ascii=False)
        os.replace(tmp, self.path)

    # -- accounts ----------------------------------------------------------
    def get(self, uid):
        a = self.data["accounts"].setdefault(uid, {
            "uid": uid, "name": None, "label": "unreviewed",
            "first_seen": _now(), "last_seen": _now(),
            "signals": [], "evidence_games": [], "bundles": [],
            "needs_recheck": False,
        })
        a.setdefault("needs_recheck", False)       # migrate older registries
        for s in a["signals"]:                     # ... and stamp pre-`code` signals
            if not s.get("code"):
                known = LEGACY_CODES.get(s.get("signal"))
                if known:
                    s["code"] = known
        return a

    def add_evidence(self, uid, *, name=None, signal, detail, games=(), bundle=None,
                     code=None):
        """Automation's only write path. Raises to `suspicious`, never further.

        `code` is a short stable slug naming the KIND of finding, and it -- with
        `detail` -- is the de-duplication identity. `signal` is the sentence a human
        reads and may be reworded freely.

        Splitting the two matters more than it looks. The identity used to BE the
        sentence, and the sentences are long explanatory prose written for moderators
        ("mirror: seat correspondence says this seat received the moves (inference —
        account linkage needs the database)"). Editing a word of it -- fixing a typo,
        softening a phrase after a complaint, swapping an em dash -- silently minted a
        NEW signal for evidence the registry had already recorded. The branch below
        keys off exactly that: a new signal means `is_new`, and `is_new` on a
        human-labelled account sets `needs_recheck` and appends "new evidence arrived
        after human review". So a copy-edit invalidated every volunteer's clearance at
        once and refilled the recheck queue with findings they had already dismissed.
        A proof-reading pass is not new evidence.
        """
        a = self.get(uid)
        if name:
            a["name"] = name
        now = _now()
        a["last_seen"] = now
        ident = signal if code is None else code

        # De-duplicate on the evidence's IDENTITY, not on a record carrying a fresh
        # timestamp. The old membership test compared whole dicts including
        # `"at": _now()`, so two scans in the same second collapsed and two scans on
        # different days never did: identical evidence re-accumulated once per run,
        # and `list` both ranked accounts by len(signals) and printed it as the
        # evidence count. The registry's ordering was a function of how often the
        # operator ran the tool.
        existing = next((s for s in a["signals"] if s.get("code", s.get("signal"))
                         == ident and s.get("detail") == detail), None)
        if existing:
            existing["last_at"] = now
            existing["count"] = existing.get("count", 1) + 1
            existing["signal"] = signal        # keep the freshest wording on display
            existing["code"] = ident
        else:
            a["signals"].append({"code": ident, "signal": signal, "detail": detail,
                                 "first_at": now, "last_at": now, "count": 1})

        new_games = [g for g in games if g not in a["evidence_games"]]
        a["evidence_games"].extend(new_games)
        if bundle and bundle not in a["bundles"]:
            a["bundles"].append(bundle)

        # Is this actually NEW, or is it last week's scan finding the same thing again?
        # Everything below turns on the difference and nothing used to.
        is_new = existing is None or bool(new_games)

        if a["label"] == "unreviewed":
            a["label"] = "suspicious"
            self._log(uid, "unreviewed", "suspicious", "detector", signal)
        elif a["label"] in HUMAN_ONLY:
            # A human label is NEVER overwritten by automation. That rule is the whole
            # ethic of this file and it stays. But the evidence still has to be
            # recorded: this branch used to do nothing at all, so once a volunteer
            # cleared an account as `normal`, every later detection -- including a
            # whole-game mirror -- was appended to `signals` and then silently
            # swallowed. No log entry, despite the promise that every transition is
            # logged; absent from `list --label suspicious`, the only surface a
            # moderator uses to find work; and still exported by export-labels as a
            # negative example.
            #
            # Only NEW evidence resurfaces it, though. `needs_recheck` means "a human
            # decided, and then something arrived that they have not seen". Re-running
            # the scan is not something arriving.
            if is_new:
                a["needs_recheck"] = True
                self._log(uid, a["label"], a["label"], "detector",
                          f"new evidence arrived after human review: {signal}")
        elif is_new:
            # Already `suspicious` -- which is automation's OWN label, not a human's.
            # This branch used to fall in with the human ones, so every rescan set
            # needs_recheck on every suspicious account and appended a log entry
            # claiming "new evidence arrived after human review" when no human had
            # reviewed anything and no evidence was new. Two scans were enough to put
            # the entire suspicious list into `list --recheck`, which is the one
            # surface that is supposed to mean "a volunteer must look at this again".
            # A detector that marks everything urgent has not prioritised anything.
            self._log(uid, "suspicious", "suspicious", "detector",
                      f"further evidence: {signal}")
        return a

    def set_label(self, uid, label, *, by, why):
        if label not in LABELS:
            raise SystemExit(f"label must be one of {LABELS}")
        if label in HUMAN_ONLY:
            by, why = _human(by, why)
        a = self.get(uid)
        old = a["label"]
        a["label"] = label
        a["needs_recheck"] = False      # a human has now looked at what arrived
        self._log(uid, old, label, by, why)
        return a

    def dismiss(self, uid, *, code, detail, by, why):
        """A named human turns one finding off.

        This is the per-finding half of the operator's control surface. `label normal`
        clears the whole account; this clears one signal — "we looked at this specific
        pair of games and it is not what the detector thought" — while the account and
        its other findings stay live.

        Three properties, all load-bearing:
          * HUMAN-ONLY, same discipline as set_label: a name that is not "detector" and
            a written reason of >= 20 characters. Automation cannot dismiss its own
            output, or the false-positive rate becomes unmeasurable.
          * A rescan does not resurrect it. The de-dup identity `(code, detail)` means
            the weekly scan re-finding the same pair updates `count` on the SAME record
            and `is_new` stays false, so a dismissal survives every future run without
            any suppression list.
          * It hides nothing new. A different pair of games is a different `detail`,
            which is a new signal, which surfaces normally — dismissing one finding
            never mutes the next one.
        Logged like every other decision; the log is append-only.
        """
        by, why = _human(by, why)
        if uid not in self.data["accounts"]:
            raise SystemExit(f"{uid} is not in the registry — check the uid against "
                             f"`registry.py list`")
        a = self.get(uid)
        sig = next((s for s in a["signals"]
                    if s.get("code", s.get("signal")) == code
                    and s.get("detail") == detail), None)
        if sig is None:
            have = ", ".join(sorted({s.get("code", "?") for s in a["signals"]})) or "none"
            raise SystemExit(f"no finding ({code}, {detail!r}) on {uid} — dismissing a "
                             f"finding that does not exist would only create the "
                             f"illusion of review. Signals on file: {have}")
        if sig.get("dismissed"):
            raise SystemExit(f"already dismissed by {sig['dismissed']['by']} at "
                             f"{sig['dismissed']['at']} — the log has the reason")
        sig["dismissed"] = {"by": by, "why": why, "at": _now()}
        self._log(uid, a["label"], a["label"], by,
                  f"dismissed finding {code} [{detail}]: {why}")
        return a

    def _log(self, uid, old, new, by, why):
        self.data["log"].append({"at": _now(), "uid": uid, "from": old,
                                 "to": new, "by": by, "why": why})


# -- CLI -------------------------------------------------------------------
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--registry", default=DEFAULT_PATH)
    sub = ap.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("show"); s.add_argument("uid")
    l = sub.add_parser("list"); l.add_argument("--label", choices=LABELS)
    l.add_argument("--recheck", action="store_true",
                   help="only accounts where new evidence arrived after a human decided")
    b = sub.add_parser("label")
    b.add_argument("uid"); b.add_argument("label", choices=LABELS)
    b.add_argument("--by", required=True); b.add_argument("--why", required=True)
    d = sub.add_parser("dismiss",
                       help="turn ONE finding off: a human marks a specific "
                            "(code, detail) signal as reviewed-and-not-cheating; it "
                            "never resurfaces on rescans, other findings are untouched")
    d.add_argument("uid")
    d.add_argument("--code", required=True,
                   help="the finding's code, e.g. mirror.self (see `show`)")
    d.add_argument("--detail", required=True,
                   help="the finding's detail, e.g. 'gidA + gidB' (see `show`)")
    d.add_argument("--by", required=True); d.add_argument("--why", required=True)
    sub.add_parser("export-labels")
    sub.add_parser("audit")

    args = ap.parse_args()
    r = Registry(args.registry)

    if args.cmd == "show":
        if args.uid not in r.data["accounts"]:
            print("not in registry"); return
        # via get(), not a raw dict read: get() runs the LEGACY_CODES migration, so a
        # pre-`code` signal displays the `code` that `dismiss --code` actually expects.
        # Reading the dict directly printed the bare sentence, and the operator then
        # could not dismiss the finding the tool told them to dismiss.
        a = r.get(args.uid)
        print(json.dumps(a, indent=1))
        print("\nhistory:")
        for e in r.data["log"]:
            if e["uid"] == args.uid:
                print(f"  {e['at']}  {e['from']} -> {e['to']}  by {e['by']}: {e['why']}")

    elif args.cmd == "list":
        rows = [a for a in r.data["accounts"].values()
                if (not args.label or a["label"] == args.label)
                and (not args.recheck or a.get("needs_recheck"))]
        # Rank by how much distinct LIVE evidence exists, not by how many times the
        # tool was run. `signals` is deduplicated on identity; a signal a human
        # dismissed no longer counts toward an account's place in the queue, or a
        # reviewed-and-cleared finding would keep the account at the top forever.
        def live(a):
            return sum(1 for s in a["signals"] if not s.get("dismissed"))
        rows.sort(key=lambda a: (-live(a), -len(a["evidence_games"])))
        print(f"{'label':15s} {'live':>5s} {'dism':>5s} {'games':>6s} "
              f"{'recheck':>7s}  account")
        for a in rows:
            nm = a["name"] or a["uid"][:12]
            print(f"{a['label']:15s} {live(a):5d} "
                  f"{len(a['signals']) - live(a):5d} "
                  f"{len(a['evidence_games']):6d} "
                  f"{'YES' if a.get('needs_recheck') else '-':>7s}  {nm}  [{a['uid']}]")
        print(f"\n{len(rows)} account(s)")
        stale = sum(1 for a in r.data["accounts"].values() if a.get("needs_recheck"))
        if stale and not args.recheck:
            print(f"{stale} account(s) have new evidence since a human decided — "
                  f"`list --recheck`")

    elif args.cmd == "label":
        # A uid is 32 hex characters, transcribed by hand off a case filename. Without
        # this check a single mistyped character created a brand-new account row and
        # labelled *it* proven_cheater: Registry.get() inserts on read, set_label()
        # then wrote the label and the audit-log entry, and `list` showed a
        # proven_cheater with no evidence and no games while the account the reviewer
        # meant to label stayed `suspicious` in the queue.
        if args.uid not in r.data["accounts"]:
            raise SystemExit(
                f"{args.uid} is not in the registry — nothing has ever been recorded "
                f"about it. Labelling it would create a new account row from a typo. "
                f"Check the uid against the case filename, or run the scan with "
                f"--registry first.")
        a = r.set_label(args.uid, args.label, by=args.by, why=args.why)
        r.save()
        print(f"{args.uid} -> {a['label']}")

    elif args.cmd == "dismiss":
        a = r.dismiss(args.uid, code=args.code, detail=args.detail,
                      by=args.by, why=args.why)
        r.save()
        live = sum(1 for s in a["signals"] if not s.get("dismissed"))
        print(f"dismissed ({args.code}, {args.detail}) on {args.uid} — "
              f"{live} live finding(s) remain")
        if live == 0 and a["label"] == "suspicious":
            print("every finding on this account is now dismissed; if the account "
                  "itself is cleared, say so explicitly:\n"
                  f"  registry.py label {args.uid} normal --by {args.by} --why ...")

    elif args.cmd == "export-labels":
        # The ground-truth set for any future supervised model. An account whose
        # label predates evidence that arrived afterwards is NOT ground truth -- it
        # is a stale human decision, and exporting it as a clean negative teaches a
        # classifier that a confirmed mirror is normal. Flagged, never silently kept.
        out = [{"uid": a["uid"], "label": a["label"],
                "stale": bool(a.get("needs_recheck"))}
               for a in r.data["accounts"].values()
               if a["label"] in ("normal", "proven_cheater")]
        print(json.dumps(out, indent=1))
        stale = sum(1 for x in out if x["stale"])
        print(f"\n# {len(out)} human-decided labels "
              f"({sum(1 for x in out if x['label'] == 'proven_cheater')} positive, "
              f"{stale} stale — re-review before training on them)", flush=True)

    elif args.cmd == "audit":
        for e in r.data["log"]:
            print(f"{e['at']}  {e['uid'][:12]}  {e['from']:>12s} -> {e['to']:<15s} "
                  f"by {e['by']}: {e['why']}")
        print(f"\n{len(r.data['log'])} transition(s)")


if __name__ == "__main__":
    main()
