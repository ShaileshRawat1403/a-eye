# i7 to DAO Mapping

This document maps the i7 framework to DAO-style deterministic orchestration components used by A-Eye.

It is intended for maintainers, auditors, and enterprise adopters.

## High-level mapping

| i7 Phase | DAO Responsibility | Artifact or Mechanism |
| --- | --- | --- |
| Intent | Workflow initialization | `WorkflowRunStarted` |
| Inspect | System scanning | `system` artifact |
| Interpret | Planning | `plan` artifact |
| Isolate | Diff computation | `diff` artifact |
| Implement | Policy and approval gating | `ApprovalRequest` / `ApprovalResolved` |
| Inspect (Verify) | Verification | `verify` artifact |
| Integrate | Finalization and persistence | `WorkflowStatusChanged` |

## Artifact model

DAO orchestration is **artifact-first**.

| Artifact | Purpose |
| --- | --- |
| `system` | observed repository and environment state |
| `plan` | intended work breakdown |
| `diff` | exact proposed modifications |
| `verify` | post-change checks and outcomes |
| `logs` | deterministic operational/audit timeline |

Artifacts are correlated by run and invocation identifiers and guarded against stale or out-of-order updates.

## Determinism guarantees

The model enforces determinism with:

- monotonic `run_id`
- monotonic `invocation_id`
- monotonic event sequence (`seq`)
- append-only event logs
- snapshot + bounded replay
- stale update rejection

Given the same event stream, replay reconstructs the same state.

## Approval model in i7 terms

In i7, Implement means authorization and controlled execution, not unchecked mutation.

DAO maps that as:

1. risk classification
2. policy tier evaluation
3. approval requirement resolution
4. execution only after required authorization

## Replay and resume semantics

Persistence keeps workflows safe and recoverable:

- events are append-only and ordered
- snapshots accelerate restoration
- replay applies only the post-snapshot tail
- resume requires explicit user action for interrupted runs

No implicit unsafe continuation occurs on restore.

## Host boundaries

- DAO core orchestration is UI-agnostic and deterministic.
- A-Eye provides the host UX (CLI/TUI), guidance, and user interaction.

This keeps orchestration correctness stable across hosts.
