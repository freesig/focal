---
id: aca25914-0b4e-4f5b-96fb-23513c27ac94
kind: statement
title: Flat Metadata Fields Are Required For Nodes
created_at_unix: 1777858886
updated_at_unix: 1777858886
reviewed: false
---

Node Markdown starts with a flat line-oriented metadata block that must include `id`, `kind`, `title`, `created_at_unix`, `updated_at_unix`, and `reviewed`. Missing or invalid metadata, including invalid reviewed values, makes the node Markdown invalid.