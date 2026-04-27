# Focal

Focal is a Rust workspace for tools that organize messy context into structured ideas and help focus attention on the most important and impactful parts.

The project stores ideas as a local graph of plain files. Ideas can be added, linked, updated, traversed, and inspected without a database, daemon, network service, or required UI.

## Workspace

This repository is organized as a Cargo workspace:

- `crates/focal_fs`: Filesystem-backed idea graph library.
- `spec/SPEC.md`: Behavioral specification and test traceability for the current library.

## Crates

### `focal-fs`

`focal-fs` provides the `focal_fs` Rust library. It stores an idea graph on disk using folders, Markdown files, and symbolic links.

The crate supports:

- Initializing and opening an idea graph.
- Creating root ideas and child ideas.
- Reading and updating idea nodes.
- Linking one idea under multiple parents.
- Unlinking and deleting nodes.
- Listing roots, parents, children, ancestors, and descendants.
- Rebuilding and validating an index from the filesystem.

The filesystem format is intended to remain human-readable, so a graph can be inspected and edited with normal file tools.

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
