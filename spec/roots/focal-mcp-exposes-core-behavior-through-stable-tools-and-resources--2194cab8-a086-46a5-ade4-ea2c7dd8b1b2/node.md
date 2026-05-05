---
id: 2194cab8-a086-46a5-ade4-ea2c7dd8b1b2
kind: statement
title: focal-mcp Exposes Core Behavior Through Stable Tools And Resources
created_at_unix: 1777858588
updated_at_unix: 1777858588
reviewed: false
---

`focal-mcp` is an adapter over `focal-core` for MCP-capable clients. It exposes each shared graph operation as a stable `focal_` tool, exposes read-only graph views through `focal://` resources and templates, supports stdio first, keeps graph access config-driven, and preserves core ordering, validation, mutation, promotion, and error behavior.