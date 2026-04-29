---
name: spec-priority
description: Prioritize a specification, requirements document, design note, or selected spec excerpt for human review before implementation. Use when Codex is given a spec or part of a spec and asked to identify which decisions, requirements, ambiguities, or open questions should be reviewed first to reduce implementation rework.
---

# Spec Priority

## Purpose

Turn the provided spec scope into reviewable items and rank the items by how much human review now is likely to change the spec and prevent implementation rework.

## Input Handling

- Treat the user's argument as the review scope. It may be pasted text, a file path, a section name, or a description of part of a spec.
- If the argument names a file or section available in the workspace, read only the relevant scope unless the user asks for the whole spec.
- If the scope is unclear but an obvious spec file exists, state the assumption and proceed. Ask a concise question only when choosing the wrong scope would make the result misleading.
- Preserve source references when possible: file path, heading, bullet label, or quoted short phrase.

## Workflow

1. Break the scope into reviewable items.
   - Prefer decisions, requirements, behaviors, constraints, user flows, interfaces, data shapes, states, permissions, errors, non-goals, and acceptance criteria.
   - Split compound requirements when different parts could change independently.
   - Merge duplicates or near-duplicates that would receive the same human decision.

2. Identify uncertainty and non-obvious choices.
   - Look for vague terms, missing defaults, undefined edge cases, implicit product decisions, conflicting statements, surprising constraints, hidden assumptions, and multiple plausible implementations.
   - Include items where the spec sounds decisive but the decision is unusually consequential or likely to be challenged by stakeholders.
   - Do not over-prioritize grammar issues, wording polish, or obvious implementation details unless they change behavior.

3. Score each item using these dimensions:
   - `Change likelihood`: How likely human review now is to change or clarify the spec.
   - `Rework avoidance`: How much implementation work would be wasted or destabilized if this item changes later.
   - `Outcome importance`: How much the item affects what users, operators, or maintainers will actually care about in the finished implementation.

4. Rank by review priority.
   - Highest priority items combine high uncertainty, high rework risk, and high outcome importance.
   - Break ties by dependency order: review decisions that constrain many later choices first.
   - Demote items that are uncertain but low-impact, or important but already obvious from the surrounding spec.

5. Produce a concise prioritized review list.
   - Lead with the top items, not a full spec summary.
   - Explain why each item should be reviewed now and what human decision or clarification is needed.
   - Include enough lower-priority items to show coverage, but keep the focus on avoiding rework.

## Output Format

Use this structure unless the user requests another format:

```markdown
## Priority Review Items

| Rank | Item | Source | Why review now | Human decision needed |
| --- | --- | --- | --- | --- |
| 1 | ... | ... | ... | ... |

## Scoring Notes

- Change likelihood: High/Medium/Low based on ambiguity, missing choices, conflicts, or stakeholder sensitivity.
- Rework avoidance: High/Medium/Low based on architecture, data model, API, workflow, migration, UX, security, integration, or testing impact.
- Outcome importance: High/Medium/Low based on user-visible behavior, correctness, trust, operational burden, or maintainability.

## Lower Priority / Probably Safe

- ...
```

For very small specs, use bullets instead of a table. For large specs, show the top 5-10 items first and mention how many lower-priority items were found.
