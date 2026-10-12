#!/usr/bin/env bash
# usage: no-fallbacks.sh <adapter> <fallbacks.json> [allowed-fallbacks.json]
# Fails when the adapter ships a query the compiler did not take that the allow-list does not name,
# or the allow-list names a query the compiler now takes, or names it with another reason.
set -euo pipefail

adapter=$1
observed=$2
allowed=${3:-"$(dirname "$0")/../allowed-fallbacks.json"}

jq -e "type == \"array\"" "$observed" > /dev/null

problems=$(jq -r --arg adapter "$adapter" --slurpfile allowed "$allowed" '
  def key: "\(.query)\t\(.reason)";
  ($allowed[0] | map(select(.adapter == $adapter))) as $listed
  | (map(key)) as $seen
  | ($listed | map(key)) as $permitted
  | (.[] | select(key as $k | $permitted | any(. == $k) | not)
      | "::error title=\($adapter) query falls back to stock Oxigraph::\($adapter) \(.query) is not compiled: \(.reason). Make the compiler take it, or add it to .github/allowed-fallbacks.json with this reason."),
    ($listed[] | select(key as $k | $seen | any(. == $k) | not)
      | "::error title=stale entry in allowed-fallbacks.json::\($adapter) \(.query) is allowed to fall back with the reason \"\(.reason)\", and does not. Remove or correct the entry.")
' "$observed")

if [ -n "$problems" ]; then
  echo "$problems"
  exit 1
fi
count=$(jq length "$observed")
if [ "$count" -eq 1 ]; then noun=query; else noun=queries; fi
echo "$adapter: $count $noun not compiled, so evaluated by stock Oxigraph; all of them allowed"
