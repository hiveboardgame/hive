#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage: scripts/samply-top-functions.sh [count] [profile.json.gz] [binary]

Print the top sampled leaf functions by self CPU time from a samply profile.
Defaults: count=20, profile=profile.json.gz, binary=path recorded in profile.
USAGE
}

top_n=20
profile=profile.json.gz
binary=

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

if [[ $# -gt 0 && "$1" =~ ^[0-9]+$ ]]; then
  top_n="$1"
  shift
fi

if [[ $# -gt 0 ]]; then
  profile="$1"
  shift
fi

if [[ $# -gt 0 ]]; then
  binary="$1"
  shift
fi

if [[ $# -gt 0 || ! "$top_n" =~ ^[0-9]+$ || "$top_n" -lt 1 ]]; then
  usage
  exit 2
fi

if [[ ! -f "$profile" ]]; then
  echo "Profile not found: $profile" >&2
  exit 1
fi

for tool in gzip jq addr2line awk sort; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing required tool: $tool" >&2
    exit 1
  fi
done

if [[ -z "$binary" ]]; then
  binary="$(
    gzip -dc "$profile" |
      jq -r '.meta.product as $product | [.libs[] | select(.path | endswith("/" + $product)) | .path][0] // empty'
  )"
fi

if [[ -z "$binary" || ! -f "$binary" ]]; then
  echo "Could not find the profiled binary. Pass it as the third argument." >&2
  exit 1
fi

gzip -dc "$profile" |
  jq -r --arg binary "$binary" '
    def sample_ms($t; $i):
      (($t.samples.threadCPUDelta // [])[$i]) as $cpu
      | if $cpu != null then ($cpu / 1000) else (($t.samples.timeDeltas // [])[$i] // 0) end;

    . as $profile
    | [
        $profile.threads[] as $thread
        | range(0; $thread.samples.length) as $i
        | ($thread.samples.stack[$i]) as $stack
        | select($stack != null)
        | ($thread.stackTable.frame[$stack]) as $frame
        | select($frame != null)
        | ($thread.frameTable.func[$frame]) as $func
        | select($func != null)
        | ($thread.funcTable.resource[$func]) as $resource
        | select($resource != null and $resource >= 0)
        | ($thread.resourceTable.lib[$resource]) as $lib
        | select($lib != null and $lib >= 0)
        | ($profile.libs[$lib].path // "") as $lib_path
        | select($lib_path == $binary)
        | {
            address: $thread.frameTable.address[$frame],
            ms: sample_ms($thread; $i),
            samples: (($thread.samples.weight // [])[$i] // 1)
          }
        | select(.address != null and .address >= 0)
      ]
    | group_by(.address)
    | map({
        address: .[0].address,
        ms: (map(.ms) | add),
        samples: (map(.samples) | add)
      })
    | .[]
    | [.ms, .samples, .address]
    | @tsv
  ' |
  while IFS=$'\t' read -r ms samples address; do
    hex_address="$(printf '0x%x' "$address")"
    symbol=
    IFS= read -r symbol < <(addr2line -Cfe "$binary" "$hex_address") || true

    if [[ -z "$symbol" || "$symbol" == "??" ]]; then
      symbol="$hex_address"
    fi

    printf '%s\t%s\t%s\n' "$ms" "$samples" "$symbol"
  done |
  awk -F '\t' '
    {
      self_ms[$3] += $1
      samples[$3] += $2
    }
    END {
      for (name in self_ms) {
        printf "%.3f\t%d\t%s\n", self_ms[name], samples[name], name
      }
    }
  ' |
  sort -t $'\t' -k1,1nr |
  awk -F '\t' -v limit="$top_n" '
    BEGIN {
      printf "%8s %8s  %s\n", "self_ms", "samples", "function"
    }
    NR <= limit {
      printf "%8.3f %8d  %s\n", $1, $2, $3
    }
  '
