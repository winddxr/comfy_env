#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

copy_fresh_workspace() {
    local dst="$1"
    mkdir -p "$dst"
    tar -C "$ROOT_DIR" \
        --exclude='./config.toml' \
        --exclude='./pyproject.toml' \
        --exclude='./uv.lock' \
        --exclude='./state/plugins.json' \
        --exclude='./state/transactions' \
        --exclude='./state/logs' \
        --exclude='./state/conflicts' \
        --exclude='./state/work' \
        --exclude='./state/ops' \
        --exclude='./cache' \
        --exclude='./.venv-prod' \
        --exclude='./.venv-candidate' \
        -cf - . | tar -C "$dst" -xf -
}

copy_fresh_workspace "$TMP_ROOT/work"
WORK_DIR="$TMP_ROOT/work"
FAKE_BIN="$TMP_ROOT/fake-bin"
mkdir -p "$FAKE_BIN"
FAKE_PY_RESOLVERS="$TMP_ROOT/fake-python"
mkdir -p "$FAKE_PY_RESOLVERS"
export FAKE_PY_RESOLVERS

reset_local_state() {
    local dir="$1"
    rm -f "$dir/config.toml" "$dir/pyproject.toml" "$dir/uv.lock"
    rm -f "$dir/state/plugins.json"
    rm -rf "$dir/.venv-prod" "$dir/.venv-candidate"
}

reset_local_state "$WORK_DIR"

cat >"$FAKE_BIN/uv" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

while [ "${1:-}" = "--cache-dir" ]; do
    shift 2
done

cmd="${1:-}"
shift || true

case "$cmd" in
    lock)
        check=false
        while [ $# -gt 0 ]; do
            case "$1" in
                --check)
                    check=true
                    shift
                    ;;
                --python)
                    shift 2
                    ;;
                *)
                    shift
                    ;;
            esac
        done
        if [ "$check" = true ]; then
            if [ "${FAKE_UV_LOCK_CHECK_FAIL:-0}" = "1" ]; then
                echo "lock check failed" >&2
                exit 1
            fi
            exit 0
        fi
        requires_python="$(python3 - <<'PY'
import pathlib
import re

text = pathlib.Path("pyproject.toml").read_text(encoding="utf-8")
match = re.search(r'^requires-python\s*=\s*"([^"]+)"', text, flags=re.MULTILINE)
print(match.group(1) if match else "")
PY
)"
        env_marker="$(python3 - <<'PY'
import pathlib

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

data = tomllib.loads(pathlib.Path("pyproject.toml").read_text(encoding="utf-8"))
tool_uv = (data.get("tool") or {}).get("uv") or {}
envs = tool_uv.get("environments") or []
if isinstance(envs, str):
    envs = [envs]
print(envs[0] if envs else "")
PY
)"
        {
            echo 'version = 1'
            echo 'revision = 3'
            if [ -n "$requires_python" ]; then
                echo "requires-python = \"$requires_python\""
            fi
            if [ -n "$env_marker" ]; then
                echo 'resolution-markers = ['
                echo "    \"$env_marker\","
                echo ']'
            fi
        } > uv.lock
        ;;
    sync)
        if [ "${FAKE_UV_SYNC_FAIL:-0}" = "1" ]; then
            echo "sync failed" >&2
            exit 1
        fi
        env_path="${UV_PROJECT_ENVIRONMENT:-}"
        if [ -z "$env_path" ]; then
            echo "missing UV_PROJECT_ENVIRONMENT" >&2
            exit 1
        fi
        mkdir -p "$env_path/bin"
        cat >"$env_path/bin/python" <<'PYEOF'
#!/usr/bin/env bash
exit "${FAKE_PYTHON_EXIT_CODE:-0}"
PYEOF
        chmod +x "$env_path/bin/python"
        ;;
    export)
        if [ "${FAKE_UV_EXPORT_FAIL:-0}" = "1" ]; then
            echo "export failed" >&2
            exit 1
        fi
        output_file=""
        while [ $# -gt 0 ]; do
            case "$1" in
                --output-file)
                    output_file="${2:-}"
                    shift 2
                    ;;
                *)
                    shift
                    ;;
            esac
        done
        if [ -z "$output_file" ]; then
            echo "missing output file" >&2
            exit 1
        fi
        cat >"$output_file" <<'PYLOCK'
