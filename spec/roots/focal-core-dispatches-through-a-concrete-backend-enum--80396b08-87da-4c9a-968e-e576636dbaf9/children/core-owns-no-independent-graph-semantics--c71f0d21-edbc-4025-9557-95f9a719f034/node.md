---
id: c71f0d21-edbc-4025-9557-95f9a719f034
kind: statement
title: Core Owns No Independent Graph Semantics
created_at_unix: 1777858955
updated_at_unix: 1777858955
reviewed: false
---

`focal-core` is strictly a dispatch facade over storage crates. It must not add another storage engine, reinterpret graph behavior, or expose backend-specific branching to applications that use the unified API.