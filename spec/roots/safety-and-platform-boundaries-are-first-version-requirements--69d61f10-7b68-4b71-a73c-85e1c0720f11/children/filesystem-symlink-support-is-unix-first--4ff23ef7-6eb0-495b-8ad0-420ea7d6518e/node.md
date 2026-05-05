---
id: 4ff23ef7-6eb0-495b-8ad0-420ea7d6518e
kind: statement
title: Filesystem Symlink Support Is Unix-First
created_at_unix: 1777859001
updated_at_unix: 1777859001
reviewed: false
---

The first filesystem backend version supports Unix-like platforms with directory symlink support, specifically macOS and Linux. Windows is unsupported in the first version and may return `SymlinkUnsupported` on unsupported platforms.