---
id: 12290ade-646f-4453-9a76-1414434b3766
kind: statement
title: Traversal Discovery And Validation Are Deterministic Contracts
created_at_unix: 1777858576
updated_at_unix: 1777858576
reviewed: false
---

Traversal returns breadth-first ancestors or descendants from neighboring nodes, deduplicated by node ID and ordered deterministically by title then ID. `rebuild_index` is a public validation and inspection path that discovers contexts, nodes, edges, and consistency problems without requiring a persistent filesystem index.