# fake pylock
[[packages]]
name = "demo"
version = "1.0.0"
PYLOCK
        ;;
    pip)
        subcmd="${1:-}"
        shift || true
        case "$subcmd" in
            freeze)
                printf 'demo==1.0.0\n'
                ;;
            *)
                ;;
        esac
        ;;
    python)
        subcmd="${1:-}"
        shift || true
        case "$subcmd" in
            find)
                request=""
                while [ $# -gt 0 ]; do
                    case "$1" in
                        --show-version|--no-python-downloads|--system)
                            shift
                            ;;
                        *)
                            request="$1"
                            shift
                            ;;
                    esac
                done
                if [ "${FAKE_UV_PYTHON_FIND_FAIL_REQUEST:-}" = "$request" ]; then
                    echo "python not found: $request" >&2
                    exit 1
                fi
                case "$request" in
                    *.*.*)
                        version="$request"
                        ;;
                    *.*)
                        version="${request}.9"
                        ;;
                    *)
                        version="${request}.0"
                        ;;
                esac
                resolved_path="$FAKE_PY_RESOLVERS/${version}"
                cat >"$resolved_path" <<PYEOF
#!/usr/bin/env bash
if [ "\${1:-}" = "-c" ]; then
    if [ "\${2:-}" = 'import sys; print(f"{sys.version_info[0]}.{sys.version_info[1]}")' ]; then
        echo "${version%.*}"
        exit 0
    fi
    exit 1
else
    exit 0
fi
PYEOF
                chmod +x "$resolved_path"
                echo "$resolved_path"
                ;;
            *)
                ;;
        esac
        ;;
    *)
        ;;
esac
EOF
chmod +x "$FAKE_BIN/uv"

assert_contains() {
    local file="$1"
    local pattern="$2"
    if ! grep -F -q "$pattern" "$file"; then
        echo "assertion failed: '$pattern' not found in $file" >&2
        exit 1
    fi
}

assert_not_contains() {
    local file="$1"
    local pattern="$2"
    if grep -F -q "$pattern" "$file"; then
        echo "assertion failed: '$pattern' unexpectedly found in $file" >&2
        exit 1
    fi
}

assert_file_exists() {
    local path="$1"
    if [ ! -e "$path" ]; then
        echo "assertion failed: expected file to exist: $path" >&2
        exit 1
    fi
}

assert_not_exists() {
    local path="$1"
    if [ -e "$path" ]; then
        echo "assertion failed: expected path to be absent: $path" >&2
        exit 1
    fi
}

help_out="$TMP_ROOT/help.txt"
bash "$WORK_DIR/bin/gov" help >"$help_out"
assert_contains "$help_out" "gov install torch --index-url <url>"
assert_contains "$help_out" "gov update run"
assert_contains "$help_out" "gov env export <output_dir>"
assert_contains "$help_out" "gov env import <bundle_dir> --comfyui-dir <abs-path> --python <python-spec>"
assert_not_contains "$help_out" "[--force]"

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
assert_contains "$WORK_DIR/pyproject.toml" "requires-python = \"==3.12.*\""
assert_contains "$WORK_DIR/pyproject.toml" "[tool.uv]"
assert_contains "$WORK_DIR/pyproject.toml" "sys_platform == 'linux' and platform_machine == 'x86_64'"
assert_contains "$WORK_DIR/uv.lock" "requires-python = \"==3.12.*\""
assert_contains "$WORK_DIR/uv.lock" "sys_platform == 'linux' and platform_machine == 'x86_64'"

PATH="$FAKE_BIN:$PATH" bash "$WORK_DIR/bin/gov" init --comfyui-dir "$TMP_ROOT/ComfyUI" --python 3.12.9 >"$TMP_ROOT/init-patch.out"
assert_contains "$WORK_DIR/config.toml" "python = \"3.12\""
assert_not_contains "$WORK_DIR/config.toml" "python = \"3.12.9\""

set +e
FAKE_UV_PYTHON_FIND_FAIL_REQUEST=3.12.8 PATH="$FAKE_BIN:$PATH" bash "$WORK_DIR/bin/gov" init --comfyui-dir "$TMP_ROOT/ComfyUI" --python 3.12.8 >"$TMP_ROOT/init-missing-patch.out" 2>"$TMP_ROOT/init-missing-patch.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected init to fail when exact python patch cannot be resolved" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/init-missing-patch.err" "failed to resolve python request: 3.12.8"

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

