---
id: e06567d6-2365-49ab-9d23-c0cafa2ea4b7
kind: statement
title: Recursive Delete Removes Only Privately Reachable Descendants
created_at_unix: 1777858922
updated_at_unix: 1777858922
reviewed: false
---

`DeleteMode::Recursive` deletes the target node and descendants reachable only through the deleted subtree. Descendants that also have parents outside the deleted subtree must remain, with only the edge from the deleted subtree removed and promotion applied when their canonical location was inside the deleted subtree.