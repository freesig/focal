---
id: 84a6c467-a448-4793-92c6-da0ad380ccbf
kind: statement
title: SQLite Variant Exists Even When Disabled
created_at_unix: 1777858958
updated_at_unix: 1777858958
reviewed: false
---

The public `Backend::Sqlite` variant remains present when the `sqlite` feature is disabled by wrapping an empty placeholder type. Graph operations on that disabled variant return a typed disabled-SQLite core error, while connection-based SQLite constructors are compiled only when the feature is enabled.