mkdir -p "$TMP_ROOT/ComfyUI/custom_nodes/demo-node"
cat >"$TMP_ROOT/ComfyUI/custom_nodes/demo-node/__init__.py" <<'EOF'
print("demo")
EOF
(
    cd "$TMP_ROOT/ComfyUI/custom_nodes/demo-node"
    git init >/dev/null 2>&1
    git config user.email "tests@example.invalid"
    git config user.name "Test User"
    git add __init__.py
    git commit -m "init" >/dev/null 2>&1
)
cat >"$TMP_ROOT/ComfyUI/custom_nodes/demo-node/__init__.py" <<'EOF'
print("demo runtime snapshot")
EOF
cat >"$TMP_ROOT/ComfyUI/custom_nodes/demo-node/runtime-extra.txt" <<'EOF'
present in working tree but not committed
EOF
mkdir -p "$TMP_ROOT/ComfyUI/custom_nodes/linked-node"
cat >"$TMP_ROOT/ComfyUI/custom_nodes/linked-node/__init__.py" <<'EOF'
print("linked")
EOF
cat >"$TMP_ROOT/ComfyUI/custom_nodes/linked-node/.git" <<'EOF'
gitdir: /tmp/fake-linked-node
EOF
cat >"$WORK_DIR/state/plugins.json" <<'EOF'
[
  {
    "id": "demo-node",
    "git_url": "https://example.invalid/demo-node.git",
    "ref": "main",
    "install_relpath": "custom_nodes/demo-node",
    "group": "node-demo-node",
    "managed_deps": [],
    "enabled": true,
    "created_at": "2026-04-04T00:00:00Z",
    "updated_at": "2026-04-04T00:00:00Z"
  },
  {
    "id": "linked-node",
    "git_url": "https://example.invalid/linked-node.git",
    "ref": "main",
    "install_relpath": "custom_nodes/linked-node",
    "group": "node-linked-node",
    "managed_deps": [],
    "enabled": true,
    "created_at": "2026-04-04T00:00:00Z",
    "updated_at": "2026-04-04T00:00:00Z"
  }
]
EOF

BUNDLE_DIR="$TMP_ROOT/bundle"
PATH="$FAKE_BIN:$PATH" bash "$WORK_DIR/bin/gov" env export "$BUNDLE_DIR" >"$TMP_ROOT/env-export.out"
assert_file_exists "$BUNDLE_DIR/manifest.json"
assert_file_exists "$BUNDLE_DIR/pyproject.toml"
assert_file_exists "$BUNDLE_DIR/uv.lock"
assert_file_exists "$BUNDLE_DIR/pylock.toml"
assert_file_exists "$BUNDLE_DIR/state/plugins.json"
assert_file_exists "$BUNDLE_DIR/custom_nodes/demo-node/__init__.py"
assert_file_exists "$BUNDLE_DIR/custom_nodes/demo-node/runtime-extra.txt"
assert_not_exists "$BUNDLE_DIR/custom_nodes/demo-node/.git"
assert_file_exists "$BUNDLE_DIR/custom_nodes/linked-node/__init__.py"
assert_not_exists "$BUNDLE_DIR/custom_nodes/linked-node/.git"
assert_file_exists "$BUNDLE_DIR/audit/prod-freeze.txt"
assert_file_exists "$BUNDLE_DIR/audit/export-summary.json"

set +e
FAKE_UV_EXPORT_FAIL=1 PATH="$FAKE_BIN:$PATH" bash "$WORK_DIR/bin/gov" env export "$TMP_ROOT/bundle-export-fail" >"$TMP_ROOT/env-export-fail.out" 2>"$TMP_ROOT/env-export-fail.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env export failure when uv export fails" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-export-fail.err" "export failed"

rm -rf "$TMP_ROOT/ComfyUI/custom_nodes/demo-node"
set +e
PATH="$FAKE_BIN:$PATH" bash "$WORK_DIR/bin/gov" env export "$TMP_ROOT/bundle-missing-node" >"$TMP_ROOT/env-export-missing.out" 2>"$TMP_ROOT/env-export-missing.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env export failure when plugin source is missing" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-export-missing.err" "plugin source directory missing"

