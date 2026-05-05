---
id: 4316128f-3d0e-40b8-9d41-7eed741d3216
kind: statement
title: MCP Server Access Is Configured Per Session
created_at_unix: 1777858982
updated_at_unix: 1777858982
reviewed: false
---

The MCP server uses a `ServerConfig` with backend and transport settings, and tools operate only on the configured graph for that server session. The first version does not expose arbitrary per-tool filesystem paths, database paths, or already-open backend handles to model calls.