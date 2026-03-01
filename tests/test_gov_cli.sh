#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

cp -a "$ROOT_DIR" "$TMP_ROOT/work"
WORK_DIR="$TMP_ROOT/work"
FAKE_BIN="$TMP_ROOT/fake-bin"
mkdir -p "$FAKE_BIN"

cat >"$FAKE_BIN/uv" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

cmd="${1:-}"
shift || true

case "$cmd" in
    lock)
        : > uv.lock
        ;;
    sync)
        env_path="${UV_PROJECT_ENVIRONMENT:-}"
        if [ -z "$env_path" ]; then
            echo "missing UV_PROJECT_ENVIRONMENT" >&2
            exit 1
        fi
        mkdir -p "$env_path/bin"
        cat >"$env_path/bin/python" <<'PYEOF'
#!/usr/bin/env bash
exit 0
PYEOF
        chmod +x "$env_path/bin/python"
        ;;
    *)
        ;;
esac
EOF
chmod +x "$FAKE_BIN/uv"

assert_contains() {
    local file="$1"
    local pattern="$2"
    if ! grep -q "$pattern" "$file"; then
        echo "assertion failed: '$pattern' not found in $file" >&2
        exit 1
    fi
}

help_out="$TMP_ROOT/help.txt"
bash "$WORK_DIR/bin/gov" help >"$help_out"
assert_contains "$help_out" "gov install torch --index-url <url>"
assert_contains "$help_out" "gov update run"

set +e
PATH="$FAKE_BIN:$PATH" bash "$WORK_DIR/bin/gov" init >"$TMP_ROOT/init-no-args.out" 2>"$TMP_ROOT/init-no-args.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected init without args to fail" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/init-no-args.err" "initial setup requires"

mkdir -p "$TMP_ROOT/ComfyUI"
PATH="$FAKE_BIN:$PATH" bash "$WORK_DIR/bin/gov" init --comfyui-dir "$TMP_ROOT/ComfyUI" --python 3.12 >"$TMP_ROOT/init.out"

assert_contains "$WORK_DIR/config.toml" "comfyui_dir = \"$TMP_ROOT/ComfyUI\""
assert_contains "$WORK_DIR/config.toml" "python = \"3.12\""

status_out="$TMP_ROOT/status.out"
bash "$WORK_DIR/bin/gov" status >"$status_out"
assert_contains "$status_out" "config_ready: yes"
assert_contains "$status_out" "python: 3.12"
assert_contains "$status_out" "torch_ready: no"

set +e
bash "$WORK_DIR/bin/gov" install >"$TMP_ROOT/install.err" 2>&1
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected install without torch to fail" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/install.err" "managed torch dependencies are not installed"

echo "test_gov_cli.sh: ok"
