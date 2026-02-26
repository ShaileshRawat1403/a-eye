# The i7 Framework

## A Deterministic Model for Agentic Workflows

The **i7 framework** is a deterministic orchestration model for AI-assisted workflows.

It does not try to make agents "smarter". It makes execution safer, auditable, resumable, and human-aligned.

In A-Eye, i7 is the orchestration framework and DAO-style components provide the engine implementation.

## Why i7 exists

Most agent systems fail in practice because they are:

- opaque
- non-deterministic
- hard to pause, audit, or resume
- unsafe in real environments

i7 is designed to solve those reliability issues first.

## The i7 loop

1. Intent
2. Inspect
3. Interpret
4. Isolate
5. Implement
6. Inspect (Verify)
7. Integrate

Each phase is observable, interruptible, and replayable.

## Phase breakdown

### 1) Intent

What is being attempted, and why?

- intent is made explicit
- no execution
- ambiguity is reduced early

### 2) Inspect

What is the current state?

- repository and context inspection
- no mutation
- produces a **system artifact**

### 3) Interpret

What should change?

- plan generation from intent + context
- may include alternatives
- produces a **plan artifact**

### 4) Isolate

What exactly will change?

- constrained diff computation
- scope reduction
- produces a **diff artifact**

### 5) Implement

Should the change be applied?

- risk classification
- policy evaluation
- approval gating where required

### 6) Inspect (Verify)

Did the change do what was intended?

- explicit verification
- failures are surfaced and recorded
- produces a **verify artifact**

### 7) Integrate

What is the final outcome?

- finalize state
- persist logs and artifacts
- allow deterministic replay and resume

## What makes i7 different

i7 is:

- **deterministic**: same inputs, same decisions, same outcomes
- **artifact-first**: every phase yields explicit state artifacts
- **interruptible**: workflows can stop safely
- **replayable**: runs can be reconstructed
- **human-aligned**: approvals are first-class, not bolted on

i7 is not a prompt style. It is an execution model.

## How i7 appears in A-Eye

A-Eye operationalizes i7 through:

- typed workflow artifacts (`system`, `plan`, `diff`, `verify`, `logs`)
- policy-gated execution and approvals
- event persistence, snapshots, replay, and resume semantics

You should not need to learn i7 vocabulary to use A-Eye. You should feel it through predictable and safe behavior.
