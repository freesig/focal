---
id: 41c83f83-5fab-4ed8-94a7-5196d53656af
kind: statement
title: Alias Entries Point To Canonical Node Directories
created_at_unix: 1777858905
updated_at_unix: 1777858905
reviewed: false
---

A symlink node entry must point to the canonical node directory rather than directly to `node.md`. The library should create relative symlinks where possible so a graph can move as a folder without breaking shared-child links.