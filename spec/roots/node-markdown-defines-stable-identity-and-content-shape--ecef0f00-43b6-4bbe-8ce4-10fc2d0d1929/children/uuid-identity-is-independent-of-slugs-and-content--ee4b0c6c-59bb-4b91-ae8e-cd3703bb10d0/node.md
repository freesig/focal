---
id: ee4b0c6c-59bb-4b91-ae8e-cd3703bb10d0
kind: statement
title: UUID Identity Is Independent Of Slugs And Content
created_at_unix: 1777858896
updated_at_unix: 1777858896
reviewed: false
---

Node IDs and context document IDs are UUID v4 strings that remain stable for the lifetime of the object. IDs are authoritative over slugs, independent of title and content, and must reject path separators, dot components, control characters, and platform-reserved forms.