copy_fresh_workspace "$TMP_ROOT/import-work"
IMPORT_WORK_DIR="$TMP_ROOT/import-work"
reset_local_state "$IMPORT_WORK_DIR"
IMPORT_COMFY="$TMP_ROOT/ComfyUI-import"
mkdir -p "$IMPORT_COMFY"
PATH="$FAKE_BIN:$PATH" bash "$IMPORT_WORK_DIR/bin/gov" env import "$BUNDLE_DIR" --comfyui-dir "$IMPORT_COMFY" --python 3.12 >"$TMP_ROOT/env-import.out"
assert_file_exists "$IMPORT_WORK_DIR/pyproject.toml"
assert_file_exists "$IMPORT_WORK_DIR/uv.lock"
assert_file_exists "$IMPORT_WORK_DIR/state/plugins.json"
assert_file_exists "$IMPORT_WORK_DIR/.venv-prod/bin/python"
assert_file_exists "$IMPORT_COMFY/custom_nodes/demo-node/__init__.py"
assert_file_exists "$IMPORT_COMFY/custom_nodes/demo-node/runtime-extra.txt"
assert_file_exists "$IMPORT_COMFY/custom_nodes/linked-node/__init__.py"
assert_not_exists "$IMPORT_COMFY/custom_nodes/linked-node/.git"
assert_contains "$IMPORT_WORK_DIR/config.toml" "comfyui_dir = \"$IMPORT_COMFY\""
assert_contains "$IMPORT_WORK_DIR/config.toml" "python = \"3.12\""
assert_contains "$IMPORT_WORK_DIR/state/plugins.json" "\"demo-node\""
assert_contains "$IMPORT_WORK_DIR/state/plugins.json" "\"linked-node\""

REEXPORT_DIR="$TMP_ROOT/bundle-reexport"
PATH="$FAKE_BIN:$PATH" bash "$IMPORT_WORK_DIR/bin/gov" env export "$REEXPORT_DIR" >"$TMP_ROOT/env-reexport.out"
assert_file_exists "$REEXPORT_DIR/custom_nodes/demo-node/__init__.py"
assert_file_exists "$REEXPORT_DIR/custom_nodes/demo-node/runtime-extra.txt"
assert_file_exists "$REEXPORT_DIR/custom_nodes/linked-node/__init__.py"
assert_not_exists "$REEXPORT_DIR/custom_nodes/demo-node/.git"
assert_not_exists "$REEXPORT_DIR/custom_nodes/linked-node/.git"

copy_fresh_workspace "$TMP_ROOT/import-python-mismatch-work"
PY_MISMATCH_WORK_DIR="$TMP_ROOT/import-python-mismatch-work"
reset_local_state "$PY_MISMATCH_WORK_DIR"
PY_MISMATCH_COMFY="$TMP_ROOT/ComfyUI-import-python-mismatch"
mkdir -p "$PY_MISMATCH_COMFY"
set +e
PATH="$FAKE_BIN:$PATH" bash "$PY_MISMATCH_WORK_DIR/bin/gov" env import "$BUNDLE_DIR" --comfyui-dir "$PY_MISMATCH_COMFY" --python 3.11 >"$TMP_ROOT/env-import-python-mismatch.out" 2>"$TMP_ROOT/env-import-python-mismatch.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env import failure for python minor mismatch" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-import-python-mismatch.err" "bundle requires-python is ==3.12.*"
assert_not_exists "$PY_MISMATCH_WORK_DIR/state/plugins.json"

cp -a "$BUNDLE_DIR" "$TMP_ROOT/bundle-python-spacing"
python3 - "$TMP_ROOT/bundle-python-spacing" <<'PY'
import json
import pathlib
import re
import sys

bundle = pathlib.Path(sys.argv[1])
project_path = bundle / "pyproject.toml"
text = project_path.read_text(encoding="utf-8")
text = re.sub(
    r'requires-python\s*=\s*"==3\.12\.\*"',
    'requires-python = "== 3.12.*"',
    text,
    count=1,
)
project_path.write_text(text, encoding="utf-8")

