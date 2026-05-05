---
id: a9b6bfea-34e4-413a-846f-4645b4928d9d
kind: statement
title: Context Markdown Body Is Arbitrary Provenance
created_at_unix: 1777858879
updated_at_unix: 1777858879
reviewed: false
---

A context document body may contain arbitrary Markdown and may be empty. Library edits manage only the context metadata block and should preserve the body as much as possible because the document is provenance, not normalized graph content.