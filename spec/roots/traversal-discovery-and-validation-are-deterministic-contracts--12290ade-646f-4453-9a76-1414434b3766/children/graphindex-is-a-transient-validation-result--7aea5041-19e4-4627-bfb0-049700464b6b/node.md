---
id: 7aea5041-19e4-4627-bfb0-049700464b6b
kind: statement
title: GraphIndex Is A Transient Validation Result
created_at_unix: 1777858930
updated_at_unix: 1777858930
reviewed: false
---

`rebuild_index` returns a transient `GraphIndex` containing context summaries, node summaries, graph edges, and graph problems. Filesystem graphs discover this by scanning on demand rather than relying on an in-memory or persistent index for normal operation.