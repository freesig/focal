---
name: spec-priority
description: Prioritize a specification, requirements document, design note, or selected spec excerpt for human review before implementation, present only the single highest-priority review item at a time, collect the user's decisions, and update the spec when review is done. Use when Codex is given a spec or part of a spec and asked to identify which decisions, requirements, ambiguities, or open questions should be reviewed first, especially items likely to cause implementation rework if reviewed late.
---

# Spec Priority

## Purpose

Turn the provided spec scope into concrete reviewable items, internally rank them, and present only the single highest-priority item for human review. Prioritize items whose late clarification or change is highly likely to force implementation rework and where the answer would produce meaningfully different software. Continue one item at a time until the user says they are done, then update the spec from those decisions.

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

3. Filter aggressively for high-value human review.
   - Ask only about items where two plausible answers would create meaningfully different software, user experience, data shape, API behavior, permissions, integrations, operational behavior, acceptance criteria, or test obligations.
   - Skip items whose answer is obvious from the surrounding spec, established product conventions, platform norms, or ordinary engineering judgment.
   - Skip items the user is unlikely to care about because the choice has no material effect on what gets built, how it behaves, or what must be validated.
   - Skip reversible, low-cost implementation choices that an engineer can safely decide during implementation.
   - If an item is merely unclear but the likely answer can be inferred and late correction would be cheap, do not ask it.
   - Before keeping a candidate, check: "Would a human answer change the software enough to matter?" If not, omit it.

4. Score each remaining item using these dimensions:
   - `Change likelihood`: How likely human review now is to change or clarify the spec.
   - `Rework avoidance`: How much implementation work would be wasted, destabilized, or repeated if this item changes after implementation starts.
   - `Outcome importance`: How much the item affects what users, operators, or maintainers will actually care about in the finished implementation.

5. Choose the next review item internally.
   - Treat rework avoidance as the dominant priority signal: an item that is likely to cause substantial redesign, data/model changes, API churn, migration work, UX restructuring, integration changes, permission/security changes, or broad test rewrites if decided late belongs near the top.
   - Highest priority items usually combine high rework risk with uncertainty, stakeholder sensitivity, or outcome importance.
   - Promote foundational decisions that constrain many later choices, even when the spec seems confident.
   - Break ties by dependency order: review decisions that constrain many later choices first.
   - Demote items that are uncertain but low-impact, or important but already obvious from the surrounding spec.

6. Present only the current highest-priority item.
   - Keep all other candidates private.
   - Lead with the single highest-priority concrete decision, requirement, ambiguity, or open question chosen from the scope.
   - Explain why this item should be reviewed now, especially what implementation rework is likely if it is reviewed later.
   - State the specific human decision or clarification needed.
   - Ask the user to answer this item or say `done` when they want the spec updated.
   - If no candidate survives the high-value filter, say that no review item appears worth interrupting for and ask whether to update the spec or stop.

7. Accept review decisions over subsequent turns.
   - Treat the user's follow-up messages as decisions, clarifications, accept/reject choices, wording, deferrals, or new review concerns unless they clearly redirect the task.
   - Maintain a running decision log mapped to the chosen item, source, and intended spec change.
   - Acknowledge captured decisions briefly, then present the next highest-priority remaining item using the same one-item format.
   - Re-filter remaining candidates after each decision; user answers may make lower-priority items obvious or irrelevant.
   - If the next remaining item is low-impact, obvious, or unlikely to change the software, skip it instead of asking.
   - Do not edit the spec while review is still in progress unless the user explicitly asks for an immediate update.

8. When the user says they are done, update the spec.
   - Apply only resolved decisions and clarifications to the relevant spec file or section.
   - Preserve unrelated content, structure, and wording style.
   - If the source was pasted text or an unavailable file, output a revised excerpt or patch instead of claiming to edit a file.
   - If a decision is contradictory or too incomplete to update safely, ask one concise blocking question or leave it as an explicit open question in the spec.

## Output Format

Use this exact single-item structure unless the user requests another format. Populate it with exactly one selected review item and keep all scoring and other candidates private.

```markdown
## Next Priority Item

**Decision:** Concrete item from the spec

**Source:** File/heading/bullet/short phrase

**Why review now:** Late review would likely cause specific implementation rework.

**Needed:** Specific decision or clarification needed.

Please answer this item, or say `done` when you want me to update the spec from the decisions so far.
```

After each answer, continue the review loop instead of treating the task as complete: capture the decision, choose the next highest-priority item that still survives the high-value filter, and present only that one item. For very small specs, use a compact version of the same one-item format. For large specs, still show only one item at a time; do not mention the remaining candidate count unless the user asks.
