---
id: 6b3e5a24-7826-4dde-a714-28d4639fa592
kind: qa
title: Who Owns The SQLite Connection?
created_at_unix: 1777858979
updated_at_unix: 1777858979
reviewed: false
---

## Question

Does `focal-sqlite` own the `rusqlite::Connection` it operates on?

## Answer

No. Its `IdeaGraph` handle borrows a caller-provided mutable `rusqlite::Connection`; `open_database` is only a convenience helper for callers that want the crate to construct a connection from a path.

## Alternative answers

- The MCP server may own a database path and open connections internally, but that is part of the MCP adapter configuration, not the `focal-sqlite` library handle contract.