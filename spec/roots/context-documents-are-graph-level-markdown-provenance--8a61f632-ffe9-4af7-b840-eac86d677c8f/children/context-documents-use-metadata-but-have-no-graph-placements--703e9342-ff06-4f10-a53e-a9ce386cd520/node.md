---
id: 703e9342-ff06-4f10-a53e-a9ce386cd520
kind: statement
title: Context Documents Use Metadata But Have No Graph Placements
created_at_unix: 1777858874
updated_at_unix: 1777858874
reviewed: false
---

Context documents use a Markdown metadata block with stable UUID identity, title, and timestamps, but they do not have parents, children, canonical placements, alias placements, or traversal behavior. Implementations must keep context CRUD separate from node and edge mutations.