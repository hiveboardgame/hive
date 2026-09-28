"""Pull the full hivegame.com finished-game archive at FULL fidelity.

Unlike tools/attic/hive_archive_to_tsv.py this keeps every field (uids, ratings,
move_times, hashes, timestamps) and never writes the 3-column TSV.

Politeness: one connection, one request at a time, configurable delay, exponential
backoff on 5xx, hard stop after repeated failures.

Pagination: KEYSET, not OFFSET. The endpoint switches on `batch_token`
(db/src/helpers/games_query_builder.rs:52) — absent → OFFSET by `page`, present →
keyset from the token. OFFSET is unstable against a live site: the archive is sorted
newest-first, so a game finishing mid-crawl shifts every later offset boundary and a
row can be skipped entirely (dedup catches duplicates but cannot restore a skip). So
the first request of each stream is OFFSET page 1 (the newest 50, always shallow) and
every request after it passes the server's returned `next_batch` token back as
`batch_token`, exactly as the site's own UI does (apis/.../display_games.rs). The
crawl is still split by (result × speed) — not for offset depth any more, but because
each split is an independent keyset stream and the splits parallelise the newest-first
frontier. A stream ends when the server returns `next_batch = null`.

Resumable: appends JSONL and skips game_ids already present.
"""
from __future__ import annotations
import argparse, json, math, re, time
from pathlib import Path
import requests, cbor2

BASE = "https://hivegame.com"
# No personal contact in a public-repo file. Point at the project instead; a site
# operator who wants to reach the puller's author can do so through the repo.
UA = "hive fair-play research puller (https://github.com/hiveboardgame/hive)"
SPEEDS = ["Bullet", "Blitz", "Rapid", "Classic", "Correspondence", "Untimed"]
RESULTS = [("W", {"ColorWins": "White"}), ("B", {"ColorWins": "Black"}), ("D", "Draw")]
BATCH = 50


def discover(session: requests.Session) -> str:
    page = session.get(BASE + "/archive", timeout=30)
    page.raise_for_status()
    m = re.search(r'"/pkg/([^"]+\.wasm)"', page.text) or \
        re.search(r"wasm_output_name\)\s*\{.*?\"([^\"]+\.wasm)\"", page.text, re.S)
    if not m:
        raise SystemExit("could not locate wasm bundle")
    wasm = session.get(f"{BASE}/pkg/{m.group(1)}", timeout=60).content
    ep = re.search(rb"/api/get_batch_from_options\d+", wasm) or \
         re.search(rb"get_batch_from_options\d+", wasm)
    if not ep:
        raise SystemExit("could not locate archive endpoint in wasm")
    name = ep.group(0).decode()
    return name if name.startswith("/") else "/api/" + name


def options(result_filter, speed, *, batch_token=None, rated=None, exclude_bots=False):
    # `page` stays 1: it only matters in OFFSET mode (batch_token is None), which is
    # just the first request of each stream. Once a token is threaded in, the server
    # ignores `page` and pages by the token.
    return {
        "player1": None, "player2": None, "fixed_colors": False,
        "exclude_bots": exclude_bots, "only_tournament": False,
        "rated": rated, "expansions": True,
        "speeds": [speed],
        "rating_min": None, "rating_max": None,
        "turn_min": None, "turn_max": None,
        "date_start": "1998-01-06T00:00:00Z", "date_end": "2030-01-01T00:00:00Z",
        "result_filter": result_filter,
        "batch_token": batch_token, "batch_size": BATCH, "page": 1,
        "sort": {"key": "Date", "ascending": False},
        "game_progress": "Finished", "include_total": True, "position_hash": None,
    }


