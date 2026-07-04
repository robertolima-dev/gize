# ADR-000: ADR process and template

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Gize's guiding rule is "analyze before implementing". We need a lightweight, consistent
way to record architectural decisions so that trade-offs are explicit and future
contributors understand *why*, not just *what*.

## Decision

Every significant decision is recorded as a numbered ADR in `/ADR`, named
`adr-NNN-slug.md`. Each ADR uses the template below and moves through the statuses:
`Proposed → Accepted → (Superseded by ADR-XXX | Deprecated)`.

An ADR is required before implementing anything it governs. Small, reversible choices do
not need an ADR.

## Template

```
# ADR-NNN: <title>
- Status: Proposed | Accepted | Superseded by ADR-XXX | Deprecated
- Date: YYYY-MM-DD
- Deciders: ...

## Context
Why is a decision needed? Forces at play.

## Alternatives
Options considered, honestly.

## Decision
What we chose.

## Trade-offs
What we give up; what we gain.

## Consequences
Future implications, follow-ups, risks introduced.
```

## Trade-offs

Lightweight process adds small overhead per decision but pays off in shared context and
avoids re-litigating settled choices.

## Consequences

- `/ADR` is the source of truth for architecture rationale.
- Backlog implementation tasks reference the ADR that unblocks them.
