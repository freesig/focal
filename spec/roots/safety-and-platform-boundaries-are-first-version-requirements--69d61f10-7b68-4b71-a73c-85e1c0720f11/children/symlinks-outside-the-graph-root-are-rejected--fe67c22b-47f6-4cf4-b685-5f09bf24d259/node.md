---
id: fe67c22b-47f6-4cf4-b685-5f09bf24d259
kind: statement
title: Symlinks Outside The Graph Root Are Rejected
created_at_unix: 1777858998
updated_at_unix: 1777858998
reviewed: false
---

`focal-fs` must not follow or preserve symlink targets that escape the graph root. Validation should report outside-root targets, and traversal through malformed or broken symlinks should return a graph error rather than silently ignoring unsafe storage.