---
id: 42a66a21-dbd9-45a6-9aeb-3f2411b0ebce
kind: statement
title: Recoverable Graph Errors Are Tool Results
created_at_unix: 1777858991
updated_at_unix: 1777858991
reviewed: false
---

Recoverable errors from `focal-core` should become MCP tool execution results with `isError: true` and structured content preserving backend kind, error kind, human-readable message, and relevant graph context. Protocol errors are reserved for malformed requests, unknown tools, schema violations, and initialization failures.