manifest_path = bundle / "manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
manifest["files"]["sha256"]["pyproject.toml"] = __import__("hashlib").sha256(project_path.read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
PY
copy_fresh_workspace "$TMP_ROOT/import-python-spacing-work"
PY_SPACING_WORK_DIR="$TMP_ROOT/import-python-spacing-work"
reset_local_state "$PY_SPACING_WORK_DIR"
PY_SPACING_COMFY="$TMP_ROOT/ComfyUI-import-python-spacing"
mkdir -p "$PY_SPACING_COMFY"
PATH="$FAKE_BIN:$PATH" bash "$PY_SPACING_WORK_DIR/bin/gov" env import "$TMP_ROOT/bundle-python-spacing" --comfyui-dir "$PY_SPACING_COMFY" --python 3.12 >"$TMP_ROOT/env-import-python-spacing.out"
assert_file_exists "$PY_SPACING_WORK_DIR/state/plugins.json"
assert_file_exists "$PY_SPACING_COMFY/custom_nodes/demo-node/__init__.py"

cp -a "$BUNDLE_DIR" "$TMP_ROOT/bundle-platform-subset"
python3 - "$TMP_ROOT/bundle-platform-subset" <<'PY'
import json
import pathlib
import re
import sys

bundle = pathlib.Path(sys.argv[1])
project_path = bundle / "pyproject.toml"
text = project_path.read_text(encoding="utf-8")
text = re.sub(
    r"(environments\s*=\s*\[\s*\")([^\"]+)(\",\s*\])",
    rf"\1sys_platform == '{sys.platform}'\3",
    text,
    count=1,
    flags=re.MULTILINE | re.DOTALL,
)
project_path.write_text(text, encoding="utf-8")

manifest_path = bundle / "manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
manifest["files"]["sha256"]["pyproject.toml"] = __import__("hashlib").sha256(project_path.read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
PY
copy_fresh_workspace "$TMP_ROOT/import-platform-subset-work"
PLATFORM_SUBSET_WORK_DIR="$TMP_ROOT/import-platform-subset-work"
reset_local_state "$PLATFORM_SUBSET_WORK_DIR"
PLATFORM_SUBSET_COMFY="$TMP_ROOT/ComfyUI-import-platform-subset"
mkdir -p "$PLATFORM_SUBSET_COMFY"
PATH="$FAKE_BIN:$PATH" bash "$PLATFORM_SUBSET_WORK_DIR/bin/gov" env import "$TMP_ROOT/bundle-platform-subset" --comfyui-dir "$PLATFORM_SUBSET_COMFY" --python 3.12 >"$TMP_ROOT/env-import-platform-subset.out"
assert_file_exists "$PLATFORM_SUBSET_WORK_DIR/state/plugins.json"
assert_file_exists "$PLATFORM_SUBSET_COMFY/custom_nodes/demo-node/__init__.py"

cp -a "$BUNDLE_DIR" "$TMP_ROOT/bundle-platform-mismatch"
python3 - "$TMP_ROOT/bundle-platform-mismatch" <<'PY'
import json
import pathlib
import re
import sys

bundle = pathlib.Path(sys.argv[1])
project_path = bundle / "pyproject.toml"
text = project_path.read_text(encoding="utf-8")
text = re.sub(
    r"(environments\s*=\s*\[\s*\")([^\"]+)(\",\s*\])",
    r"\1sys_platform == 'darwin' and platform_machine == 'arm64'\3",
    text,
    count=1,
    flags=re.MULTILINE | re.DOTALL,
)
project_path.write_text(text, encoding="utf-8")

manifest_path = bundle / "manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
manifest["files"]["sha256"]["pyproject.toml"] = __import__("hashlib").sha256(project_path.read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
PY
copy_fresh_workspace "$TMP_ROOT/import-platform-mismatch-work"
PLATFORM_MISMATCH_WORK_DIR="$TMP_ROOT/import-platform-mismatch-work"
reset_local_state "$PLATFORM_MISMATCH_WORK_DIR"
PLATFORM_MISMATCH_COMFY="$TMP_ROOT/ComfyUI-import-platform-mismatch"
mkdir -p "$PLATFORM_MISMATCH_COMFY"
set +e
PATH="$FAKE_BIN:$PATH" bash "$PLATFORM_MISMATCH_WORK_DIR/bin/gov" env import "$TMP_ROOT/bundle-platform-mismatch" --comfyui-dir "$PLATFORM_MISMATCH_COMFY" --python 3.12 >"$TMP_ROOT/env-import-platform-mismatch.out" 2>"$TMP_ROOT/env-import-platform-mismatch.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env import failure for platform mismatch" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-import-platform-mismatch.err" "bundle environments do not support this host"
assert_not_exists "$PLATFORM_MISMATCH_WORK_DIR/state/plugins.json"

cp -a "$BUNDLE_DIR" "$TMP_ROOT/bundle-corrupt"
printf '\n# corrupt\n' >>"$TMP_ROOT/bundle-corrupt/pyproject.toml"
copy_fresh_workspace "$TMP_ROOT/import-corrupt-work"
reset_local_state "$TMP_ROOT/import-corrupt-work"
mkdir -p "$TMP_ROOT/ComfyUI-import-corrupt"
set +e
PATH="$FAKE_BIN:$PATH" bash "$TMP_ROOT/import-corrupt-work/bin/gov" env import "$TMP_ROOT/bundle-corrupt" --comfyui-dir "$TMP_ROOT/ComfyUI-import-corrupt" --python 3.12 >"$TMP_ROOT/env-import-corrupt.out" 2>"$TMP_ROOT/env-import-corrupt.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env import failure for corrupt bundle" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-import-corrupt.err" "bundle checksum mismatch"

copy_fresh_workspace "$TMP_ROOT/import-conflict-work"
EXACT_WORK_DIR="$TMP_ROOT/import-conflict-work"
EXACT_COMFY="$TMP_ROOT/ComfyUI-import-conflict"
mkdir -p "$EXACT_WORK_DIR/state" "$EXACT_WORK_DIR/.venv-prod" "$EXACT_COMFY/custom_nodes/demo-node" "$EXACT_COMFY/custom_nodes/legacy-node"
cat >"$EXACT_WORK_DIR/pyproject.toml" <<'EOF'
[project]
name = "original"
version = "0.0.1"
EOF
cat >"$EXACT_WORK_DIR/uv.lock" <<'EOF'
original-lock
EOF
cat >"$EXACT_WORK_DIR/state/plugins.json" <<'EOF'
[
  {
    "id": "legacy-node",
    "git_url": "https://example.invalid/legacy.git",
    "ref": "main",
    "install_relpath": "custom_nodes/legacy-node",
    "group": "node-legacy-node",
    "managed_deps": [],
    "enabled": true,
    "created_at": "2026-04-02T00:00:00Z",
    "updated_at": "2026-04-02T00:00:00Z"
  }
]
EOF
touch "$EXACT_WORK_DIR/.venv-prod/stale.txt"
cat >"$EXACT_COMFY/custom_nodes/demo-node/original.txt" <<'EOF'
old demo node
EOF
cat >"$EXACT_COMFY/custom_nodes/legacy-node/legacy.txt" <<'EOF'
legacy node
EOF
PATH="$FAKE_BIN:$PATH" bash "$EXACT_WORK_DIR/bin/gov" env import "$BUNDLE_DIR" --comfyui-dir "$EXACT_COMFY" --python 3.12 >"$TMP_ROOT/env-import-exact.out"
assert_file_exists "$EXACT_COMFY/custom_nodes/demo-node/__init__.py"
assert_not_exists "$EXACT_COMFY/custom_nodes/legacy-node"
assert_contains "$EXACT_WORK_DIR/state/plugins.json" "\"demo-node\""
assert_not_contains "$EXACT_WORK_DIR/state/plugins.json" "\"legacy-node\""
assert_not_contains "$EXACT_WORK_DIR/pyproject.toml" "name = \"original\""
assert_not_contains "$EXACT_WORK_DIR/uv.lock" "original-lock"

copy_fresh_workspace "$TMP_ROOT/import-relative-work"
RELATIVE_WORK_DIR="$TMP_ROOT/import-relative-work"
reset_local_state "$RELATIVE_WORK_DIR"
mkdir -p "$ROOT_DIR/../relative-comfy"
set +e
(cd "$ROOT_DIR" && PATH="$FAKE_BIN:$PATH" bash "$RELATIVE_WORK_DIR/bin/gov" env import "$BUNDLE_DIR" --comfyui-dir ../relative-comfy --python 3.12 >"$TMP_ROOT/env-import-relative.out" 2>"$TMP_ROOT/env-import-relative.err")
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env import failure for relative comfyui dir" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-import-relative.err" "ComfyUI path must be absolute"

copy_fresh_workspace "$TMP_ROOT/import-lockfail-work"
LOCKFAIL_WORK_DIR="$TMP_ROOT/import-lockfail-work"
reset_local_state "$LOCKFAIL_WORK_DIR"
LOCKFAIL_COMFY="$TMP_ROOT/ComfyUI-import-lockfail"
mkdir -p "$LOCKFAIL_COMFY"
set +e
FAKE_UV_LOCK_CHECK_FAIL=1 PATH="$FAKE_BIN:$PATH" bash "$LOCKFAIL_WORK_DIR/bin/gov" env import "$BUNDLE_DIR" --comfyui-dir "$LOCKFAIL_COMFY" --python 3.12 >"$TMP_ROOT/env-import-lockfail.out" 2>"$TMP_ROOT/env-import-lockfail.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env import failure when lock check fails" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-import-lockfail.err" "bundle lock check failed"
assert_not_exists "$LOCKFAIL_WORK_DIR/state/plugins.json"

copy_fresh_workspace "$TMP_ROOT/import-rollback-work"
ROLLBACK_WORK_DIR="$TMP_ROOT/import-rollback-work"
reset_local_state "$ROLLBACK_WORK_DIR"
ROLLBACK_COMFY="$TMP_ROOT/ComfyUI-import-rollback"
mkdir -p "$ROLLBACK_WORK_DIR/state" "$ROLLBACK_COMFY/custom_nodes/demo-node" "$ROLLBACK_COMFY/custom_nodes/legacy-node"
cat >"$ROLLBACK_WORK_DIR/pyproject.toml" <<'EOF'
[project]
name = "original"
version = "0.0.1"
EOF
cat >"$ROLLBACK_WORK_DIR/uv.lock" <<'EOF'
original-lock
EOF
cat >"$ROLLBACK_WORK_DIR/state/plugins.json" <<'EOF'
[
  {
    "id": "demo-node",
    "git_url": "https://example.invalid/original.git",
    "ref": "main",
    "install_relpath": "custom_nodes/demo-node",
    "group": "node-demo-node",
    "managed_deps": [],
    "enabled": true,
    "created_at": "2026-04-01T00:00:00Z",
    "updated_at": "2026-04-01T00:00:00Z"
  },
  {
    "id": "legacy-node",
    "git_url": "https://example.invalid/legacy.git",
    "ref": "main",
    "install_relpath": "custom_nodes/legacy-node",
    "group": "node-legacy-node",
    "managed_deps": [],
    "enabled": true,
    "created_at": "2026-04-01T00:00:00Z",
    "updated_at": "2026-04-01T00:00:00Z"
  }
]
EOF
cat >"$ROLLBACK_COMFY/custom_nodes/demo-node/original.txt" <<'EOF'
original node
EOF
cat >"$ROLLBACK_COMFY/custom_nodes/legacy-node/legacy.txt" <<'EOF'
legacy node
EOF
set +e
FAKE_PYTHON_EXIT_CODE=1 PATH="$FAKE_BIN:$PATH" bash "$ROLLBACK_WORK_DIR/bin/gov" env import "$BUNDLE_DIR" --comfyui-dir "$ROLLBACK_COMFY" --python 3.12 >"$TMP_ROOT/env-import-rollback.out" 2>"$TMP_ROOT/env-import-rollback.err"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
    echo "expected env import failure when smoke test fails" >&2
    exit 1
fi
assert_contains "$TMP_ROOT/env-import-rollback.err" "smoke test failed during env import"
assert_contains "$ROLLBACK_WORK_DIR/pyproject.toml" "name = \"original\""
assert_contains "$ROLLBACK_WORK_DIR/uv.lock" "original-lock"
assert_contains "$ROLLBACK_COMFY/custom_nodes/demo-node/original.txt" "original node"
assert_contains "$ROLLBACK_COMFY/custom_nodes/legacy-node/legacy.txt" "legacy node"
assert_contains "$ROLLBACK_WORK_DIR/state/plugins.json" "\"legacy-node\""

echo "test_gov_cli.sh: ok"
