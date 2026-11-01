#!/usr/bin/env bash
# editroom installer: checks dependencies, builds the cutroom engine into ~/.editroom (never touches
# ~/.cargo/bin/cutroom, which Creator Club uses) and puts an `editroom` command next to it.
#
#   scripts/install.sh              # check + build (skips the build when the engine source is unchanged)
#   scripts/install.sh --force      # rebuild anyway
#   scripts/install.sh --if-stale   # quiet: build only when needed (the pipeline calls this)
#   scripts/install.sh --check      # only report dependencies
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOME_DIR="${EDITROOM_HOME:-$HOME/.editroom}"
BIN="$HOME_DIR/bin"
MODE="${1:-}"
quiet=0; [[ "$MODE" == "--if-stale" ]] && quiet=1
say() { [[ $quiet == 1 ]] || echo "$@"; }

# --- dependencies -------------------------------------------------------------------------------
missing=0
need() { # name, command, required(1/0), hint
  if command -v "$2" >/dev/null 2>&1; then say "  ok   $1 ($(command -v "$2"))"
  elif [[ "$3" == 1 ]]; then echo "  MISSING $1: $4" >&2; missing=1
  else say "  --   $1 not found: $4"; fi
}
if [[ $quiet == 0 ]]; then echo "editroom: checking dependencies"; fi
[[ -x "$HOME/.cargo/bin/cargo" ]] && export PATH="$HOME/.cargo/bin:$PATH"
need "rust (cargo)" cargo 1 "install with: curl https://sh.rustup.rs -sSf | sh"
need ffmpeg ffmpeg 1 "brew install ffmpeg"
need ffprobe ffprobe 1 "brew install ffmpeg"
need tesseract tesseract 1 "brew install tesseract (privacy scan)"
need "claude CLI" claude 1 "https://claude.com/claude-code (editorial passes)"
need whisper-cli whisper-cli 0 "brew install whisper-cpp (only for plain files without a transcript)"
need node node 0 "brew install node (proof screenshots)"
if [[ -d "/Applications/Google Chrome.app" ]]; then say "  ok   Google Chrome"; else say "  --   Google Chrome not found (proof screenshots use puppeteer's own Chrome then)"; fi
[[ $missing == 1 ]] && { echo "editroom: install the missing tools above, then run this again" >&2; exit 1; }
[[ "$MODE" == "--check" ]] && exit 0

# --- engine build -------------------------------------------------------------------------------
mkdir -p "$BIN"
stamp_file="$HOME_DIR/engine.stamp"
stamp="$(cd "$ROOT/engine" && find crates Cargo.toml Cargo.lock -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.lock' -o -name '*.ttf' -o -name '*.otf' \) -print0 | sort -z | xargs -0 shasum | shasum | cut -c1-16)"
if [[ "$MODE" != "--force" && -x "$BIN/cutroom" && -f "$stamp_file" && "$(cat "$stamp_file")" == "$stamp" ]]; then
  say "editroom: engine up to date ($BIN/cutroom)"
else
  say "editroom: building the cutroom engine (release, ~20 s on an M5 Ultra)"
  if [[ $quiet == 1 ]]; then
    cargo install --quiet --path "$ROOT/engine/crates/cli" --root "$HOME_DIR" --force --locked >/dev/null 2>"$HOME_DIR/build.log" \
      || { echo "editroom: engine build failed, see $HOME_DIR/build.log" >&2; exit 1; }
  else
    cargo install --path "$ROOT/engine/crates/cli" --root "$HOME_DIR" --force --locked
  fi
  echo "$stamp" > "$stamp_file"
fi

# --- the editroom command -----------------------------------------------------------------------
echo "$ROOT" > "$HOME_DIR/root"
cat > "$BIN/editroom" <<EOF
#!/usr/bin/env bash
# editroom: thin dispatcher to the plugin scripts (installed by scripts/install.sh)
ROOT="\$(cat "$HOME_DIR/root")"
cmd="\${1:-help}"; shift || true
case "\$cmd" in
  find)    exec python3 -I "\$ROOT/scripts/find_project.py" "\$@" ;;
  run)     exec bash "\$ROOT/scripts/pipeline.sh" "\$@" ;;
  qa)      exec python3 -I "\$ROOT/scripts/qa.py" "\$@" ;;
  proof)   exec node "\$ROOT/scripts/capture_proof.mjs" "\$@" ;;
  install) exec bash "\$ROOT/scripts/install.sh" "\$@" ;;
  root)    echo "\$ROOT" ;;
  *) echo "editroom find [name] | run <project|path> [--work DIR] [--from-stage S] [--dry-run] | qa <video> --work DIR | proof <url> <out.png> | install [--force]"; exit 0 ;;
esac
EOF
chmod +x "$BIN/editroom"

say "editroom: ready"
say "  engine   $BIN/cutroom ($("$BIN/cutroom" --version 2>/dev/null || echo '?'))"
say "  command  $BIN/editroom"
case ":$PATH:" in *":$BIN:"*) ;; *) say "  tip: add $BIN to PATH, or call it by full path (the skill always does)";; esac
