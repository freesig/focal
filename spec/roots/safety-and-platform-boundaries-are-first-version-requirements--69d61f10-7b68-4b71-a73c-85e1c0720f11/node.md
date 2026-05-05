---
id: 69d61f10-7b68-4b71-a73c-85e1c0720f11
kind: statement
title: Safety And Platform Boundaries Are First-Version Requirements
created_at_unix: 1777858590
updated_at_unix: 1777858590
reviewed: false
---

The filesystem backend treats the graph root as a hard boundary: symlink targets, destructive operations, context deletes, generated IDs, and path handling must not escape it. The first version supports macOS and Linux for symlink-backed filesystem graphs, while SQLite safety is scoped to the configured graph namespace and parameterized SQL.