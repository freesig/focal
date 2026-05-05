---
id: 68f46b98-ede8-4820-be6c-15c12479ba5c
kind: statement
title: Node Directory Suffixes Carry Authority
created_at_unix: 1777858902
updated_at_unix: 1777858902
reviewed: false
---

Filesystem node directories use `<slug>--<node-id>`, where the final UUID suffix is authoritative and the slug is only readable decoration. If the slug disagrees with the title stored in `node.md`, the Markdown metadata wins, and title edits must not rename directories.