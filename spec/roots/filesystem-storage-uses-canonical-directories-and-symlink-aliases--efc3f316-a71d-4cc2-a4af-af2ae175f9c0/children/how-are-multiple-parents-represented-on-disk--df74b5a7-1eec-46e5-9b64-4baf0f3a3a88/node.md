---
id: df74b5a7-1eec-46e5-9b64-4baf0f3a3a88
kind: qa
title: How Are Multiple Parents Represented On Disk?
created_at_unix: 1777858908
updated_at_unix: 1777858908
reviewed: false
---

## Question

How should `focal-fs` represent a node that has more than one parent?

## Answer

One parent owns the canonical real directory, and every additional parent gets a symlink alias entry pointing to that canonical directory.

## Alternative answers

- Duplicating Markdown under each parent is rejected because it would break stable node identity and shared edits.
- A database-style edge table is reserved for the SQLite backend, not the filesystem backend.