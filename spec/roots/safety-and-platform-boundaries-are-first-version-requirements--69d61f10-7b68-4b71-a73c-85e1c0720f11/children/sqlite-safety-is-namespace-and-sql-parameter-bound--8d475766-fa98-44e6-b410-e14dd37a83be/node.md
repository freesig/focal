---
id: 8d475766-fa98-44e6-b410-e14dd37a83be
kind: statement
title: SQLite Safety Is Namespace And SQL Parameter Bound
created_at_unix: 1777859003
updated_at_unix: 1777859003
reviewed: false
---

`focal-sqlite` treats the configured graph namespace as the graph boundary and must use parameterized SQL for caller-supplied values. It must not shell out or allow arbitrary SQL access as part of the public graph API.