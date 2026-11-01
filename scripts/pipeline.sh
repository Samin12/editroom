#!/usr/bin/env bash
# editroom pipeline (see pipeline.py --help). Keeps the engine build fresh, then runs the stages.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bash "$HERE/install.sh" --if-stale
exec python3 -I "$HERE/pipeline.py" "$@"
