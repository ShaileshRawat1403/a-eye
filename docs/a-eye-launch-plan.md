# A-Eye Launch Plan

## Goal

Launch A-Eye as a universal CLI agent for non-developers, with deterministic workflows, explicit approvals, and auditable artifacts.

## Non-Negotiables

1. Do not break existing base CLI/core flows while migrating.
2. No autonomous writes/shell execution.
3. Tiered policy gates + explicit approvals.
4. Scan-grounded context only.
5. Structured artifacts for audit/replay.
6. Default safe mode (Tier 1).
7. Secret redaction in logs/artifacts.

## As-Is

- `a-eye` runs as a standalone binary in this repository workspace.
- Core commands exist (`scan/plan/patch/apply/verify/explain/learn/run/status`).
- Basic tests pass for A-Eye flow.
- Core architecture is still embedded in the monorepo workspace.

## To-Be

- Standalone `a-eye` binary with provider-agnostic runtime.
- A-Eye integration becomes optional adapter (not product core).
- Full policy + artifact + replay guarantees.

## Phased Scope

For crate/module/package renaming sequence, see `docs/a-eye-internal-rename-plan.md`.

### Phase 1: Product Hardening (in current fork)

- Centralize policy checks for all writes/shell calls.
- Enforce schema validation for intent/plan/patch/system/run artifacts.
- Add secret redaction before any artifact/log persistence.
- Improve scanner quality and risk-zone detection.
- Freeze deterministic workflow contracts.

### Phase 2: Universal Core Extraction

- Extract reusable crates/modules:
  - `a-eye-core` (contracts, workflows, artifacts)
  - `a-eye-policy` (tiers, approvals, redaction)
  - `a-eye-runtime` (execution engine)
  - `a-eye-providers` (LLM provider adapters)
- Keep compatibility adapters only where needed during extraction.

### Phase 3: Standalone CLI Launch

- Ship standalone `a-eye` binary.
- Backward-compatible config/artifact loading.
- Release packaging + platform binaries.

## Launch Readiness Checklist

- [ ] Tier-1 safe mode validated end-to-end.
- [ ] Tier-2 approval prompts tested for every write/shell action.
- [ ] Artifact schemas validated on write/read.
- [ ] Redaction test suite for common secret patterns.
- [ ] Deterministic workflow replay passes.
- [ ] User docs rewritten for non-dev usage.
- [ ] Install and onboarding tested on macOS/Linux/Windows.
- [ ] Rollback plan documented.

## Branding Migration Plan

### Now

- User-facing name: **A-Eye**.
- Command surface: `a-eye` (alias `aeye`).
- Config primary file: `a-eye.yaml` (fallback `aeye.yaml`).

### Next

- Remove remaining legacy branding in docs/help.
- Keep internal crate names stable until extraction is complete.
- After standalone split, retire legacy internal naming incrementally.
