---
id: 23e1d51f-77b6-4358-a7ff-f515d9a2890c
kind: statement
title: Filesystem Destructive Operations Stay Inside Graph Root
created_at_unix: 1777858996
updated_at_unix: 1777858996
reviewed: false
---

Before deleting or rewriting filesystem paths, `focal-fs` must canonicalize targets, verify they are inside the graph root, and ensure context document deletes only remove Markdown files inside `<graph-root>/context/`. The backend must never delete paths outside the graph boundary.