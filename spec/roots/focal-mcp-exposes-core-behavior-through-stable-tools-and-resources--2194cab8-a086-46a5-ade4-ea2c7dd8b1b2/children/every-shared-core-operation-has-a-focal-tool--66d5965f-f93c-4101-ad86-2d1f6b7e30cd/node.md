---
id: 66d5965f-f93c-4101-ad86-2d1f6b7e30cd
kind: statement
title: Every Shared Core Operation Has A focal Tool
created_at_unix: 1777858985
updated_at_unix: 1777858985
reviewed: false
---

`focal-mcp` must expose one stable `focal_` MCP tool for every shared `focal-core` graph operation, including context document CRUD, node CRUD, linking, unlinking, listing, traversal, and `rebuild_index`. Tool schemas mirror Rust public field names and stable lowercase enum values.