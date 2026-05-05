---
id: c92b1a79-9660-4861-b2a9-effce4a022c5
kind: statement
title: Stdio Output Must Remain JSON-RPC Only
created_at_unix: 1777858993
updated_at_unix: 1777858993
reviewed: false
---

For stdio transport, stdout must contain only MCP JSON-RPC messages, with logs written to stderr. This is a protocol-level safety requirement and must be covered by MCP transport tests.