#!/bin/sh
set -eu

if [ "$#" -ne 4 ] || [ "$1" != "--request" ] || [ "$3" != "--minimum" ]; then
  echo "usage: run-smoke.sh --request <canonical-generated-request> --minimum <logical-case-count>" >&2
  exit 2
fi

request=$2
minimum=$4
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cargo_bin=cargo
lake_bin=lake
jq_bin=jq
bound_producer=${NECO_VERIFICATION_BOUND_PRODUCER:-}

case "$minimum" in
  '' | *[!0-9]*)
    echo "minimum must be an unsigned integer" >&2
    exit 2
    ;;
esac

if [ "$minimum" -lt 1000 ]; then
  echo "minimum must be at least 1000" >&2
  exit 2
fi

if [ ! -f "$request" ]; then
  echo "canonical generated request does not exist: $request" >&2
  exit 2
fi

if [ -z "$bound_producer" ] || [ ! -x "$bound_producer" ]; then
  echo "NECO_VERIFICATION_BOUND_PRODUCER must name an executable bound producer" >&2
  exit 2
fi

case_count=$("$jq_bin" -er '.cases | if type == "array" then length else error("generated request cases must be an array") end' "$request")

if [ "$case_count" -lt "$minimum" ]; then
  echo "generated request has $case_count logical cases; minimum is $minimum" >&2
  exit 2
fi

artifacts=$root/verification/artifacts
lean_artifacts=$artifacts/lean
rust_artifacts=$artifacts/rust
mkdir -p "$lean_artifacts" "$rust_artifacts"

(cd "$root/verification/lean" && "$lake_bin" build)
cp "$root/verification/lean/.lake/build/bin/neco-calculus-checker" \
  "$lean_artifacts/neco-calculus-checker"

batch=$rust_artifacts/calculus-generated-observation-batch.json
checker_result=$lean_artifacts/calculus-generated-checker-result.json
"$bound_producer" generate --request "$request" > "$batch"
"$bound_producer" self-check --request "$request" --batch "$batch" \
  > "$rust_artifacts/calculus-generated-public-api-parity.json"
"$lean_artifacts/neco-calculus-checker" check "$batch" > "$checker_result"

observer_result=$("$cargo_bin" run \
  --quiet \
  --manifest-path "$root/verification/observer/Cargo.toml" \
  --locked \
  -- --batch "$batch" --checker-result "$checker_result")

printf '%s' "$observer_result" | "$jq_bin" -e --argjson expected "$case_count" \
  '.cases | length == 1 and .verified == $expected' >/dev/null
printf '%s\n' "$observer_result"
