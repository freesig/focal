---
id: 2c55c73a-e716-471a-9783-541bb3fd2c4e
kind: statement
title: Shared Behavior Tests Run Across Backends
created_at_unix: 1777859006
updated_at_unix: 1777859006
reviewed: false
---

The same public behavior tests should cover filesystem, SQLite, and unified core dispatch where applicable. SQLite tests must demonstrate parity for context CRUD, ordered alternative answers, alias placements, promotion, recursive delete with shared descendants, traversal determinism, validation, and storage errors.