---
id: 0a17376b-569c-4f8c-913e-b340989f66db
kind: statement
title: Graph Recognition Does Not Depend On A Version File
created_at_unix: 1777858899
updated_at_unix: 1777858899
reviewed: false
---

A filesystem graph is recognized by the presence of `roots/`, valid node directories, and valid context Markdown files when present. The first version requires no `.idea-graph/VERSION`; `init_graph` creates `roots/` and `context/`, and `open_graph` auto-creates missing `context/` for an otherwise valid graph root.