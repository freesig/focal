---
name: spec-priority
description: Prioritize a specification, requirements document, design note, or selected spec excerpt for human review before implementation, collect the user's review decisions, and update the spec when review is done. Use when Codex is given a spec or part of a spec and asked to identify which decisions, requirements, ambiguities, or open questions should be reviewed first, especially items likely to cause implementation rework if reviewed late.
---

# Spec Priority

## Purpose

Turn the provided spec scope into concrete reviewable items, choose the highest-priority items, and output those items for human review. Prioritize items whose late clarification or change is highly likely to force implementation rework. Continue accepting review decisions until the user says they are done, then update the spec from those decisions.

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
   - `Rework avoidance`: How much implementation work would be wasted, destabilized, or repeated if this item changes after implementation starts.
   - `Outcome importance`: How much the item affects what users, operators, or maintainers will actually care about in the finished implementation.

4. Rank by review priority.
   - Treat rework avoidance as the dominant priority signal: an item that is likely to cause substantial redesign, data/model changes, API churn, migration work, UX restructuring, integration changes, permission/security changes, or broad test rewrites if decided late belongs near the top.
   - Highest priority items usually combine high rework risk with uncertainty, stakeholder sensitivity, or outcome importance.
   - Promote foundational decisions that constrain many later choices, even when the spec seems confident.
   - Break ties by dependency order: review decisions that constrain many later choices first.
   - Demote items that are uncertain but low-impact, or important but already obvious from the surrounding spec.

5. Produce a concise prioritized review list.
   - Lead with the actual highest-priority items chosen from the scope, not a full spec summary, generic categories, or an empty table template.
   - Each item must name the concrete decision, requirement, ambiguity, or open question to review.
   - Explain why each item should be reviewed now, especially what implementation rework is likely if it is reviewed later, and what human decision or clarification is needed.
   - Include enough lower-priority items to show coverage, but keep the focus on avoiding rework.
   - Ask the user to review items by rank or source reference and to say `done` when they want the spec updated.

6. Accept review decisions over subsequent turns.
   - Treat the user's follow-up messages as decisions, clarifications, accept/reject choices, wording, deferrals, or new review concerns unless they clearly redirect the task.
   - Maintain a running decision log mapped to the ranked item, source, and intended spec change.
   - Acknowledge captured decisions briefly and ask for the next decision when review is still in progress.
   - Do not edit the spec while review is still in progress unless the user explicitly asks for an immediate update.

7. When the user says they are done, update the spec.
   - Apply only resolved decisions and clarifications to the relevant spec file or section.
   - Preserve unrelated content, structure, and wording style.
   - If the source was pasted text or an unavailable file, output a revised excerpt or patch instead of claiming to edit a file.
   - If a decision is contradictory or too incomplete to update safely, ask one concise blocking question or leave it as an explicit open question in the spec.

## Output Format

Use this structure unless the user requests another format. Populate it with the selected review items; do not output only the schema, scoring rubric, or placeholder rows.

```markdown
## Priority Review Items

| Rank | Item | Source | Why review now | Human decision needed |
| --- | --- | --- | --- | --- |
| 1 | Concrete item from the spec | File/heading/bullet/short phrase | Late review would likely cause specific implementation rework | Specific decision or clarification needed |

## Scoring Notes

- Change likelihood: High/Medium/Low based on ambiguity, missing choices, conflicts, or stakeholder sensitivity.
- Rework avoidance: High/Medium/Low based on architecture, data model, API, workflow, migration, UX, security, integration, or testing impact.
- Outcome importance: High/Medium/Low based on user-visible behavior, correctness, trust, operational burden, or maintainability.

## Lower Priority / Probably Safe

- Concrete lower-priority item from the spec
```

After the initial list, continue the review loop instead of treating the task as complete. For very small specs, use bullets instead of a table, but still output the concrete highest-priority review items. For large specs, show the top 5-10 items first and mention how many lower-priority items were found.
