---
id: db75c89f-c0e2-475c-a249-6380471f35d4
kind: statement
title: Unlink Orphan Policies Are Explicit Caller Choices
created_at_unix: 1777858916
updated_at_unix: 1777858916
reviewed: false
---

When unlinking would leave a child with no parents, callers must choose `MoveToRoots`, `DeleteIfNoParents`, or `FailIfWouldOrphan`. The library should not silently orphan, delete, or promote nodes without the requested orphan policy.