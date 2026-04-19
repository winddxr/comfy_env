# pin

## `pin add`

### Synopsis

```
gov pin add <pkg==version>...
```

### Purpose

Add exact-version override pins to `dependency-groups.overrides`. These pins take precedence during dependency resolution, providing a global override mechanism for compatibility fixes.

### Arguments

- One or more `pkg==version` specs (positional, space-separated)

### Preconditions

- `config.toml` must exist
- Each spec must match `<name>==<version>` format
- Package must not be torch family (torch, torchvision, torchaudio) — rejected with error
- Warns (but proceeds) for non-recommended packages (numpy, transformers are recommended)

### Reads

- `config.toml` — python, prod env
- `pyproject.toml` — current overrides group

### Writes

- `pyproject.toml` — upserts entries in `dependency-groups.overrides`
- `uv.lock` — re-locked
- `.venv-prod/` — synced

### Success Path

```
1. Validate all specs are pkg==version format
2. Reject torch family packages
3. Warn for non-recommended packages
4. Deduplicate specs (last-wins for same package name)
5. op_begin(kind="pin_add")
6. Copy truth to staged workdir
7. For each spec (deduplicated):
   a. If package already exists in overrides: `uv remove --group overrides --frozen <pkg>` in workdir
   b. `uv add --group overrides --frozen <spec>` in workdir
8. Lock workdir: `uv lock --python <py>` via [Staged Workdir Protocol]
9. IF lock succeeds:
   a. Copy workdir truth → root (pyproject.toml, uv.lock)
   b. Sync prod env via [Prod Sync Protocol]
   c. Smoke test via [Smoke Test Protocol]
   d. op_finalize(success)
```

### Failure Path

```
IF validation fails: exit before mutation
IF lock fails: op_restore → re-sync prod → op_finalize(failed)
IF sync fails: op_restore → re-sync prod → op_finalize(failed)
IF smoke test fails: op_restore → re-sync prod → op_finalize(failed)
```

### Platform Notes

- Package name normalization: lowercase, replace `[-_.]` with `-`

---

## `pin list`

### Synopsis

```
gov pin list
```

### Purpose

Display current entries in `dependency-groups.overrides`.

### Preconditions

- `pyproject.toml` must exist

### Reads

- `pyproject.toml` — overrides group entries

### Writes

Nothing.

### Success Path

```
1. Read dependency-groups.overrides from pyproject.toml
2. Print each entry to stdout, one per line
3. If empty: print informational message
```

---

## `pin remove`

### Synopsis

```
gov pin remove <pkg>...
```

### Purpose

Remove packages from `dependency-groups.overrides`.

### Arguments

- One or more package names (positional, normalized before matching)

### Preconditions

- `config.toml` must exist
- Each package must currently exist in overrides group
- Package must not be torch family

### Reads

- `config.toml` — python, prod env
- `pyproject.toml` — current overrides group

### Writes

- `pyproject.toml` — removes entries from `dependency-groups.overrides`
- `uv.lock` — re-locked
- `.venv-prod/` — synced

### Success Path

```
1. Normalize package names
2. Reject torch family
3. Verify each package exists in current overrides
4. op_begin(kind="pin_remove")
5. Copy truth to staged workdir
6. For each package:
   `uv remove --group overrides --python <py> --frozen <pkg>` in workdir
7. Lock workdir via [Staged Workdir Protocol]
8. IF lock succeeds:
   a. Copy workdir truth → root
   b. Sync prod env via [Prod Sync Protocol]
   c. Smoke test via [Smoke Test Protocol]
   d. op_finalize(success)
```

### Failure Path

```
IF package not in overrides: exit with error (no mutation)
IF lock fails: op_restore → re-sync prod → op_finalize(failed)
IF sync/smoke fails: op_restore → re-sync prod → op_finalize(failed)
```
