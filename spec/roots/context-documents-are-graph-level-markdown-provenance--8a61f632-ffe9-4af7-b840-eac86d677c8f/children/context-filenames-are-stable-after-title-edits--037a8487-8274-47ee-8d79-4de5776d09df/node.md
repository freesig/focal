---
id: 037a8487-8274-47ee-8d79-4de5776d09df
kind: statement
title: Context Filenames Are Stable After Title Edits
created_at_unix: 1777858876
updated_at_unix: 1777858876
reviewed: false
---

For both filesystem files and SQLite logical filenames, a context document's filename is assigned when it is created and must not be recalculated or renamed when the title changes. The title metadata is authoritative even when the slug in the stored filename becomes stale.