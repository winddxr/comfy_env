# Comfy Environment Governance (`comfy_env`)

Batch-1 to Batch-3 implementation for ComfyUI dependency governance.

## Scope

This directory provides:

- `gov init` for production environment bootstrap (`.venv-prod`)
- `gov node add` for manual git node registration
- `gov tx run` / `gov tx inspect` / `gov tx abort` for candidate transaction execution
- `gov status` for environment and transaction summary
- `gov run` placeholder for future production switch control
- Batch-2/3 commands: `gov resolve`, `gov tx promote`, `gov node remove`, `gov rollback`, `gov snapshot`

Current implementation includes `resolve`, `tx promote`, `node remove`, `rollback`, and `snapshot` flows.

Development and testing master guide:

- `comfy_env_dev_test_guide.md`

## Directory layout

```text
comfy_env/
├── bin/
│   └── gov
├── cache/
│   └── wheels/
├── snapshots/
├── state/
│   ├── conflicts/
│   ├── plugins.json
│   ├── logs/
│   ├── snapshots_index.json
│   ├── transactions/
│   └── work/
├── config.toml
└── pyproject.toml
```

## Quick start

```bash
cd comfy_env
./bin/gov init
./bin/gov status
```
