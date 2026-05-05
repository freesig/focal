---
id: 41fda602-8913-41b1-952b-65f85c4895ba
kind: statement
title: Canonical Parent Removal Triggers Alias Promotion
created_at_unix: 1777858919
updated_at_unix: 1777858919
reviewed: false
---

If a canonical placement is removed while alias parents remain, the backend must promote the lexicographically first alias path into the new canonical placement and rewrite remaining alias targets or placement rows. Promotion must preserve node IDs, Markdown content, and the full child subtree.