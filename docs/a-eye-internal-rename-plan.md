# A-Eye Internal Rename Plan

## Objective

Rename internal package/module/crate identifiers from legacy `codex-*` naming to `a-eye-*` (or `aeye-*` where tool constraints require), without breaking builds, tests, scripts, or user workflows.

## Guardrails

1. No user-visible regressions in CLI behavior.
2. Keep backward compatibility during migration with aliasing/shims.
3. Land changes in small phases with green CI at each phase.
4. Do not rename everything in one PR.

## Phased Execution

### Phase 0: Baseline + Freeze

- Record current passing baseline:
  - `just fmt`
  - `cargo test -p codex-cli`
  - `cargo test -p codex-core --test all aeye_`
- Freeze feature work touching workspace/package wiring while rename is in progress.
- Add a migration tracking issue/checklist and owner per phase.

### Phase 1: Compatibility Surface (No Breakage)

- Keep both binaries available:
  - `a-eye` (primary)
  - `codex` (compatibility)
- Keep command compatibility:
  - `a-eye ...` (primary)
  - `codex a-eye ...` (compatibility)
- Keep artifact/config compatibility:
  - `a-eye.yaml` (primary), `aeye.yaml` fallback
  - Existing `.nlpg` layout unchanged

### Phase 2: Workspace Alias Layer

- Introduce target package names while keeping legacy dependency aliases temporarily.
- Pattern for transition:

```toml
# New package name (target)
aeye-core = { path = "core" }

# Temporary compatibility alias for downstream crates not yet migrated
codex-core = { package = "aeye-core", path = "core" }
```

- Migrate leaf crates first, then central crates (`core`, `common`, `protocol`, `cli`).

#### Phase 2 Status (started)

- Added workspace alias keys for pilot migration:
  - `aeye-arg0`, `aeye-cli`, `aeye-common`, `aeye-core`, `aeye-protocol`, and `aeye-utils-*` keys for migrated utility crates.
- Completed leaf utility package rename batch:
  - `codex-rs/utils/{absolute-path,cache,cargo-bin,home-dir,image,json-to-toml,pty,readiness,string}` package names were migrated to `aeye-utils-*`.
  - Legacy aliases retained in workspace dependencies:
    - `codex-utils-* = { package = "aeye-utils-*", ... }`.
- Completed `arg0` package rename:
  - `codex-rs/arg0` package renamed from `codex-arg0` -> `aeye-arg0`.
  - Legacy alias retained:
    - `codex-arg0 = { package = "aeye-arg0", ... }`.
- Completed `common` package rename:
  - `codex-rs/common` package renamed from `codex-common` -> `aeye-common`.
  - Legacy alias retained:
    - `codex-common = { package = "aeye-common", ... }`.
- Completed `protocol` package rename:
  - `codex-rs/protocol` package renamed from `codex-protocol` -> `aeye-protocol`.
  - Legacy alias retained:
    - `codex-protocol = { package = "aeye-protocol", ... }`.

### Phase 3: Module/Import Migration

- Replace internal imports incrementally (`codex_*` -> `aeye_*`) crate-by-crate.
- Update tests, harnesses, and scripts in the same phase as the crate migration.
- Keep old aliases until the entire dependency graph is clean.

### Phase 4: Package/Path Rename Completion

- Remove temporary `codex-*` aliases from workspace dependencies.
- Rename remaining package names/paths to target convention.
- Keep only runtime user-facing compatibility shims if needed.

### Phase 5: Cleanup + Lock

- Remove final legacy references in docs/help text where safe.
- Add CI guard preventing new `codex-*` names in user-facing surfaces.
- Tag first “fully rebranded internal” release.

## Migration Map (Initial)

| Current package | Target package | Phase | Compatibility strategy |
|---|---|---|---|
| `codex-cli` | `aeye-cli` | 2-4 | Keep `codex` binary as compatibility entrypoint |
| `codex-core` | `aeye-core` | 2-4 | Use Cargo package alias during transition |
| `codex-common` | `aeye-common` | 2-4 | Alias + incremental import migration |
| `codex-protocol` | `aeye-protocol` | 2-4 | Alias + snapshot/test updates |
| `codex-exec` | `aeye-exec` | 3-4 | Migrate after `core/common` |
| `codex-tui` | `aeye-tui` | 3-4 | Migrate with snapshot batch |
| `codex-arg0` | `aeye-arg0` | 2-4 | Keep helper behavior unchanged |
| `codex-apply-patch` | `aeye-apply-patch` | 3-4 | Preserve `apply_patch` helper name |
| `codex-linux-sandbox` | `aeye-linux-sandbox` | 4-5 | Keep sandbox interface compatibility |
| `codex-login` | `aeye-login` | 3-4 | Migrate with auth command paths |
| `codex-chatgpt` | `aeye-chatgpt` | 3-4 | Migrate with API/auth dependencies |
| `codex-mcp-server` | `aeye-mcp-server` | 3-4 | Migrate after protocol layer |
| `codex-rmcp-client` | `aeye-rmcp-client` | 3-4 | Migrate with MCP stack |
| `codex-app-server` | `aeye-app-server` | 4 | Migrate after CLI stabilization |
| `codex-cloud-tasks` | `aeye-cloud-tasks` | 4 | Migrate after core/auth/protocol |
| `codex-utils-*` | `aeye-utils-*` | 2-4 | Batch by utility domain |

## Validation Gate Per Phase

- `just fmt`
- `just fix -p codex-cli` (or migrated equivalent crate)
- `cargo test -p codex-cli`
- `cargo test -p codex-core --test all aeye_`
- Smoke:
  - `cargo run --bin a-eye -- status`
  - compatibility check: `cargo run --bin codex -- a-eye status`

## Risk Register

- **Risk:** workspace-wide rename breaks dependency graph.
  - **Mitigation:** alias-first strategy + crate-by-crate migration.
- **Risk:** scripts/tests still refer to old binary/package names.
  - **Mitigation:** dual-binary period + CI grep checks.
- **Risk:** external tooling expects `codex` paths/config.
  - **Mitigation:** retain compatibility shim for at least one release cycle.

## Exit Criteria

- No internal package names start with `codex-` (except explicitly retained compatibility shims).
- No user-facing docs/help recommend legacy commands.
- Full CI and A-Eye integration tests pass.
