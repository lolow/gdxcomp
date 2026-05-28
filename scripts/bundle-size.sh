#!/usr/bin/env bash
# Report the size of every JS/CSS chunk produced by `npm run build`.
# Used to track Phase-4 bundle-reduction work.
#
# Usage:
#   ./scripts/bundle-size.sh
#
# Outputs one line per asset to stdout: "<bytes>\t<path>".
set -euo pipefail

cd "$(dirname "$0")/.."

npm run build >/dev/null

echo "# bundle sizes ($(date -Is))"
for f in dist/assets/*.js dist/assets/*.css; do
  [ -f "$f" ] || continue
  bytes=$(stat -c %s "$f")
  printf "%10d  %s\n" "$bytes" "$f"
done
echo "total:"
du -bsh dist/
