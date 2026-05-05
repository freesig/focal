---
id: ea586e62-3295-4767-8d67-6fa9309f53f4
title: Initial idea: Focal Idea Graph Rust Library Specification
created_at_unix: 1777858547
updated_at_unix: 1777858547
---

Source path: spec/SPEC.md
Workspace: /Users/freesig/work/focal
Captured on: 2026-05-04
Source heading: Focal Idea Graph Rust Library Specification

Assumptions:
- The entire 1,919-line `spec/SPEC.md` file is the original idea source for this graph.
- The active Focal graph had no existing roots, so this run seeds initial roots rather than extending an existing graph.
- The source file remains the exact source of implementation wording; this context document preserves the source metadata and section map used to generate the initial Focal nodes.

Section map from the original idea source:

- 1. Purpose
- 2. Goals
- 3. Non-Goals
- 4. Terminology
- 5. `focal-fs` Filesystem Layout
- 6. `focal-fs` Node Directory Naming
- 7. Node IDs
- 8. Markdown Format
  - Statement Node Markdown
  - Question-Answer Node Markdown
  - Original Idea Context Markdown
- 9. Rust Data Model
- 10. Public API
- 11. Delete and Unlink Modes
- 12. Symlink Semantics
- 13. Graph Shape and Cycles
- 14. Traversal
- 15. Editing Semantics
- 16. Add Semantics
- 17. Read Semantics
- 18. Limited Movement and Promotion
- 19. Indexing and Discovery
- 20. Error Handling
- 21. Atomicity and Consistency
- 22. Concurrency
- 23. Platform Requirements
- 24. Validation
- 25. Security and Safety
- 26. Testing Requirements
- `focal-core` Unified API Crate
- `focal-mcp` MCP Server Crate
  - MCP Tools
  - MCP Resources
  - MCP Prompts
  - MCP Safety
  - MCP Testing Requirements
- `focal-sqlite` SQLite Backend
- Spec Test Traceability
- 27. Example Usage
- 28. Acceptance Criteria

Original purpose summary from source:

Build small Rust libraries for storing and navigating an idea graph. Each graph can also store graph-level original idea context documents as editable Markdown records of the messy material that generated or later shaped the graph. The shared behavior is exposed through simple Rust functions for reading, adding, editing, linking, unlinking, deleting, and traversing nodes.

Specified crates:

- `focal-types`: common public data types.
- `focal-fs`: filesystem backend using folders, Markdown files, and symbolic links.
- `focal-sqlite`: SQLite backend using `rusqlite` and the same graph behavior.
- `focal-core`: unified public API over concrete backend enum dispatch.
- `focal-mcp`: MCP adapter exposing the same behavior as tools and resources for AI agents.

Core source constraints:

- Nodes are statement or question-answer entries with stable UUID identities.
- Question-answer nodes preserve primary answer and ordered alternative answers separately.
- Context documents are graph-level Markdown records, not nodes and not linked to individual nodes in the first version.
- The graph is a directed acyclic graph with shared children allowed.
- `focal-fs` uses canonical directories and symlink aliases for shared children.
- `focal-sqlite` uses namespace-scoped rows, logical paths, and alias placements to mirror filesystem behavior.
- Traversal is deterministic, breadth-first, and deduplicated.
- Deleting and unlinking must preserve shared descendants unless explicitly targeted.
- Validation, safety boundaries, and typed recoverable errors are part of the public contract.
- MCP exposes only the configured graph through stable `focal_` tools and `focal://` read-only resources.