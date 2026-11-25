#!/usr/bin/env bash
# Refresh fuzz corpora from yaml-test-suite in.yaml files + indicator seeds.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUITE="$ROOT/vendor/yaml-test-suite"
PARSE_CORPUS="$ROOT/fuzz/corpus/fuzz_parse_str"
RT_CORPUS="$ROOT/fuzz/corpus/fuzz_roundtrip"

mkdir -p "$PARSE_CORPUS" "$RT_CORPUS"

n=0
while IFS= read -r -d '' f; do
  id="$(basename "$(dirname "$f")")"
  # Skip nested name/ tags paths; keep top-level suite ids only.
  parent="$(basename "$(dirname "$(dirname "$f")")")"
  if [[ "$parent" != "yaml-test-suite" ]]; then
    continue
  fi
  cp "$f" "$PARSE_CORPUS/${id}.yaml"
  cp "$f" "$RT_CORPUS/${id}.yaml"
  n=$((n + 1))
done < <(find "$SUITE" -maxdepth 2 -name in.yaml -print0)

# Indicator / marker mutation seeds
cat >"$PARSE_CORPUS/seed_markers.yaml" <<'EOF'
---
...
---
a: 1
...
EOF
cat >"$PARSE_CORPUS/seed_indicators.yaml" <<'EOF'
|  
>  
? k
: v
&a *a
!tag x
EOF
cp "$PARSE_CORPUS/seed_markers.yaml" "$RT_CORPUS/seed_markers.yaml"
cp "$PARSE_CORPUS/seed_indicators.yaml" "$RT_CORPUS/seed_indicators.yaml"

echo "Seeded $n suite files + indicator seeds into fuzz/corpus/"
