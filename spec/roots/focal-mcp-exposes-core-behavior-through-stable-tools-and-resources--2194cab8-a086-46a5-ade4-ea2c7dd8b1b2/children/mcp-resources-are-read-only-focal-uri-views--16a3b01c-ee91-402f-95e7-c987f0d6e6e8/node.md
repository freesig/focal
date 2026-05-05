---
id: 16a3b01c-ee91-402f-95e7-c987f0d6e6e8
kind: statement
title: MCP Resources Are Read-Only focal URI Views
created_at_unix: 1777858988
updated_at_unix: 1777858988
reviewed: false
---

The server exposes read-only graph views through stable `focal://` resources and templates for graph index, contexts, roots, nodes, parents, children, ancestors, and descendants. Resource reads must validate URI IDs and query parameters before calling the matching core operation.