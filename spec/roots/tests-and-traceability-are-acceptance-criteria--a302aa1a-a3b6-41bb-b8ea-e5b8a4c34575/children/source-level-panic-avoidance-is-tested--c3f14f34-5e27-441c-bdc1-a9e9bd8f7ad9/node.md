---
id: c3f14f34-5e27-441c-bdc1-a9e9bd8f7ad9
kind: statement
title: Source-Level Panic Avoidance Is Tested
created_at_unix: 1777859013
updated_at_unix: 1777859013
reviewed: false
---

The test plan explicitly requires public `focal-core` and `focal-mcp` code paths to avoid panic-style handling for recoverable states. This turns error-handling discipline into an observable acceptance condition rather than a code style preference.