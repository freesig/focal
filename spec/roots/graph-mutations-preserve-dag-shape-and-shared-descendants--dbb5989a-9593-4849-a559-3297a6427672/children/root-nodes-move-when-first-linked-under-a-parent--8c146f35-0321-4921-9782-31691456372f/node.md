---
id: 8c146f35-0321-4921-9782-31691456372f
kind: statement
title: Root Nodes Move When First Linked Under A Parent
created_at_unix: 1777858913
updated_at_unix: 1777858913
reviewed: false
---

When an existing root node with no other parents is linked under a parent, the backend moves that node's canonical placement under the new parent rather than leaving it as a root plus adding an alias. This preserves the shared semantics between filesystem and SQLite placement models.