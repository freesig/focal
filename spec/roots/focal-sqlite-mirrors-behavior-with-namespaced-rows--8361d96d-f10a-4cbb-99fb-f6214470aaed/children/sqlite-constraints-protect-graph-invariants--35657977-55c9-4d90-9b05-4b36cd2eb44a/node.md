---
id: 35657977-55c9-4d90-9b05-4b36cd2eb44a
kind: statement
title: SQLite Constraints Protect Graph Invariants
created_at_unix: 1777858976
updated_at_unix: 1777858976
reviewed: false
---

The schema should enforce namespace uniqueness, context ID and filename uniqueness within a namespace, node ID uniqueness within a namespace, edge uniqueness, and canonical-placement uniqueness where SQLite supports the constraint. Validation must still report stored inconsistencies such as missing endpoints or duplicate placements.