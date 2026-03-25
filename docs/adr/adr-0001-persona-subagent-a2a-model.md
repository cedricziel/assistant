# ADR-0001: Persona, Subagent Process, and A2A Profile Model

- Status: Accepted
- Date: 2026-03-25
- Decision Makers: Core maintainers

## Context

The project currently uses "agent" in multiple ways:

1. Assistant context/persona (long-lived identity with context files such as `SOUL.md` and `IDENTITY.md`)
2. Delegated subagent execution (`agent-spawn`, `agent-status`, `agent-terminate`)
3. A2A agent card and related metadata for external discovery

This naming overlap creates ambiguity in architecture, UX, and docs. It also makes it harder to reason about inheritance and runtime behavior when delegated work is bound to another context.

## Decision

We standardize the domain model and terminology as follows:

### 1) Persona (formerly "assistant context")

A Persona is the primary long-lived assistant identity.

- Has its own memory bundle and identity files (`SOUL.md`, `IDENTITY.md`, `USER.md`, etc.)
- Has its own workspace and storage scope
- Is the root owner of conversations and delegated work

### 2) Subagent Process

A Subagent Process is an ephemeral delegated execution unit spawned by a Persona.

- Exists to complete a specific delegated task
- Has its own lifecycle (running, completed, failed, cancelled)
- Can be:
  - anonymous (task-scoped, minimal inherited context)
  - persona-bound (delegated as another Persona)

### 3) A2A Profile (formerly treated as a separate "agent" concept)

An A2A Profile is the external protocol contract for a Persona.

- Includes discovery metadata (card), capabilities, bindings, and auth requirements
- Is attached to a Persona (optional), not a parallel root entity

## Inheritance Policy

Persona-bound subagent processes inherit the full bound Persona context by default.

This includes:

- persona memory/identity corpus
- tool policy and trust settings
- model/provider defaults
- workspace and storage scope

Parents may still narrow delegated execution (for example, with an explicit tool allowlist) on a per-task basis.

Anonymous subagent processes do not implicitly inherit a full persona context.

## Consequences

### Positive

- Clear and consistent vocabulary across runtime, storage, and UI
- Cleaner mental model: Persona is the root; subagents are execution processes; A2A is an interface contract
- Better alignment of delegated behavior with user expectation

### Trade-offs

- Existing naming in code and schema may remain temporarily inconsistent during migration
- Documentation and UX strings need coordinated updates

## Implementation Guidance

1. Use the words Persona, Subagent Process, and A2A Profile in all new docs and UI copy.
2. Keep backward compatibility for existing CLI/tool names where necessary, but document aliases and preferred terms.
3. Align data ownership rules so subagent processes are always attributable to a parent Persona.
4. Treat A2A card/profile configuration as Persona-scoped configuration.

## Migration Plan

1. Documentation and UI terminology pass
2. Runtime behavior alignment (persona binding and lineage semantics)
3. Storage/API naming convergence (with compatibility shims where needed)
4. A2A integration as Persona-attached profile

## Non-Goals

- Renaming every existing identifier immediately
- Breaking CLI compatibility in a single release
