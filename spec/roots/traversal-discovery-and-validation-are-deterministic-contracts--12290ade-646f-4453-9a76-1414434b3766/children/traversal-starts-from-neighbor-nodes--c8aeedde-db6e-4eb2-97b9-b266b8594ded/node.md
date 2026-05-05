---
id: c8aeedde-db6e-4eb2-97b9-b266b8594ded
kind: statement
title: Traversal Starts From Neighbor Nodes
created_at_unix: 1777858925
updated_at_unix: 1777858925
reviewed: false
---

Ancestor and descendant traversal starts from the start node's parents or children, not from the start node itself. `TraversalOptions.max_depth` limits how far the breadth-first walk expands, with `None` representing no depth limit.