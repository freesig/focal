---
id: d6a17d4b-fffa-4a34-bea9-1d7ad1b5b502
kind: statement
title: Edits Preserve Identity And Placements
created_at_unix: 1777858952
updated_at_unix: 1777858952
reviewed: false
---

Node and context edits update managed metadata and `updated_at_unix` without changing IDs, canonical placements, alias placements, filesystem directory names, SQLite logical paths, parent-child relationships, or context filenames. Reviewed-only edits must preserve content and graph structure.