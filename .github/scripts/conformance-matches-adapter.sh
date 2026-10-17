#!/usr/bin/env bash
# usage: conformance-matches-adapter.sh <ccda adapter checkout> [conformance directory]
# Fails, naming the file, when a conformance copy differs from the adapter's fixtures/in file of that name.
set -euo pipefail

adapter=$1
copies=${2:-"$(dirname "$0")/../../crates/bridge/tests/cda/conformance"}

failed=0
count=0
for copy in "$copies"/*.xml; do
  name=$(basename "$copy")
  count=$((count + 1))
  if [ ! -f "$adapter/fixtures/in/$name" ]; then
    echo "::error file=crates/bridge/tests/cda/conformance/$name::$name is not in cascade-bridge-adapter-ccda fixtures/in. The adapter owns Jane Doe's files; delete the copy."
    failed=1
  elif ! cmp -s "$copy" "$adapter/fixtures/in/$name"; then
    echo "::error file=crates/bridge/tests/cda/conformance/$name::$name differs from cascade-bridge-adapter-ccda fixtures/in/$name. Change the file in the adapter and copy it here."
    failed=1
  fi
done
[ "$count" -gt 0 ] || { echo "::error::no conformance files under $copies"; exit 1; }
exit "$failed"
