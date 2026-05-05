---
id: efc3f316-a71d-4cc2-a4af-af2ae175f9c0
kind: statement
title: Filesystem Storage Uses Canonical Directories And Symlink Aliases
created_at_unix: 1777858569
updated_at_unix: 1777858569
reviewed: false
---

`focal-fs` represents the graph as a human-editable directory tree under `roots/`, with one real node directory containing `node.md` and `children/` for each canonical placement. Shared children are represented by relative symlink alias directories pointing to the canonical node directory, while context documents live under top-level `context/`.