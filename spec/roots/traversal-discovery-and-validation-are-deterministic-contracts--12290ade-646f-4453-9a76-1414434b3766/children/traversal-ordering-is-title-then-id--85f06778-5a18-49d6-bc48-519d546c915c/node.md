---
id: 85f06778-5a18-49d6-bc48-519d546c915c
kind: statement
title: Traversal Ordering Is Title Then ID
created_at_unix: 1777858927
updated_at_unix: 1777858927
reviewed: false
---

Children, ancestors, and descendants must be returned deterministically: breadth-first, deduplicated by node ID, and ordered lexicographically by node title then node ID. This ordering applies even when multiple paths can reach the same node.