---
id: 8361d96d-f10a-4cbb-99fb-f6214470aaed
kind: statement
title: focal-sqlite Mirrors Behavior With Namespaced Rows
created_at_unix: 1777858585
updated_at_unix: 1777858585
reviewed: false
---

`focal-sqlite` implements the same graph behavior as `focal-fs` using a caller-provided `rusqlite::Connection` and a named graph namespace. It stores contexts, nodes, content, canonical placements, alias placements, edges, stable logical paths, and mutations in SQLite rows while matching filesystem traversal, promotion, validation, and ordering semantics.