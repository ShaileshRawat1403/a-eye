# A-Eye Agent Safety Contract

This document outlines the safety contract and operational constraints for any AI agent, including A-Eye, operating within this repository. Adherence to these rules is mandatory to ensure safety, predictability, and user control.

## Core Principles

- **Human-in-the-Loop is Mandatory**: No action that modifies files or executes system commands shall be performed without explicit, logged user approval for that specific action. There is no autonomous execution mode.
- **Deterministic by Default**: Agents must prefer deterministic, recipe-driven workflows over open-ended, non-deterministic reasoning.
- **Least Privilege**: Agents must operate with the minimum permissions necessary to complete a task. The default operational tier is always the most restrictive one.
- **Transparency and Auditability**: All significant actions, decisions, and generated artifacts (plans, patches, summaries) must be logged to a structured, auditable artifact store.

## Operational Constraints

1.  **No Direct Execution**: Agents must not directly execute shell commands or apply file patches. These actions must be gated by the Policy Engine, which requires user approval based on tiered permissions.
2.  **Scoped Modifications**: File modifications must be minimal, targeted, and directly related to the approved plan. Unrelated refactoring or "cleanup" is strictly forbidden.
3.  **Secret Handling**: Agents must never access, store, or exfiltrate secrets (API keys, passwords, tokens). All logs and artifacts must be scanned and redacted for secret patterns.
4.  **Grounded Awareness**: Architectural and system facts must be derived from explicit system scans (`a-eye scan`). Agents must not hallucinate or assume details about the environment.
5.  **Test-Driven Changes**: All new features or modifications to agent behavior must be accompanied by tests that validate correctness and enforce safety constraints (e.g., tier gating, approval flows).
6.  **User Veto**: The user always has the final say. Any proposed plan or action can be vetoed, and the agent must halt that line of work.

_This contract applies to all contributors and automated systems working on this codebase._
