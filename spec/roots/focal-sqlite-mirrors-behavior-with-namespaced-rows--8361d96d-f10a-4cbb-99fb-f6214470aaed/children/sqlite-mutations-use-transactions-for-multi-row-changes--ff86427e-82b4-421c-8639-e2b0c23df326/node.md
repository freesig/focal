---
id: ff86427e-82b4-421c-8639-e2b0c23df326
kind: statement
title: SQLite Mutations Use Transactions For Multi-Row Changes
created_at_unix: 1777858971
updated_at_unix: 1777858971
reviewed: false
---

Mutations that insert, delete, unlink, or promote multiple SQLite rows should run as one transaction. Failed multi-row operations should leave callers able to inspect persisted state with `rebuild_index`.