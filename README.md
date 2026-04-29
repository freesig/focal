# Focal

Focal is a Rust workspace for tools that organize messy context into structured ideas and help focus attention on the most important and impactful parts.

The project stores ideas as a local graph. Ideas can be added, linked, updated, traversed, and inspected through storage backends that share the same core model.

## Workspace

This repository is organized as a Cargo workspace:

- `crates/focal_types`: Shared graph types and errors.
- `crates/focal_fs`: Filesystem-backed idea graph library.
- `crates/focal_sqlite`: SQLite-backed idea graph library.
- `spec/SPEC.md`: Behavioral specification and test traceability for the current library.

## Crates

### `focal-types`

`focal-types` defines the shared node, traversal, index, and error types used by Focal storage backends.

### `focal-fs`

`focal-fs` stores idea graphs on disk using folders, Markdown files, and symbolic links for local, human-readable workflows.

### `focal-sqlite`

`focal-sqlite` stores idea graphs in named SQLite namespaces using a caller-provided `rusqlite::Connection`.

## Development

Run the full test suite:

```sh
cargo test
```

Format the workspace:

```sh
cargo fmt
```

Check the workspace:

```sh
cargo check
```
