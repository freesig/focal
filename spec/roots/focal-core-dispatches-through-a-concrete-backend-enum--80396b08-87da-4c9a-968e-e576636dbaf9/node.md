---
id: 80396b08-87da-4c9a-968e-e576636dbaf9
kind: statement
title: focal-core Dispatches Through A Concrete Backend Enum
created_at_unix: 1777858583
updated_at_unix: 1777858583
reviewed: false
---

`focal-core` owns no storage format and defines no independent graph semantics. It re-exports shared types, exposes a concrete `Backend` enum over filesystem and SQLite variants, gates SQLite support behind a Cargo feature, keeps a disabled SQLite placeholder variant when the feature is off, and forwards every operation to the selected backend while preserving typed backend errors.