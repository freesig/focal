---
id: f7b129cd-87dd-461f-b255-b7750fa00db3
kind: statement
title: Core Code Avoids Panic Paths For Recoverable States
created_at_unix: 1777858963
updated_at_unix: 1777858963
reviewed: false
---

Public core operation paths must handle internal `Option` values and backend failures with `match`, `if let`, `let else`, `ok_or_else`, `?`, or explicit conversion. Recoverable graph states must not use `unwrap`, `expect`, unchecked indexing, `panic!`, `todo!`, or `unimplemented!`.