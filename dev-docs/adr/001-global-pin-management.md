# ADR-001: Global Pin Management via `dependency-groups.overrides`

## Metadata

- Status: Accepted
- Date: 2026-04-18

## Context

Users need a way to force exact dependency versions across all lock/sync/promote flows without waiting for a conflict-resolution cycle. For example, pinning `numpy==1.26.4` for CUDA compatibility or `transformers==4.44.0` to avoid a breaking API change.

Previously, the only way to influence resolved versions was through the transaction `resolve` flow, which required an active conflict. There was no direct mechanism for proactive version control.

## Decision

Add a `gov pin` command family (`add`, `list`, `remove`) that writes exact-version specs into the shared `dependency-groups.overrides` group in `pyproject.toml`.

Key design choices:

1. **Exact versions only** — `pin add` accepts only `pkg==version`, no range constraints. This keeps override semantics simple and auditable.
2. **Torch-family exclusion** — `torch`, `torchvision`, `torchaudio` are rejected by `pin`; they are governed exclusively by `gov install torch` to avoid dual authority.
3. **Upsert semantics** — `pin add` on an already-pinned package replaces the old pin (remove-then-add in staged workdir), preventing duplicate entries.
4. **Staged workdir with rollback** — mutations happen in a staged workdir; only after successful `lock + prod sync + smoke test` does the result promote to root truth. Failure restores `pyproject.toml` and `uv.lock` and re-syncs prod to the recovered state.
5. **Operation-backed** — each `pin add` / `pin remove` generates a reversible operation in `state/ops/`, consistent with other destructive flows.

## Consequences

- `dependency-groups.overrides` becomes a shared override surface that affects all subsequent lock, prod sync, and transaction resolve outcomes.
- Operators can proactively enforce compatibility constraints without entering a transaction or conflict-resolution flow.
- The rollback guarantee extends to pin operations: failed pin mutations leave local truth unchanged.
