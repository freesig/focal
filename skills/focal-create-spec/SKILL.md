---
name: focal-create-spec
description: Create or extend a Focal MCP idea graph/spec from an initial idea, pasted notes, a local file path, requirements sketch, design prompt, or similarly messy source. Use when Codex should ask only essential clarifying questions, store the source as Focal context, create root spec nodes for the idea's major components, then recursively add non-obvious statement or question-answer child nodes to the depth the user chooses.
---

# Focal Create Spec

## Purpose

Turn a rough idea into a Focal spec graph. Preserve the original idea as context, decompose it into root nodes, then deepen only the parts where additional non-obvious statements or question-answer nodes are revealed by their parent.

## Input Handling

- Treat the user's argument as the idea source. It may be pasted text, a local path, a named file, a section, an existing graph/node reference, or a short prompt.
- If the source names a local file or section, read only the relevant scope unless the user asks for the whole file.
- If adding to an existing Focal spec, inspect current roots and relevant descendants before writing so new nodes extend the graph instead of duplicating it.
- Ask clarifying questions only when missing information is likely to make the generated spec materially unaligned with the user's intent. Ask at most 3 questions before creating nodes.
- Skip clarifying questions when a reasonable assumption is low-risk; state the assumption in the context document instead.

Good clarifying questions target foundational choices such as audience, boundary of scope, primary workflow, source of truth, permission model, data ownership, integration boundary, success criteria, or a domain rule that would reshape many nodes. Do not ask generic questions about implementation preferences, wording, priorities, or edge cases unless the answer would change the graph structure or core behavior.

## Context Document

Before creating spec nodes, call `focal_add_context_document` with the original idea and any clarifying answers captured so far.

- Title the document after the idea and source, such as `Initial idea: <short name>` or `Added idea: <short name>`.
- Include source metadata when available: path, heading, date, and any assumptions made instead of asking.
- Preserve messy notes rather than rewriting them into polished spec language. The context document is provenance for later graph work.

## Root Generation

Create root nodes with `focal_add_root_node`, or attach to existing relevant nodes when the user is explicitly extending part of an existing graph.

Roots are the main components of the idea, not a table of contents. Use them to pull the idea apart into major features, product areas, actors, workflows, interfaces, data domains, constraints, or operating principles.

For each candidate root, apply this filter:

- Keep it if removing the root would lose a material component of the idea.
- Keep it if it defines a direction that later child nodes can explore.
- Skip it if it merely repeats the project name, restates an obvious platform default, or would contain only obvious children.
- Merge candidates that would lead to the same child questions or decisions.

Prefer `statement` roots for settled, non-obvious claims or chosen directions. Use `qa` roots only for foundational open decisions where the answer is not obvious and different answers would reshape the spec. If the active Focal MCP schema exposes structured content, use:

```json
{ "type": "statement", "body": "..." }
```

or:

```json
{ "type": "qa", "question": "...", "answer": "...", "alternative_answers": [] }
```

If the active tool schema exposes `content` as a string, still set `kind` to `statement` or `qa` and format the content clearly with statement body text or `Question`, `Answer`, and optional `Alternative answers` sections.

After adding roots, show the created root titles and ask how deep to go unless the user already specified a depth.

## Depth Prompt

Ask for a concrete expansion depth:

- `0`: roots only.
- `1`: add direct children under roots.
- `2`: add grandchildren as well.
- `until saturated`: continue until no non-obvious children remain, with a default cap of 3 levels unless the user explicitly permits more.

If the user gives a vague answer such as "medium" or "reasonable", choose depth `1` for small ideas and depth `2` for broad ideas, then state the choice before expanding.

## Recursive Expansion

For each parent at the current frontier, generate child candidates that are revealed by that parent. A child should explore a consequence, decision, constraint, exception, data shape, user-facing behavior, operational need, or unresolved question implied by the parent.

Use `focal_add_child_node` for candidates that pass the non-obviousness filter. It is correct to add no children for a parent when all possible children are obvious.

Statement nodes should capture non-obvious claims, constraints, decisions, invariants, or consequences. They should not say things that are standard, tautological, or already contained in the parent.

Question-answer nodes should capture non-obvious questions where:

- two or more plausible answers would produce meaningfully different software, behavior, data, workflows, or tests;
- the answer is supplied by the idea, a clarifying response, or a defensible inference from the parent; and
- the question is worth preserving because later implementation could otherwise drift.

If the answer is genuinely unknown but the question is important, create the QA node with an empty or explicitly unresolved answer only if the Focal schema allows it. Do not invent certainty. Use `alternative_answers` only for realistic alternatives that are present or strongly implied.

## Non-Obviousness Filter

Before creating any node, privately test it:

- Would a competent implementer already assume this from the parent and surrounding graph?
- Would removing it change what gets built, validated, or debated?
- Does it introduce a distinct decision, constraint, domain rule, interface, workflow, or risk?
- Is it revealed by the parent rather than merely adjacent to it?
- Is it more useful as a child of this parent than as a sibling/root elsewhere?

Create the node only when the answers show real information value. When in doubt, skip the node or combine it with a stronger sibling.

Avoid these low-value nodes:

- "The system should be user friendly."
- "The app needs authentication" when authentication is ordinary and not otherwise special.
- "The feature should handle errors" without a specific non-obvious error behavior.
- "What tech stack should be used?" unless the stack choice is central to the product contract.
- Rewordings of the parent as smaller bullets.

## Adding To Existing Graphs

When extending an existing spec:

- Use `focal_list_roots`, `focal_read_node`, `focal_list_children`, or `focal_list_descendants` to find the relevant attachment points.
- Add a new context document for the new source material; do not overwrite earlier context unless the user asks.
- Reuse existing nodes by adding children beneath them when the new idea deepens an existing direction.
- Add new roots only for major components not already represented.
- Avoid changing or deleting existing nodes during creation unless the user explicitly asks for an update.

## Completion

After expansion, summarize what was added: context document title, root titles, depth used, and any parents where no children were added because every candidate was obvious. Keep the summary brief and do not dump the whole graph unless asked.
