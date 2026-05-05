---
id: fdaf3720-a947-44ec-8443-412f8ec5048d
kind: statement
title: Path Fields Mean Filesystem Paths Or Logical Paths
created_at_unix: 1777858944
updated_at_unix: 1777858944
reviewed: false
---

Public path fields such as canonical paths, alias paths, graph edge paths, and context paths are real filesystem paths for `focal-fs` but logical graph paths for `focal-sqlite`. Callers must not infer that SQLite paths exist on disk.