def post(session, ep, payload, delay, attempt=0):
    r = session.post(BASE + ep, data=cbor2.dumps(payload), timeout=120)
    if r.status_code == 200:
        return cbor2.loads(r.content), len(r.content)
    if r.status_code in (429, 500, 502, 503, 504) and attempt < 4:
        back = delay * (2 ** attempt) + 2
        print(f"    HTTP {r.status_code} — backing off {back:.0f}s", flush=True)
        time.sleep(back)
        return post(session, ep, payload, delay, attempt + 1)
    raise RuntimeError(f"HTTP {r.status_code}: {r.text[:200]}")


def jdefault(o):
    """CBOR returns UUIDs as raw 16-byte strings; hex them rather than letting
    str() emit a Python bytes repr. uid is the join key for every classifier."""
    if isinstance(o, (bytes, bytearray)):
        return o.hex()
    return str(o)


def load_seen(path: Path) -> set[str]:
    seen = set()
    if path.exists():
        with path.open(encoding="utf-8") as f:
            for line in f:
                try:
                    o = json.loads(line)
                except json.JSONDecodeError:
                    continue
                gid = o.get("game_id") or o.get("uuid")
                if gid:
                    seen.add(str(gid))
    return seen


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="hivegame_archive_full_raw.jsonl")
    ap.add_argument("--delay", type=float, default=1.0, help="seconds between requests")
    ap.add_argument("--rated", choices=["true", "false", "any"], default="any")
    ap.add_argument("--exclude-bots", action="store_true")
    ap.add_argument("--max-requests", type=int, default=0, help="0 = unlimited")
    args = ap.parse_args()

    rated = {"true": True, "false": False, "any": None}[args.rated]
    out = Path(args.out)
    seen = load_seen(out)
    print(f"resuming with {len(seen)} games already on disk -> {out}", flush=True)

    s = requests.Session()
    s.headers.update({"user-agent": UA, "content-type": "application/cbor",
                      "accept": "application/cbor"})
    ep = discover(s)
    print(f"endpoint {ep}", flush=True)

    reqs = 0
    written = 0
    bytes_seen = 0
    t_start = time.time()
    fh = out.open("a", encoding="utf-8")
    try:
        for speed in SPEEDS:
            for tag, rf in RESULTS:
                token = None          # first request: OFFSET page 1; then keyset
                total = None
                while True:
                    if args.max_requests and reqs >= args.max_requests:
                        print("hit --max-requests, stopping", flush=True)
                        return
                    batch, nbytes = post(
                        s, ep,
                        {"options": options(rf, speed, batch_token=token, rated=rated,
                                            exclude_bots=args.exclude_bots)},
                        args.delay)
                    reqs += 1
                    bytes_seen += nbytes
                    games = batch.get("games") or []
                    if total is None:
                        total = int(batch.get("total") or 0)
                        if total:
                            print(f"[{speed}/{tag}] {total} games "
                                  f"({math.ceil(total / BATCH)} pages)", flush=True)
                    for g in games:
                        gid = g.get("game_id") or g.get("uuid")
                        if gid and str(gid) in seen:
                            continue
                        if gid:
                            seen.add(str(gid))
                        fh.write(json.dumps(g, default=jdefault, ensure_ascii=False) + "\n")
                        written += 1
                    fh.flush()
                    # The server decides when a keyset stream is exhausted: it returns
                    # next_batch = null once fewer than batch_size rows remain. Trust
                    # that rather than a page*BATCH>=total arithmetic that OFFSET drift
                    # would make wrong anyway.
                    token = batch.get("next_batch")
                    if not games or not token:
                        break
                    time.sleep(args.delay)
                    if reqs % 50 == 0:
                        el = time.time() - t_start
                        print(f"  ...{reqs} reqs, {written} new games, "
                              f"{bytes_seen/1e6:.0f} MB, {el/60:.1f} min", flush=True)
                time.sleep(args.delay)
    finally:
        fh.close()
        el = time.time() - t_start
        print(f"DONE: {reqs} requests, {written} new games, total on disk "
              f"{len(seen)}, {bytes_seen/1e6:.0f} MB, {el/60:.1f} min", flush=True)


if __name__ == "__main__":
    main()
