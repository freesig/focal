---
id: ecfbc410-0da5-4671-a7d5-48306a950b3d
kind: statement
title: SQLite Graphs Are Namespaced Inside Shared Databases
created_at_unix: 1777858966
updated_at_unix: 1777858966
reviewed: false
---

A single SQLite database may contain multiple named graph namespaces. Every graph metadata, context document, node, placement, and edge query must be scoped to the selected graph namespace so operations cannot bleed across graphs.