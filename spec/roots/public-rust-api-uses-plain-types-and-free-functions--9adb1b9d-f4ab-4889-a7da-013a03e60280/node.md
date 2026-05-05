---
id: 9adb1b9d-f4ab-4889-a7da-013a03e60280
kind: statement
title: Public Rust API Uses Plain Types And Free Functions
created_at_unix: 1777858579
updated_at_unix: 1777858579
reviewed: false
---

The crates expose small, constructible Rust data types and synchronous free functions for graph operations. Backend handles own or borrow only their storage-specific state, public operations return typed `Result` errors for recoverable absence or storage failures, and consuming applications do not need a CLI, web server, daemon, async runtime, or trait-object dispatch layer.