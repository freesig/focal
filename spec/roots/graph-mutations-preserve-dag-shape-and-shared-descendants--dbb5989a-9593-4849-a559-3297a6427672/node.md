---
id: dbb5989a-9593-4849-a559-3297a6427672
kind: statement
title: Graph Mutations Preserve DAG Shape And Shared Descendants
created_at_unix: 1777858572
updated_at_unix: 1777858572
reviewed: false
---

The first version treats the idea graph as a directed acyclic graph with zero or more parents and children per node. Link, unlink, delete, and promotion behavior must reject cycles, avoid orphaning unless the caller selects an orphan policy, and preserve shared descendants unless a requested operation explicitly deletes them.