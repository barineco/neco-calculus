#!/bin/sh
set -eu

if [ "$#" -eq 0 ]; then
  echo "usage: check-canonical.sh <json-file-or-directory>..." >&2
  exit 2
fi

python3 - "$@" <<'PY'
import json
import pathlib
import sys

paths = []
for argument in sys.argv[1:]:
    source = pathlib.Path(argument)
    paths.extend(sorted(source.rglob("*.json")) if source.is_dir() else [source])
if not paths:
    raise SystemExit("JSON file がありません")

def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate key: {key}")
        result[key] = value
    return result

for path in paths:
    raw = path.read_bytes()
    text = raw.decode("utf-8")
    value = json.loads(text, object_pairs_hook=unique_object)
    canonical = json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    if canonical != raw:
        raise SystemExit(f"{path}: canonical JSON bytes と一致しません")
print(f"canonical-json: {len(paths)} passed")
PY
