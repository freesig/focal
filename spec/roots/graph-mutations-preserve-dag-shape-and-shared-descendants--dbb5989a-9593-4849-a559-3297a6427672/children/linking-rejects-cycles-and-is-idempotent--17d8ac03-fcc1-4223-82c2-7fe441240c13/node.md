---
id: 17d8ac03-fcc1-4223-82c2-7fe441240c13
kind: statement
title: Linking Rejects Cycles And Is Idempotent
created_at_unix: 1777858910
updated_at_unix: 1777858910
reviewed: false
---

`link_existing_node` must return success without mutation when the edge already exists, reject self-links, and reject any link that would make an ancestor a descendant of itself. Traversal must still track visited IDs to avoid infinite loops if manual edits create malformed cycles.