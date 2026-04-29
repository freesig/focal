---
name: align-spec
description: Compare a provided spec, requirements document, design note, or selected spec excerpt against the current codebase, find behavior that is diverged, missing, stale, or only partially implemented, prioritize those alignment items by the implementation rework risk of reviewing them late, present one item at a time for human decision, record responses until the user says done, then update the code and/or spec to match the resolved decisions. Use when Codex is asked to align specs with implementation, audit code against requirements, reconcile docs and code, or identify partial implementations before continuing development.
---

# Align Spec

## Purpose

Use the provided spec scope and the current codebase to find meaningful misalignment between intended behavior and implemented behavior. Prioritize alignment questions whose late review is most likely to cause rework, present only one item at a time, collect the user's decisions, then apply the resolved changes to code, tests, or the spec when the user says `done`.

## Input Handling

- Treat the user's argument as the spec scope. It may be pasted text, a file path, a section name, an issue description, or a reference to an existing requirements document.
- If the argument names a file or section available in the workspace, read only the relevant scope unless the user asks for the whole spec.
- If the scope is unclear but an obvious spec file exists, state the assumption and proceed. Ask a concise question only when choosing the wrong scope would make the review misleading.
- Preserve source references for every candidate: spec path and heading, short quoted phrase, code path and symbol, test path, route, schema, or UI surface.
- If no local codebase is available, perform the spec-side review anyway and clearly state that code alignment could not be verified.

## Code Investigation

1. Locate implementation surfaces implied by the spec.
   - Search for named features, routes, commands, APIs, models, database tables, config keys, UI labels, events, background jobs, permissions, and acceptance criteria.
   - Prefer `rg`, `rg --files`, project manifests, router definitions, test names, and existing architecture boundaries over broad manual browsing.
   - Read tests as implementation evidence, but do not treat tests alone as proof unless the behavior is also implemented or intentionally mocked.

2. Build a private alignment map.
   - For each material spec requirement, classify the implementation as `implemented`, `missing`, `partial`, `diverged`, `obsolete spec`, `untested`, or `uncertain`.
   - Record the evidence that supports the classification. Keep enough detail to edit safely later.
   - Distinguish intended divergence from accidental drift when comments, migrations, feature flags, changelogs, or recent commits explain the difference.

3. Filter for review-worthy items.
   - Keep items where a human decision could change code behavior, data shape, public contract, migration path, UI flow, permissions, error handling, operational behavior, acceptance criteria, or test obligations.
   - Skip small naming mismatches, wording polish, internal refactor opportunities, or cheap implementation details unless they affect the delivered behavior or would mislead future work.
   - Skip gaps whose fix is obvious and low-risk; make those edits directly during the completion phase if they are clearly implied by resolved decisions.

## Prioritization

Score remaining candidates privately using these dimensions:

- `Rework avoidance`: How much code, data, tests, UI, integration work, or spec structure would need to be redone if this is clarified late.
- `Spec/code confidence gap`: How strongly the spec and code disagree, or how incomplete the implementation appears.
- `Decision likelihood`: How likely human review is to choose a non-obvious direction instead of the current code or current spec.
- `Outcome importance`: How much the item affects users, operators, maintainers, APIs, data integrity, security, permissions, or release readiness.

Treat rework avoidance as the dominant priority signal. Promote foundational decisions that constrain later implementation, especially data models, public APIs, permissions, migrations, state machines, routing, cross-service contracts, and major UX flows. Demote items that are uncertain but isolated, reversible, already obvious from surrounding context, or cheap to correct later.

## Review Loop

1. Present only the highest-priority item that survives the filter.
   - Keep the full candidate list and scoring private.
   - Be concrete about the spec claim, the observed code behavior, and the likely rework if review waits.
   - Ask for the specific decision needed: change code to match the spec, change the spec to match code, define a missing behavior, defer, or mark as intentionally out of scope.

2. Capture the user's response as a decision.
   - Treat follow-up messages as decisions, clarifications, accept/reject choices, requested edits, deferrals, or new alignment concerns unless they clearly redirect the task.
   - Maintain a private decision log with the selected item, source references, chosen direction, and intended code/spec/test changes.
   - Acknowledge the captured decision briefly, then present the next highest-priority remaining item.
   - Re-filter after each answer; resolved decisions may make lower-priority items obvious, irrelevant, or newly important.
   - Do not edit files during the review loop unless the user explicitly asks for an immediate change.

3. Stop asking when the user says `done`.
   - Treat `done`, `that's enough`, `stop`, or equivalent wording as the signal to apply decisions captured so far.
   - If no decisions have been captured, ask whether to stop without changes or perform a direct low-risk alignment pass.

## Completion Edits

When the user says they are done:

- Apply only resolved decisions and directly implied low-risk fixes.
- Update code when the user chose the spec as the source of truth, or when the implementation is plainly incomplete relative to an accepted requirement.
- Update the spec when the user chose current code behavior as the source of truth, when a requirement is intentionally deferred or out of scope, or when acceptance criteria need clarification.
- Update tests when behavior changes, when existing tests encode stale expectations, or when the main risk is an untested requirement.
- Preserve unrelated content, structure, and coding style. Do not bundle unrelated refactors with alignment edits.
- If a decision is contradictory, incomplete, or unsafe to apply, ask one concise blocking question or write it as an explicit open question in the spec instead of guessing.
- If the spec was pasted text or an unavailable external document, output a revised excerpt or patch rather than claiming to edit a file.
- After editing, summarize changed files and the decisions they implement. Run the most relevant available tests or explain why tests were not run.

## Output Format

Use this exact one-item structure during the review loop unless the user requests another format:

```markdown
## Next Alignment Item

**Spec:** File/heading/bullet/short phrase

**Code:** File/symbol/test/route or `No matching implementation found`

**Issue:** Missing, partial, diverged, obsolete spec, untested, or uncertain behavior

**Why review now:** Late review would likely cause specific implementation rework.

**Needed:** Specific decision: update code, update spec, define behavior, defer, or mark out of scope.

Please answer this item, or say `done` when you want me to apply the decisions captured so far.
```

For very small scopes, use a compact version of the same structure. For large specs, still show only one item at a time and do not mention the remaining candidate count unless the user asks.
