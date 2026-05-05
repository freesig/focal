---
id: ae4639a1-7c57-484e-978a-f9fb126f5afe
kind: statement
title: SQLite Stores Logical Placements Not Files
created_at_unix: 1777858969
updated_at_unix: 1777858969
reviewed: false
---

`focal-sqlite` must not create or depend on `roots/`, `node.md`, `children/`, or symlinks for normal operation. It stores stable logical filenames and placement paths that follow the same slug-and-ID conventions as `focal-fs` for comparable ordering, summaries, and diagnostics.