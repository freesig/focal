---
id: 86d9ed91-489a-45f4-9444-eea06a37867b
kind: statement
title: Core Errors Preserve Backend Error Types
created_at_unix: 1777858961
updated_at_unix: 1777858961
reviewed: false
---

`focal_core::Error` must preserve backend failures as typed variants instead of flattening them into strings. Missing values, storage failures, and validation errors should be converted from the backend error type and propagated with path, node ID, context ID, or storage context intact.