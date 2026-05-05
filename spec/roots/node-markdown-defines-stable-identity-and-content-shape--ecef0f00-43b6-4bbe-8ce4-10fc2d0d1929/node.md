---
id: ecef0f00-43b6-4bbe-8ce4-10fc2d0d1929
kind: statement
title: Node Markdown Defines Stable Identity And Content Shape
created_at_unix: 1777858567
updated_at_unix: 1777858567
reviewed: false
---

Every node is stored as Markdown with managed metadata for UUID identity, kind, title, timestamps, and reviewed state. Statement nodes keep body Markdown after metadata; question-answer nodes must keep managed Question, Answer, and Alternative answers sections, with ordered caller-provided alternatives kept separate from the primary answer.