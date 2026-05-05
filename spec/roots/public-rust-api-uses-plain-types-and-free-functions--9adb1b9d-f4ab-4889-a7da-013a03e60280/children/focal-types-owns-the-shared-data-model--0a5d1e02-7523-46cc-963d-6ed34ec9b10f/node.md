---
id: 0a5d1e02-7523-46cc-963d-6ed34ec9b10f
kind: statement
title: focal-types Owns The Shared Data Model
created_at_unix: 1777858940
updated_at_unix: 1777858940
reviewed: false
---

The public data model lives in `focal-types` and is reused by storage crates and the core facade. Types such as `Node`, `NewNode`, `NodePatch`, `ContextDocument`, `TraversalOptions`, `GraphIndex`, and mode enums must stay small, cloneable where specified, and easy to construct in tests.