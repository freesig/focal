# Focal Idea Graph Rust Library Specification

## 1. Purpose

Build small Rust libraries for storing and navigating an idea graph.

Each graph can also store graph-level original idea context documents: editable Markdown records of the messy material that generated or later shaped the graph.

The shared behavior is exposed through simple Rust functions for reading, adding, editing, linking, unlinking, deleting, and traversing nodes.

Two storage crates are specified:

- `focal-fs`: stores the graph and original idea context as plain folders, Markdown files, and symbolic links.
- `focal-sqlite`: stores the same graph model and original idea context in SQLite through `rusqlite` instead of the filesystem.

A shared `focal-types` crate contains the common public data types used by both storage crates.

A `focal-core` crate provides the unified public API for applications that want to choose a storage backend at runtime without writing backend-specific dispatch code.

Each implementation should be understandable and usable from other Rust applications without requiring a CLI or web server in the consuming application. `focal-fs` should remain dependency-light and should not require a daemon, database, or async runtime. `focal-sqlite` should use a caller-provided `rusqlite::Connection`.

## 2. Goals

- Store every idea node as Markdown content.
- Store graph-level original idea context as editable Markdown documents.
- Support multiple original idea context Markdown documents per graph.
- Represent parent-child structure in the backend's native storage.
- For `focal-fs`, keep each node in its own folder.
- For `focal-fs`, keep each node's children in a dedicated child subfolder.
- For `focal-fs`, use symlinks when a node has more than one parent.
- For `focal-fs`, keep original idea context Markdown files in a dedicated top-level `context/` folder.
- For `focal-sqlite`, store nodes, content, and edges in SQLite while preserving the same graph behavior as `focal-fs`.
- For `focal-sqlite`, store original idea context documents in SQLite rows scoped to the graph namespace.
- Provide a `focal-core` crate whose free-function API dispatches to `focal-fs` and, when the `sqlite` feature is enabled, `focal-sqlite` through a concrete `Backend` enum.
- Support statement nodes and question-answer nodes.
- Support creating, reading, updating, deleting, and listing graph-level original idea context documents.
- Support graph navigation from any node.
- Support listing ancestors and descendants.
- Preserve stable node identity when titles or backend-specific storage locations change.
- Keep `focal-fs` graphs usable by humans editing files manually, while making library-created changes consistent.
- Keep `focal-sqlite` graph operations deterministic and compatible with the same public behavior tests as `focal-fs`.

## 3. Non-Goals

- No database backend for `focal-fs`.
- No network service for `focal-fs`.
- No required CLI.
- No additional storage engine inside `focal-core`; it is only a dispatch facade over storage crates.
- No rich Markdown rendering.
- No collaborative multi-writer synchronization.
- No automatic semantic analysis of ideas.
- No support for arbitrary graph cycles in the first version.
- No requirement for `focal-sqlite` to support manual filesystem edits, symlink repair, or arbitrary SQL access by callers.
- No node-level provenance links from original idea context documents to generated nodes in the first version.
- No version history or append-only audit log for original idea context document edits.

## 4. Terminology

- **Storage backend**: The persistence mechanism used by a crate. `focal-fs` uses the filesystem; `focal-sqlite` uses SQLite through `rusqlite`.
- **Unified backend**: The `focal-core::Backend` enum value used by callers that want the same API over either `focal-fs` or `focal-sqlite`.
- **Graph root**: For `focal-fs`, the root directory containing the whole idea graph.
- **SQLite graph**: For `focal-sqlite`, a named graph namespace stored in tables inside a shared SQLite database and opened through a borrowed `rusqlite::Connection`.
- **Original idea context**: Messy graph-level Markdown material that helped generate or evolve the graph, stored separately from nodes.
- **Context document**: One editable original idea context Markdown document. Context documents are graph-level records and are not linked to individual nodes in the first version.
- **Context directory**: For `focal-fs`, the top-level `<graph-root>/context/` directory containing context Markdown files.
- **Node**: One idea entry. A node is either a statement or a question-answer pair.
- **Node directory**: For `focal-fs`, the directory that contains one node's `node.md` file and `children/` directory.
- **Canonical node directory**: For `focal-fs`, the real directory that owns the node's Markdown and children.
- **Alias node directory**: For `focal-fs`, a symlink to a canonical node directory.
- **Canonical placement**: The authoritative backend location for a node. In `focal-fs`, this is a canonical node directory. In `focal-sqlite`, this is a persisted placement row.
- **Alias placement**: An additional parent placement for a node. In `focal-fs`, this is an alias node directory. In `focal-sqlite`, this is a persisted non-canonical placement row.
- **Parent edge**: A relationship from one node to one child node.
- **Child edge**: The same relationship viewed from the parent node.
- **Root node**: A node with no parent.
- **Ancestor**: A parent, parent's parent, and so on.
- **Descendant**: A child, child's child, and so on.
- **Slug**: A backend-safe title fragment used in filesystem directory names or logical SQLite paths.
- **Node ID**: Stable, backend-safe identifier for a node. The ID must not change when the title, Markdown, folder slug, or logical path changes.

## 5. `focal-fs` Filesystem Layout

This section applies to the `focal-fs` crate.

The graph root uses this layout:

```text
<graph-root>/
  context/
    <context-slug>--<context-id>.md
  roots/
    <slug>--<node-id>/
      node.md
      children/
        <child-slug>--<child-id>/
          node.md
          children/
        <linked-child-slug>--<linked-child-id> -> ../../somewhere/<canonical-child-dir>
```

Required directory for graph recognition:

- `<graph-root>/roots/`

Managed directories:

- `<graph-root>/context/`

Required files:

- `<node-dir>/node.md`

Required child directory:

- `<node-dir>/children/`

Optional metadata directory:

- `<graph-root>/.idea-graph/`

No `VERSION` file is required in the first version. The `.idea-graph/` directory is reserved for optional metadata such as lock files, but a graph is identified by the presence of `roots/`, valid node directories, and valid context Markdown files when any are present. `init_graph` must create `context/`, and `open_graph` must auto-create `context/` when it is missing from an otherwise valid graph root.

Every real node directory must contain both `node.md` and `children/`.

Every symlink node entry must point to a canonical node directory, not directly to `node.md`.

The library must create relative symlinks where possible so a graph can be moved as a folder without breaking links.

Every context document file must be a regular Markdown file under `context/` whose file name ends with `--<context-id>.md`.

## 6. `focal-fs` Node Directory Naming

Node directories use:

```text
<slug>--<node-id>
```

Rules:

- The final `--<node-id>` suffix is authoritative.
- The slug is for readability only.
- The slug is generated from the title when a node is created.
- The slug may become stale after title edits.
- If the slug and `node.md` title disagree, `node.md` wins.
- If two generated directory names would conflict, append `-2`, `-3`, and so on before the `--<node-id>` suffix.

Example:

```text
why-rust-for-local-graphs--550e8400-e29b-41d4-a716-446655440000
```

## 7. Node IDs

Node IDs and context document IDs must be:

- Stable for the lifetime of the node or context document.
- Unique within their graph resource collection.
- Safe for use in directory names and logical paths.
- Independent of title and content.

Required format:

```text
xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

Example:

```text
550e8400-e29b-41d4-a716-446655440000
```

The library must generate UUID v4 IDs for new nodes.

Valid ID characters:

```text
a-f 0-9 -
```

The library must reject IDs that are not valid UUID strings. It must also reject IDs containing path separators, `.` path components, control characters, or platform-reserved names.

The library must generate UUID v4 IDs for new context documents using the same format and validation rules. Context document IDs are unique within the graph's context document collection and do not share an identity namespace with node IDs.

## 8. Markdown Format

Each node has a Markdown representation. For `focal-fs`, each node is stored in `node.md`. For `focal-sqlite`, the same fields and content are stored in SQLite rows, and any Markdown import or export should use this format.

The file starts with a simple line-oriented metadata block. The metadata block looks like YAML front matter, but the parser only needs to support flat `key: value` lines.

Required metadata fields:

- `id`
- `kind`
- `title`
- `created_at_unix`
- `updated_at_unix`

Supported `kind` values:

- `statement`
- `qa`

### Statement Node Markdown

```markdown
---
id: 550e8400-e29b-41d4-a716-446655440000
kind: statement
title: Rust keeps local tools simple
created_at_unix: 1777274701
updated_at_unix: 1777274701
---

Rust is a good fit for local graph tools because it can manage files safely
without requiring a runtime service.
```

For a statement node:

- The body is the Markdown content after the metadata block.
- The title lives in metadata only.
- The library must not require, insert, remove, or manage a top-level `# Heading`.
- The library should preserve body Markdown as much as possible when editing metadata only.

### Question-Answer Node Markdown

```markdown
---
id: 7d9f2e5c-0f22-4c18-a0be-9f23e772a0bc
kind: qa
title: Why use symlinks?
created_at_unix: 1777274712
updated_at_unix: 1777274712
---

## Question

Why should a child node with multiple parents be represented with a symlink?

## Answer

A symlink lets the graph keep one canonical Markdown file while allowing the
same node to appear under more than one parent folder.
```

For a question-answer node:

- The question is the Markdown content under `## Question`.
- The answer is the Markdown content under `## Answer`.
- The library must reject `qa` files missing either section.

### Original Idea Context Markdown

Original idea context is stored as graph-level Markdown documents. For `focal-fs`, each context document is stored as a Markdown file under `<graph-root>/context/`. For `focal-sqlite`, the same fields and Markdown body are stored in SQLite rows scoped to the graph namespace.

Context documents are not nodes and do not have parents, children, canonical placements, aliases, or traversal behavior.

For `focal-fs`, context file names use:

```text
<slug>--<context-id>.md
```

Rules:

- The final `--<context-id>.md` suffix is authoritative.
- The slug is for readability only.
- The slug is generated from the title when a context document is created.
- The slug may become stale after title edits.
- If the slug and Markdown title metadata disagree, the Markdown title metadata wins.
- If two generated file names would conflict, append `-2`, `-3`, and so on before the `--<context-id>.md` suffix.

The file starts with the same simple line-oriented metadata block style as nodes.

Required metadata fields:

- `id`
- `title`
- `created_at_unix`
- `updated_at_unix`

Example:

```markdown
---
id: 7a736f79-bf3f-4d1e-8bd8-71fd9b94a2d4
title: Raw planning notes
created_at_unix: 1777274800
updated_at_unix: 1777274800
---

The initial prompt mixed feature ideas, unresolved questions, and examples.
Keep it here so future graph changes can refer back to the original messy
context without turning it into graph nodes immediately.
```

For a context document:

- The body is the Markdown content after the metadata block.
- The title lives in metadata only.
- The library must not require, insert, remove, or manage a top-level `# Heading`.
- The body may be empty.
- The body may contain arbitrary Markdown.
- The library should preserve body Markdown as much as possible when editing metadata only.
- Context title edits update metadata but do not rename the filesystem file or SQLite logical filename.

## 9. Rust Data Model

The public API should expose plain Rust types from a shared `focal-types` crate. `focal-fs` and `focal-sqlite` should use these shared types directly, and may re-export them for caller convenience.

Backend-specific graph handles and initialization functions live in their backend crates.

```rust
use std::path::PathBuf;

pub type NodeId = String;
pub type ContextId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Statement,
    QuestionAnswer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub title: String,
    pub content: NodeContent,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    pub canonical_path: PathBuf,
    pub alias_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeContent {
    Statement { body: String },
    QuestionAnswer { question: String, answer: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNode {
    pub kind: NodeKind,
    pub title: String,
    pub content: NodeContent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodePatch {
    pub title: Option<String>,
    pub content: Option<NodeContent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSummary {
    pub id: NodeId,
    pub kind: NodeKind,
    pub title: String,
    pub canonical_path: PathBuf,
    pub is_alias: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextDocument {
    pub id: ContextId,
    pub title: String,
    pub filename: String,
    pub markdown: String,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewContextDocument {
    pub title: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContextDocumentPatch {
    pub title: Option<String>,
    pub markdown: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSummary {
    pub id: ContextId,
    pub title: String,
    pub filename: String,
    pub path: PathBuf,
}
```

The implementation may add private fields, but public types should stay small and easy to construct in tests.

For `focal-fs`, `canonical_path`, `alias_paths`, and `GraphEdge.path` are filesystem paths. For `focal-sqlite`, those fields are logical graph paths built from the same slug and node ID format, such as `roots/parent--<id>/children/child--<id>`. They identify the same canonical and alias placements but must not imply that files or directories exist on disk.

For `focal-sqlite`, logical placement paths and slugs are stored when a placement is created. They must remain stable after title edits, matching the `focal-fs` behavior where directory slugs may become stale.

For `focal-fs`, `ContextDocument.path` and `ContextSummary.path` are filesystem paths under `<graph-root>/context/`. For `focal-sqlite`, these fields are logical graph paths such as `context/raw-planning-notes--<context-id>.md`. The `filename` field is the final file-name component, including the `.md` extension. SQLite stores this logical filename when the context document is created and must not recalculate it after title edits.

## 10. Public API

Each crate should provide an opaque graph handle and simple functions with equivalent behavior.

`focal-fs` opens graphs from a filesystem root:

```rust
use std::path::Path;

pub struct IdeaGraph {
    root: std::path::PathBuf,
}

pub fn init_graph(root: impl AsRef<Path>) -> Result<IdeaGraph, GraphError>;

pub fn open_graph(root: impl AsRef<Path>) -> Result<IdeaGraph, GraphError>;

pub fn add_context_document(
    graph: &IdeaGraph,
    context: NewContextDocument,
) -> Result<ContextId, GraphError>;

pub fn read_context_document(
    graph: &IdeaGraph,
    context_id: &str,
) -> Result<ContextDocument, GraphError>;

pub fn update_context_document(
    graph: &IdeaGraph,
    context_id: &str,
    patch: ContextDocumentPatch,
) -> Result<ContextDocument, GraphError>;

pub fn delete_context_document(
    graph: &IdeaGraph,
    context_id: &str,
) -> Result<(), GraphError>;

pub fn list_context_documents(
    graph: &IdeaGraph,
) -> Result<Vec<ContextSummary>, GraphError>;

pub fn add_root_node(
    graph: &IdeaGraph,
    node: NewNode,
) -> Result<NodeId, GraphError>;

pub fn add_child_node(
    graph: &IdeaGraph,
    parent_id: &str,
    node: NewNode,
) -> Result<NodeId, GraphError>;

pub fn read_node(
    graph: &IdeaGraph,
    node_id: &str,
) -> Result<Node, GraphError>;

pub fn update_node(
    graph: &IdeaGraph,
    node_id: &str,
    patch: NodePatch,
) -> Result<Node, GraphError>;

pub fn delete_node(
    graph: &IdeaGraph,
    node_id: &str,
    mode: DeleteMode,
) -> Result<(), GraphError>;

pub fn link_existing_node(
    graph: &IdeaGraph,
    parent_id: &str,
    child_id: &str,
) -> Result<(), GraphError>;

pub fn unlink_child(
    graph: &IdeaGraph,
    parent_id: &str,
    child_id: &str,
    orphan_policy: OrphanPolicy,
) -> Result<(), GraphError>;

pub fn list_roots(
    graph: &IdeaGraph,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_children(
    graph: &IdeaGraph,
    node_id: &str,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_parents(
    graph: &IdeaGraph,
    node_id: &str,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_ancestors(
    graph: &IdeaGraph,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_descendants(
    graph: &IdeaGraph,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn rebuild_index(
    graph: &IdeaGraph,
) -> Result<GraphIndex, GraphError>;
```

`rebuild_index` may be public even if a backend does not persist a separate index. It gives callers a way to validate and inspect the graph.

`focal-sqlite` opens graphs from a borrowed mutable `rusqlite::Connection` and a graph namespace:

```rust
use std::path::Path;

use rusqlite::Connection;

pub struct IdeaGraph<'conn> {
    connection: &'conn mut Connection,
    graph_name: String,
}

pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, GraphError>;

pub fn init_graph<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<IdeaGraph<'conn>, GraphError>;

pub fn open_graph<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<IdeaGraph<'conn>, GraphError>;

pub fn add_context_document(
    graph: &mut IdeaGraph<'_>,
    context: NewContextDocument,
) -> Result<ContextId, GraphError>;

pub fn read_context_document(
    graph: &IdeaGraph<'_>,
    context_id: &str,
) -> Result<ContextDocument, GraphError>;

pub fn update_context_document(
    graph: &mut IdeaGraph<'_>,
    context_id: &str,
    patch: ContextDocumentPatch,
) -> Result<ContextDocument, GraphError>;

pub fn delete_context_document(
    graph: &mut IdeaGraph<'_>,
    context_id: &str,
) -> Result<(), GraphError>;

pub fn list_context_documents(
    graph: &IdeaGraph<'_>,
) -> Result<Vec<ContextSummary>, GraphError>;

pub fn add_root_node(
    graph: &mut IdeaGraph<'_>,
    node: NewNode,
) -> Result<NodeId, GraphError>;

pub fn add_child_node(
    graph: &mut IdeaGraph<'_>,
    parent_id: &str,
    node: NewNode,
) -> Result<NodeId, GraphError>;

pub fn read_node(
    graph: &IdeaGraph<'_>,
    node_id: &str,
) -> Result<Node, GraphError>;

pub fn update_node(
    graph: &mut IdeaGraph<'_>,
    node_id: &str,
    patch: NodePatch,
) -> Result<Node, GraphError>;

pub fn delete_node(
    graph: &mut IdeaGraph<'_>,
    node_id: &str,
    mode: DeleteMode,
) -> Result<(), GraphError>;

pub fn link_existing_node(
    graph: &mut IdeaGraph<'_>,
    parent_id: &str,
    child_id: &str,
) -> Result<(), GraphError>;

pub fn unlink_child(
    graph: &mut IdeaGraph<'_>,
    parent_id: &str,
    child_id: &str,
    orphan_policy: OrphanPolicy,
) -> Result<(), GraphError>;

pub fn list_roots(
    graph: &IdeaGraph<'_>,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_children(
    graph: &IdeaGraph<'_>,
    node_id: &str,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_parents(
    graph: &IdeaGraph<'_>,
    node_id: &str,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_ancestors(
    graph: &IdeaGraph<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn list_descendants(
    graph: &IdeaGraph<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError>;

pub fn rebuild_index(
    graph: &IdeaGraph<'_>,
) -> Result<GraphIndex, GraphError>;
```

The `focal-sqlite` graph handle mutably borrows the caller-provided connection and must not own or close it. Mutating operations, including context document add, update, and delete operations, take `&mut IdeaGraph`; read, list, traversal, and validation operations take `&IdeaGraph`.

`open_database` is a convenience helper for callers that want the crate to create a `rusqlite::Connection` from a database path. Callers may also construct and configure their own `rusqlite::Connection` and pass it to `init_graph` or `open_graph`.

`focal-core` exposes a unified API over both storage crates. It should depend on `focal-types` and `focal-fs` by default. SQLite support is enabled by a Cargo feature named `sqlite`; when that feature is enabled, `focal-core` also depends on `focal-sqlite` and `rusqlite`. The backend crates must not depend on `focal-core`.

The unified API must be plain Rust free functions over a concrete backend enum, not a trait hierarchy, trait object, macro-generated API, async runtime, CLI wrapper, or service layer. It should re-export the shared `focal-types` public data types so callers can use the core crate as their primary dependency.

```rust
use std::path::Path;

#[cfg(feature = "sqlite")]
use rusqlite::Connection;

pub enum Backend<'conn> {
    Fs(focal_fs::IdeaGraph),
    #[cfg(feature = "sqlite")]
    Sqlite(focal_sqlite::IdeaGraph<'conn>),
    #[cfg(not(feature = "sqlite"))]
    Sqlite(EmptySqlite<'conn>),
}

#[cfg(not(feature = "sqlite"))]
pub struct EmptySqlite<'conn> {
    graph_name: String,
    _lifetime: std::marker::PhantomData<&'conn mut ()>,
}

pub enum Error {
    Fs(focal_fs::Error),
    #[cfg(feature = "sqlite")]
    Sqlite(focal_sqlite::Error),
    #[cfg(not(feature = "sqlite"))]
    Sqlite(DisabledSqliteError),
}

#[cfg(not(feature = "sqlite"))]
pub struct DisabledSqliteError {
    pub graph_name: String,
}

pub fn init_fs(
    root: impl AsRef<Path>,
) -> Result<Backend<'static>, Error>;

pub fn open_fs(
    root: impl AsRef<Path>,
) -> Result<Backend<'static>, Error>;

#[cfg(feature = "sqlite")]
pub fn open_database(
    path: impl AsRef<Path>,
) -> Result<Connection, Error>;

#[cfg(feature = "sqlite")]
pub fn init_sqlite<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<Backend<'conn>, Error>;

#[cfg(feature = "sqlite")]
pub fn open_sqlite<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<Backend<'conn>, Error>;

#[cfg(not(feature = "sqlite"))]
pub fn disabled_sqlite(
    graph_name: impl Into<String>,
) -> Backend<'static>;

pub fn add_context_document(
    backend: &mut Backend<'_>,
    context: NewContextDocument,
) -> Result<ContextId, Error>;

pub fn read_context_document(
    backend: &Backend<'_>,
    context_id: &str,
) -> Result<ContextDocument, Error>;

pub fn update_context_document(
    backend: &mut Backend<'_>,
    context_id: &str,
    patch: ContextDocumentPatch,
) -> Result<ContextDocument, Error>;

pub fn delete_context_document(
    backend: &mut Backend<'_>,
    context_id: &str,
) -> Result<(), Error>;

pub fn list_context_documents(
    backend: &Backend<'_>,
) -> Result<Vec<ContextSummary>, Error>;

pub fn add_root_node(
    backend: &mut Backend<'_>,
    node: NewNode,
) -> Result<NodeId, Error>;

pub fn add_child_node(
    backend: &mut Backend<'_>,
    parent_id: &str,
    node: NewNode,
) -> Result<NodeId, Error>;

pub fn read_node(
    backend: &Backend<'_>,
    node_id: &str,
) -> Result<Node, Error>;

pub fn update_node(
    backend: &mut Backend<'_>,
    node_id: &str,
    patch: NodePatch,
) -> Result<Node, Error>;

pub fn delete_node(
    backend: &mut Backend<'_>,
    node_id: &str,
    mode: DeleteMode,
) -> Result<(), Error>;

pub fn link_existing_node(
    backend: &mut Backend<'_>,
    parent_id: &str,
    child_id: &str,
) -> Result<(), Error>;

pub fn unlink_child(
    backend: &mut Backend<'_>,
    parent_id: &str,
    child_id: &str,
    orphan_policy: OrphanPolicy,
) -> Result<(), Error>;

pub fn list_roots(
    backend: &Backend<'_>,
) -> Result<Vec<NodeSummary>, Error>;

pub fn list_children(
    backend: &Backend<'_>,
    node_id: &str,
) -> Result<Vec<NodeSummary>, Error>;

pub fn list_parents(
    backend: &Backend<'_>,
    node_id: &str,
) -> Result<Vec<NodeSummary>, Error>;

pub fn list_ancestors(
    backend: &Backend<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, Error>;

pub fn list_descendants(
    backend: &Backend<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, Error>;

pub fn rebuild_index(
    backend: &Backend<'_>,
) -> Result<GraphIndex, Error>;
```

The `Fs` variant does not borrow a connection. The filesystem constructor return type may use `Backend<'static>` or an equivalent lifetime design that lets filesystem backends be passed to every core operation accepting `Backend<'_>`.

The `Sqlite` variant must remain present whether or not the `sqlite` feature is enabled. With the feature enabled, it wraps `focal_sqlite::IdeaGraph<'conn>`. With the feature disabled, it wraps an empty SQLite placeholder type such as `EmptySqlite<'conn>`, which must not depend on `focal-sqlite` or `rusqlite`.

When the `sqlite` feature is disabled, every graph operation called with `Backend::Sqlite(EmptySqlite { .. })` must return `Err(focal_core::Error::Sqlite(DisabledSqliteError { .. }))` or an equivalent disabled-SQLite error payload. The SQLite connection-based constructor functions must only be available when the `sqlite` feature is enabled because their signatures require `rusqlite::Connection`.

Core functions must dispatch exhaustively with `match backend` and forward to the corresponding backend crate operation. Mutating core operations take `&mut Backend<'_>` so the same call shape works for `focal-sqlite`; `focal-fs` may still implement its native mutating operations with shared references internally.

The `focal-core` implementation must follow functional Rust error handling:

- All public core graph operations return `Result<T, focal_core::Error>`.
- Backend crates should expose public error types as `focal_fs::Error` and `focal_sqlite::Error`. These backend error types may differ.
- `focal_core::Error` must preserve backend failures as typed variants: `Fs(focal_fs::Error)` for filesystem failures and, when the `sqlite` feature is enabled, `Sqlite(focal_sqlite::Error)` for SQLite failures.
- `focal_core::Error` should implement `From<focal_fs::Error>` and, when the `sqlite` feature is enabled, `From<focal_sqlite::Error>`.
- Missing context documents, nodes, parents, children, placements, canonical paths, or alias paths must be converted to the appropriate backend error and then into `focal_core::Error`.
- Internal `Option` values must be handled with `match`, `if let`, `let else`, or `ok_or_else`; public core code must not use `unwrap`, `expect`, unchecked indexing, `panic!`, `todo!`, or `unimplemented!` for recoverable states.
- Backend errors must be propagated with `?` or converted explicitly without discarding path, node ID, or storage context.
- The core crate must not silently ignore errors from either backend to keep the shared API behavior identical across `focal-fs` and `focal-sqlite`.

## 11. Delete and Unlink Modes

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    FailIfHasChildren,
    Recursive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrphanPolicy {
    MoveToRoots,
    DeleteIfNoParents,
    FailIfWouldOrphan,
}
```

### `FailIfHasChildren`

Delete the node only if it has no children.

If the node has children, return `GraphError::NodeHasChildren`.

### `Recursive`

Delete the node and all descendants that are only reachable through the deleted subtree.

If a descendant also has another parent outside the deleted subtree, only remove the edge from the deleted subtree. The shared descendant itself must remain.

### `MoveToRoots`

When unlinking a child would leave it with no parents, move the child's canonical placement under roots.

### `DeleteIfNoParents`

When unlinking a child would leave it with no parents, delete the child according to `DeleteMode::Recursive`.

### `FailIfWouldOrphan`

When unlinking a child would leave it with no parents, abort with `GraphError::WouldOrphanNode`.

## 12. Symlink Semantics

This section applies to the `focal-fs` crate. `focal-sqlite` must provide equivalent shared-parent behavior with persisted placement rows instead of symlinks.

A node with one parent may be represented as a real directory under that parent's `children/`.

A node with multiple parents must have:

- One canonical real directory.
- One symlink entry for each additional parent.

When `link_existing_node(parent_id, child_id)` is called:

- If the edge already exists, return success without changing the filesystem.
- If `parent_id == child_id`, return `GraphError::CycleDetected`.
- If the link would make an ancestor a descendant of itself, return `GraphError::CycleDetected`.
- If the child is currently a root node with no other parents, move the child's canonical placement under the new parent.
- Otherwise, create a symlink in `<parent-dir>/children/`.
- The symlink must point to the child's canonical node directory.

When reading through a symlink:

- The returned node ID and content must come from the canonical `node.md`.
- The caller should be able to see alias paths in `Node.alias_paths`.

When deleting or unlinking:

- Removing an alias parent removes only the symlink edge.
- Removing the canonical parent while aliases remain must preserve the node unless the requested operation deletes the node.
- If preservation is required, the library must promote one alias path to become the new canonical directory.
- During promotion, the library must rewrite remaining alias symlink targets so they resolve to the new canonical directory.
- Promotion is required behavior and must be covered by thorough tests.

## 13. Graph Shape and Cycles

The first version must treat the idea graph as a directed acyclic graph.

Rules:

- A node may have zero or more parents.
- A node may have zero or more children.
- A root node has zero parents.
- Cycles are not allowed.
- Linking a node under one of its descendants must fail.
- Traversal must still protect itself against malformed manual cycles by tracking visited node IDs.

## 14. Traversal

Traversal uses deterministic breadth-first ordering.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraversalOptions {
    pub max_depth: Option<usize>,
}
```

Default traversal options:

```rust
TraversalOptions {
    max_depth: None,
}
```

Traversal requirements:

- Traversal must deduplicate by node ID.
- Traversal must not emit the same node twice even when multiple paths exist.
- Children should be returned in deterministic order.
- Deterministic order should be lexicographic by node title, then node ID.
- Traversal must use breadth-first ordering.
- Traversal starts from the start node's neighbors, not the start node itself.
- `list_ancestors` walks parent edges upward.
- `list_descendants` walks child edges downward.
- For `focal-fs`, traversal through symlinks must behave the same as traversal through real directories.
- For `focal-fs`, if malformed manual edits create a missing target, traversal should return `GraphError::BrokenSymlink` unless the caller later gets an option to ignore broken links.

## 15. Editing Semantics

`update_node` must support:

- Editing a statement body.
- Editing a question.
- Editing an answer.
- Editing a title.
- Updating `updated_at_unix`.

When title changes:

- Rewrite the title metadata field.
- Do not rewrite a top-level `# Heading`.
- Do not rename the canonical placement.
- Do not rename alias placements.
- Keep the node ID unchanged.
- Keep parent and child relationships unchanged.
- The directory slug or logical path slug remains stable and may become stale after a title edit.

Markdown rewrite rules:

- Metadata should be rewritten in a stable field order.
- Existing content should be preserved where possible.
- For statement nodes, preserve body Markdown.
- For question-answer nodes, preserve question and answer Markdown except for the managed `## Question` and `## Answer` section headings.
- For context documents, preserve body Markdown except for the managed metadata block.

`update_context_document` must support:

- Editing context document Markdown.
- Editing a context document title.
- Updating `updated_at_unix`.

When a context document title changes:

- Rewrite the title metadata field.
- Do not rewrite a top-level `# Heading`.
- Do not rename the filesystem file or SQLite logical filename.
- Keep the context document ID unchanged.
- Keep graph nodes and graph edges unchanged.

## 16. Add Semantics

Adding a root node:

- Creates a canonical placement under roots.
- Stores node metadata and Markdown content.
- For `focal-fs`, creates a canonical node directory under `<graph-root>/roots/`, writes `node.md`, and creates `children/`.
- Returns the generated node ID.

Adding a child node:

- Finds the parent's canonical placement.
- Creates a canonical placement under the parent.
- Stores node metadata and Markdown content.
- For `focal-fs`, creates a canonical node directory under `<parent-dir>/children/`, writes `node.md`, and creates `children/`.
- Returns the generated node ID.

Adding a context document:

- Creates one graph-level original idea context document.
- Generates a UUID v4 context document ID.
- Stores context metadata and Markdown body.
- For `focal-fs`, creates a Markdown file under `<graph-root>/context/`.
- For `focal-sqlite`, inserts a context document row scoped to the graph namespace.
- Returns the generated context document ID.

Input validation:

- Title must not be empty after trimming.
- Statement body may be empty, but should be allowed.
- Question text must not be empty after trimming.
- Answer text may be empty if the caller wants to capture unanswered questions, but the `## Answer` section must exist.
- Node ID must be unique.
- Context document title must not be empty after trimming.
- Context document Markdown body may be empty.
- Context document ID must be unique within the graph's context document collection.

## 17. Read Semantics

`read_node` must:

- Locate a node by ID anywhere in the graph.
- Parse metadata.
- Parse content according to `kind`.
- Return canonical path and alias paths. For `focal-sqlite`, these are logical graph paths.
- For `focal-fs`, return an error if multiple real canonical directories claim the same ID.
- For `focal-fs`, return an error if the node is found only as a broken symlink.

`list_roots` must:

- Return all root nodes.
- For `focal-fs`, return all nodes under `<graph-root>/roots/`.
- Include canonical root placements.
- Include promoted roots.
- Exclude broken symlinks unless returning a validation error.

`read_context_document` must:

- Locate a context document by ID in the graph-level context document collection.
- Parse metadata.
- Return the Markdown body exactly as stored apart from normal newline handling.
- Return the filesystem path for `focal-fs` and a logical context path for `focal-sqlite`.
- For `focal-fs`, return an error if multiple context Markdown files claim the same ID.

`list_context_documents` must:

- Return all graph-level context documents.
- Sort deterministically by title, then context document ID.
- For `focal-fs`, scan `<graph-root>/context/`.
- For `focal-sqlite`, query context document rows scoped to the graph namespace.

`delete_context_document` must:

- Delete only the selected context document.
- Leave all nodes, edges, placements, aliases, and traversal behavior unchanged.
- For `focal-fs`, delete the matching Markdown file under `<graph-root>/context/`.
- For `focal-sqlite`, delete the matching context document row scoped to the graph namespace.

## 18. Limited Movement and Promotion

The first version does not expose general node move behavior.

Automatic movement is limited to:

- Promotion when deleting or unlinking a canonical parent while alias parents remain.
- Moving an orphaned canonical node to roots when `OrphanPolicy::MoveToRoots` is explicitly requested.

Promotion requirements:

- If a canonical placement is removed from one parent while alias placements remain, choose the lexicographically first alias path as the new canonical location.
- Replace that alias placement with the canonical placement.
- Rewrite other aliases to point to the new canonical placement when the backend stores explicit alias targets.
- Preserve the promoted node's Markdown content and full child subtree.
- Keep the promoted node ID unchanged.
- Keep child node IDs unchanged.
- Keep Markdown unchanged except for `updated_at_unix` if the operation intentionally records structural edits.
- Title edits must not trigger directory or logical placement moves in the first version.

## 19. Indexing and Discovery

`focal-fs` must discover nodes and context documents by scanning the filesystem on demand.

The first `focal-fs` version must not require an in-memory index or a persistent index for normal operation. `focal-sqlite` may use SQLite tables and indexes as its source of truth.

`GraphIndex` is a transient validation and inspection result returned by `rebuild_index`.

```rust
#[derive(Debug, Clone)]
pub struct GraphIndex {
    pub contexts: Vec<ContextSummary>,
    pub nodes: Vec<NodeSummary>,
    pub edges: Vec<GraphEdge>,
    pub problems: Vec<GraphProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub parent_id: NodeId,
    pub child_id: NodeId,
    pub path: PathBuf,
    pub is_symlink: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphProblem {
    BrokenSymlink { path: PathBuf },
    DuplicateContextDocument { id: ContextId, paths: Vec<PathBuf> },
    DuplicateCanonicalNode { id: NodeId, paths: Vec<PathBuf> },
    MissingNodeMarkdown { path: PathBuf },
    MissingChildrenDirectory { path: PathBuf },
    InvalidContextMarkdown { path: PathBuf, reason: String },
    InvalidMarkdown { path: PathBuf, reason: String },
    CycleDetected { node_id: NodeId },
}
```

`focal-fs` scan rules:

- Ensure `context/` exists before scanning by creating it if missing.
- Scan `context/` for context Markdown files.
- Scan `roots/` recursively.
- Record context documents by context document ID.
- Record real directories as canonical candidates.
- Record symlink entries as edges to canonical targets.
- Do not recursively descend into symlink entries after recording the edge.
- Deduplicate by node ID.
- Deduplicate context documents by context document ID.
- Validate that every context document file has valid context metadata.
- Validate that every real node directory has `node.md` and `children/`.

For `focal-sqlite`, `rebuild_index` must query SQLite and return the same `GraphIndex` shape. `ContextSummary.path` and `GraphEdge.path` must be logical graph paths, and `GraphEdge.is_symlink` must be `true` for alias placements and `false` for canonical placements.

## 20. Error Handling

Storage backend public functions return `Result<T, GraphError>` or a crate-local error type with the same required error semantics. Backend crates that expose crate-local errors should name them `focal_fs::Error` and `focal_sqlite::Error` so `focal-core` can preserve them as typed variants.

```rust
#[derive(Debug)]
pub enum GraphError {
    Io(std::io::Error),
    Storage(String),
    InvalidGraphRoot(String),
    ContextNotFound(String),
    NodeNotFound(String),
    ParentNotFound(String),
    ChildNotFound(String),
    DuplicateContextId(String),
    DuplicateNodeId(String),
    InvalidContextId(String),
    InvalidNodeId(String),
    InvalidTitle,
    InvalidContextMarkdown { path: std::path::PathBuf, reason: String },
    InvalidMarkdown { path: std::path::PathBuf, reason: String },
    MissingNodeMarkdown(std::path::PathBuf),
    MissingChildrenDirectory(std::path::PathBuf),
    BrokenSymlink(std::path::PathBuf),
    SymlinkUnsupported(String),
    CycleDetected,
    NodeHasChildren(String),
    WouldOrphanNode(String),
    PermissionDenied(std::path::PathBuf),
    AliasConflict(std::path::PathBuf),
    DuplicateContextDocument { id: String, paths: Vec<std::path::PathBuf> },
    DuplicateCanonicalNode { id: String, paths: Vec<std::path::PathBuf> },
}
```

The implementation should provide `Display` and `std::error::Error` implementations.

`GraphError::Io` should preserve the original `std::io::Error`.

`GraphError::Storage` should be used for backend errors that are not naturally `std::io::Error`, including SQLite, `rusqlite`, or SQL execution failures.

`focal-core` is the exception to the backend error return type. It owns `focal_core::Error` and its public graph operations return `Result<T, focal_core::Error>`.

## 21. Atomicity and Consistency

`focal-fs` uses best-effort filesystem operations.

Requirements:

- Create directories before writing `node.md`.
- Ensure the `context/` directory exists before writing or scanning context Markdown files.
- Create symlinks only after the target node exists.
- Do not require rollback for multi-step directory or symlink operations.
- Do not require transactional guarantees for `focal-fs`.
- After an operation fails, callers should use `rebuild_index` to validate and repair the graph.
- Error values should include enough path information to help manual repair where possible.
- The implementation may use atomic file writes for Markdown, but atomic file writes are not required by this version of the spec.

`focal-sqlite` should execute each multi-row mutation as a single SQLite transaction. After a failed operation, callers should still be able to use `rebuild_index` to inspect persisted state.

## 22. Concurrency

The first `focal-fs` version only needs to support one writer process at a time.

Requirements:

- Multiple readers are acceptable.
- Concurrent writes are not guaranteed safe.
- The library should document this limitation.
- If a simple lock is implemented, use an exclusive lock file created with `create_new(true)` under `.idea-graph/write.lock`.
- Stale lock handling can be deferred.

`focal-sqlite` may rely on SQLite locking and `rusqlite` connection behavior for write serialization and consistency. The crate should document any additional client-side concurrency limitations.

## 23. Platform Requirements

These platform requirements apply to `focal-fs`.

Supported targets:

- Unix-like systems with directory symlink support.

Supported platforms:

- macOS
- Linux

Windows support:

- Windows is unsupported in the first version.
- The library may return `GraphError::SymlinkUnsupported` on unsupported platforms.

The code should isolate platform-specific symlink creation behind a small internal function.

`focal-sqlite` should support any platform where `rusqlite` can open a supported SQLite database.

## 24. Validation

The library should expose a validation path through `rebuild_index`.

`focal-fs` validation should detect:

- Missing `roots/`.
- Duplicate context document IDs.
- Invalid context Markdown metadata.
- Node directories without `node.md`.
- Node directories without `children/`.
- Invalid Markdown metadata.
- Duplicate node IDs.
- Broken symlinks.
- Cycles.
- Edges pointing outside the graph root.

For `focal-fs`, symlink targets outside the graph root must be rejected by default.

`focal-sqlite` validation should detect duplicate context document IDs, invalid context document content, duplicate node IDs, invalid node content, missing edge endpoints, duplicate parent-child placements, multiple canonical placements for one node, cycles, and any stored logical path that does not match the context document ID or node ID it claims to represent.

## 25. Security and Safety

`focal-fs` must treat the graph root as a boundary.

Requirements:

- Do not follow symlinks outside the graph root.
- Reject `..` path traversal in generated or user-supplied IDs.
- Never delete paths outside the graph root.
- Canonicalize paths before destructive operations.
- Validate that a delete target is inside the graph root.
- Validate that context document deletes only remove Markdown files inside `<graph-root>/context/`.
- Avoid shelling out to system commands.

`focal-sqlite` must treat the configured SQLite graph namespace as the graph boundary. It must use parameterized SQL for caller-supplied values and must not shell out to system commands.

## 26. Testing Requirements

Each backend implementation should include tests for the shared behavior below. Filesystem-specific assertions apply to `focal-fs`; SQLite-specific assertions apply to `focal-sqlite`.

Initialization:

- `init_graph` creates backend storage.
- For `focal-fs`, `init_graph` creates `roots/`.
- For `focal-fs`, `init_graph` creates `context/`.
- For `focal-fs`, `open_graph` creates `context/` when it is missing from an otherwise valid graph root.
- For `focal-fs`, `init_graph` does not require `.idea-graph/VERSION`.
- For `focal-fs`, `open_graph` fails for a non-graph directory.
- For `focal-sqlite`, `open_graph` fails when required graph tables, metadata, or the named graph namespace are missing.

Adding:

- Add a root statement node.
- Add a root question-answer node.
- Add a child statement node.
- Add a child question-answer node.
- Add multiple context documents.
- Verify generated node IDs are UUID strings.
- Verify generated context document IDs are UUID strings.
- Reject empty titles.
- Reject empty context document titles.
- Reject empty questions for `qa` nodes.

Reading:

- Read a statement node from its ID.
- Read a question-answer node from its ID.
- Read a context document from its ID.
- List context documents in deterministic order.
- For `focal-fs`, read a node through a symlink parent and get the canonical content.
- For `focal-sqlite`, read a node through an alias placement and get the canonical content.

Editing:

- Update statement body.
- Update question text.
- Update answer text.
- Update title and verify metadata updates.
- Verify title edits do not require a top-level `# Heading`.
- Verify title edits do not rename the node directory or logical path.
- Verify node ID remains stable across edits.
- Update context document Markdown.
- Update context document title.
- Verify context title edits do not require a top-level `# Heading`.
- Verify context title edits do not rename the context file or logical filename.
- Verify context document ID remains stable across edits.

Linking:

- Link an existing child under a second parent.
- For `focal-fs`, verify a symlink is created.
- For `focal-sqlite`, verify an alias placement row is created.
- Verify `list_parents` returns both parents.
- Verify duplicate link is idempotent.
- Reject a link that would create a cycle.

Unlinking:

- Remove an alias parent and keep the canonical node.
- Remove a canonical parent while an alias parent remains and verify promotion.
- Verify promoted nodes preserve Markdown content and the full child subtree.
- For `focal-fs`, verify promotion rewrites remaining alias symlinks.
- For `focal-sqlite`, verify promotion rewrites canonical and alias placement rows.
- Verify promotion with multiple aliases chooses the lexicographically first alias path.
- Move an orphaned node to roots when requested.
- Fail when unlinking would orphan a node and `FailIfWouldOrphan` is used.

Deleting:

- Delete a leaf node.
- Delete a context document without changing nodes or edges.
- Fail deleting a non-leaf with `FailIfHasChildren`.
- Recursively delete a subtree.
- Preserve shared descendants during recursive delete.
- Verify recursive delete promotes shared descendants correctly when their canonical location is inside the deleted subtree but aliases remain outside it.

Traversal:

- List children.
- List parents.
- List ancestors.
- List descendants.
- Deduplicate nodes reached through multiple paths.
- Respect `max_depth`.
- Respect breadth-first ordering.

Validation:

- Detect duplicate IDs.
- Detect duplicate context document IDs.
- For `focal-fs`, detect broken symlinks.
- For `focal-fs`, auto-create missing `context/` before validation scans it.
- For `focal-fs`, detect malformed context Markdown metadata.
- For `focal-fs`, detect missing `children/`.
- For `focal-fs`, detect malformed `node.md`.
- For `focal-sqlite`, detect invalid stored context document content, missing edge endpoints, duplicate placements, and invalid stored node content.
- Detect manual cycle and avoid infinite traversal.

Safety:

- For `focal-fs`, reject symlink targets outside graph root.
- For `focal-fs`, never delete outside graph root.
- For `focal-fs`, never delete context document paths outside `<graph-root>/context/`.
- For `focal-sqlite`, use parameterized SQL for caller-supplied values.

Unified core API:

- Construct a `focal_core::Backend::Fs` value from a filesystem graph and run the shared behavior tests through the `focal-core` free functions.
- With the `sqlite` feature enabled, construct a `focal_core::Backend::Sqlite` value from a SQLite graph and run the shared behavior tests through the same `focal-core` free functions.
- With the `sqlite` feature disabled, construct a `focal_core::Backend::Sqlite` value with the empty SQLite placeholder type and verify every graph operation returns the disabled-SQLite core error.
- Verify mutating core functions dispatch correctly for both backend variants.
- Verify read-only core functions dispatch correctly for both backend variants.
- Verify backend-specific errors are propagated as typed `focal_core::Error` variants without being swallowed or collapsed into strings.
- Verify context document operations dispatch correctly for both backend variants.
- Verify missing context documents, nodes, parents, children, and placements are returned as errors, not panics or public `Option` values.
- Verify public `focal-core` code paths do not use `unwrap`, `expect`, `panic!`, `todo!`, or `unimplemented!` for recoverable backend states.

## `focal-core` Unified API Crate

The workspace should include a `focal-core` crate, with Rust library name `focal_core`, that provides the unified free-function API described in Section 10.

`focal-core` owns no storage format and defines no independent graph semantics. It is a dispatch facade over `focal-fs` and, when the `sqlite` feature is enabled, `focal-sqlite`. It uses the shared `focal-types` data model and owns the `focal_core::Error` type used by the unified API.

The crate should:

- Re-export the shared `focal-types` public types.
- Define the concrete `Backend` enum over filesystem and SQLite graph handles.
- Always include filesystem backend support.
- Gate the `focal-sqlite` and `rusqlite` dependencies behind a Cargo feature named `sqlite`.
- Keep a public `Backend::Sqlite` variant when the `sqlite` feature is disabled by using an empty SQLite placeholder payload.
- Provide filesystem constructor functions unconditionally.
- Provide SQLite connection-based constructor functions only when the `sqlite` feature is enabled.
- Provide the shared graph operation functions that take `&Backend<'_>` or `&mut Backend<'_>`.
- Provide context document CRUD and list functions through the same backend dispatch model as node and traversal operations.
- Dispatch each operation by matching the `Backend` variant and forwarding to the matching backend crate.
- Preserve native backend errors as typed `focal_core::Error` variants.
- Avoid exposing backend-specific branching to applications using the unified API.

The crate must not:

- Introduce a third storage implementation.
- Require callers to use traits, trait objects, async tasks, a CLI, or a daemon.
- Convert recoverable missing-value states into panics.
- Return public `Option` values for graph operations that can report absence with `focal_core::Error`.

## `focal-sqlite` SQLite Backend

The workspace should include a `focal-sqlite` crate, with Rust library name `focal_sqlite`, that implements the same idea graph behavior as `focal-fs` while using SQLite through `rusqlite` as the source of truth.

`focal-sqlite` must not create or depend on `roots/`, `node.md`, `children/`, or symlink entries for normal operation. Filesystem paths exposed by shared public types are logical graph paths only.

A single SQLite database may contain multiple named graph namespaces. Every graph metadata, context document, node, placement, and edge query must be scoped to the selected graph namespace.

Initialization:

- `open_database(path)` should create a `rusqlite::Connection` from a database path for callers that want a convenience helper.
- `init_graph(&mut Connection, graph_name)` should create the required shared SQLite schema if missing, then create the named graph namespace if missing.
- `open_graph(&mut Connection, graph_name)` should validate that the required shared schema and named graph namespace already exist before returning an `IdeaGraph`.
- The `IdeaGraph` handle should borrow the caller-provided `rusqlite::Connection` and must not own or close it.
- The crate owns v1 schema creation through `init_graph`; callers own the connection lifecycle and may configure the `rusqlite::Connection` themselves.
- The first version does not migrate incompatible existing SQLite tables. If existing tables are partial or incompatible, `init_graph` may return a storage error and `open_graph` should reject the schema as invalid.

Storage requirements:

- Store graph namespace metadata in SQLite.
- Store context documents in SQLite with stable UUID context document IDs, filename, title, Markdown body, `created_at_unix`, and `updated_at_unix`.
- Store nodes in SQLite with stable UUID node IDs, kind, title, Markdown content fields, `created_at_unix`, and `updated_at_unix`.
- Store parent-child relationships in SQLite as placement rows scoped to the graph namespace.
- Each `(graph_name, context_id)` context document must be unique.
- Each `(graph_name, filename)` context document filename should be unique.
- A root node has a canonical placement with no parent.
- A child node has exactly one canonical placement and zero or more alias placements.
- Each `(graph_name, parent_id, child_id)` edge must be unique.
- Each node must have at most one canonical placement within a graph namespace.
- Logical paths should use the same slug and `--<node-id>` or `--<context-id>.md` naming conventions as `focal-fs` so ordering, summaries, and diagnostics stay comparable.
- Each context document should store its logical filename and slug at creation time. Title edits must not recalculate or rename stored logical filenames.
- Each placement should store its logical path and slug at creation time. Title edits must not recalculate or rename stored logical paths.

Behavioral requirements:

- Public operations after initialization must match `focal-fs` semantics unless this section explicitly says otherwise.
- Mutating operations should take `&mut IdeaGraph`; read, list, traversal, and validation operations should take `&IdeaGraph`.
- `add_context_document` inserts a context document row scoped to the selected graph namespace instead of creating a Markdown file.
- `read_context_document` returns the Markdown body and logical context path from the context document row.
- `update_context_document` updates the title, Markdown body, and `updated_at_unix` while preserving the context document ID and logical filename.
- `delete_context_document` deletes only the selected context document row and must not alter nodes, placements, or edges.
- `list_context_documents` returns context summaries in deterministic order.
- `add_root_node` and `add_child_node` insert node and placement rows scoped to the selected graph namespace instead of creating directories and Markdown files.
- `read_node` returns node content from the canonical node row, with logical canonical and alias paths.
- `link_existing_node` creates an alias placement row when the edge does not already exist, except when linking a root node with no other parents under a parent; in that case it moves the root's canonical placement under the new parent to match `focal-fs`.
- `unlink_child` removes only the requested placement and applies the same orphan policies as `focal-fs`.
- Promotion chooses the lexicographically first alias logical path and marks that placement canonical.
- `delete_node` preserves shared descendants and removes only rows that are exclusively reachable through the deleted subtree.
- Traversal order, cycle rejection, ancestor listing, descendant listing, and deduplication must match `focal-fs`.
- `rebuild_index` returns the same `GraphIndex` shape using SQLite queries.

Consistency requirements:

- Mutations that insert, delete, or promote multiple rows should run as one SQLite transaction.
- The schema should use database constraints for graph namespace uniqueness, context document ID uniqueness within a namespace, context filename uniqueness within a namespace, node ID uniqueness within a namespace, edge uniqueness, and canonical placement uniqueness where SQLite supports them.
- The crate should be safe to use with multiple readers and should document the write consistency guarantees it relies on from SQLite and `rusqlite`.

Testing requirements:

- The shared public API behavior tests should run against `focal-sqlite` with SQLite-backed storage.
- Tests should cover SQLite schema initialization, opening an existing SQLite graph namespace, context document CRUD and listing, alias placement creation, promotion, recursive delete with shared descendants, traversal determinism, validation, and storage errors.

## Spec Test Traceability

Each row points to at least one `focal-fs` in-crate unit test and one public API integration test for the numbered section. Sections that define `focal-core` behavior should also point to the `focal-core` integration tests that protect the unified dispatch API. When `focal-sqlite` is implemented, it must add equivalent traceability for the shared behavior and SQLite-specific backend requirements.

| Section | Unit tests | Integration tests |
|---|---|---|
| 1 | `crates/focal_fs/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_fs/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 2 | `crates/focal_fs/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_fs/tests/graph.rs::init_open_add_read_and_update_nodes`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_crud_namespace_sort_and_stable_filename` |
| 3 | `crates/focal_fs/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_fs/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 4 | `crates/focal_fs/src/model.rs::tests::spec_09_model_types_are_plain_constructible_values` | `crates/focal_fs/tests/graph.rs::linking_is_idempotent_and_rejects_cycles` |
| 5 | `crates/focal_fs/src/fs_utils.rs::tests::spec_05_and_23_symlinks_are_relative_directory_links_inside_graph` | `crates/focal_fs/tests/graph.rs::init_open_add_read_and_update_nodes`<br>`crates/focal_fs/tests/graph.rs::context_directory_is_created_and_open_auto_creates_missing_directory` |
| 6 | `crates/focal_fs/src/fs_utils.rs::tests::spec_06_directory_names_use_readable_unique_slugs_and_authoritative_ids` | `crates/focal_fs/tests/graph.rs::broken_symlink_name_is_not_reused_for_new_link` |
| 7 | `crates/focal_fs/src/fs_utils.rs::tests::spec_07_node_id_validation_accepts_uuid_strings_only`<br>`crates/focal_types/src/lib.rs::tests::context_models_are_plain_constructible_values` | `crates/focal_fs/tests/graph.rs::rejects_invalid_inputs_and_ids`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_crud_namespace_sort_and_stable_filename` |
| 8 | `crates/focal_fs/src/markdown.rs::tests::spec_08_statement_markdown_round_trips_without_heading_management`<br>`crates/focal_fs/src/markdown.rs::tests::spec_08_question_answer_markdown_requires_managed_sections`<br>`crates/focal_fs/src/markdown.rs::tests::spec_08_context_markdown_round_trips_without_heading_management`<br>`crates/focal_fs/src/markdown.rs::tests::spec_08_context_markdown_allows_empty_body_and_requires_metadata` | `crates/focal_fs/tests/graph.rs::root_and_child_question_answer_nodes_support_empty_answers`<br>`crates/focal_fs/tests/graph.rs::question_answer_updates_keep_identity_paths_and_managed_sections`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename` |
| 9 | `crates/focal_fs/src/model.rs::tests::spec_09_model_types_are_plain_constructible_values`<br>`crates/focal_types/src/lib.rs::tests::context_models_are_plain_constructible_values` | `crates/focal_fs/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 10 | `crates/focal_fs/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_fs/tests/graph.rs::example_usage_from_spec_runs_as_documented`<br>`crates/focal_core/tests/fs.rs::fs_backend_dispatches_shared_operations`<br>`crates/focal_core/tests/fs.rs::fs_backend_dispatches_context_operations`<br>`crates/focal_core/tests/fs.rs::fs_backend_error_wrapper_converts_with_from`<br>`crates/focal_core/tests/sqlite.rs::sqlite_backend_dispatches_shared_operations`<br>`crates/focal_core/tests/sqlite.rs::sqlite_backend_dispatches_context_operations`<br>`crates/focal_core/tests/sqlite.rs::sqlite_backend_error_wrapper_converts_with_from`<br>`crates/focal_core/tests/disabled_sqlite.rs::disabled_sqlite_backend_returns_disabled_error_for_every_operation` |
| 11 | `crates/focal_fs/src/model.rs::tests::spec_11_delete_and_orphan_modes_are_copyable_contract_values` | `crates/focal_fs/tests/graph.rs::delete_modes_handle_leaf_and_non_leaf_nodes`<br>`crates/focal_fs/tests/graph.rs::unlink_orphan_policies_move_fail_and_delete` |
| 12 | `crates/focal_fs/src/fs_utils.rs::tests::spec_05_and_23_symlinks_are_relative_directory_links_inside_graph` | `crates/focal_fs/tests/graph.rs::linking_is_idempotent_and_rejects_cycles`<br>`crates/focal_fs/tests/graph.rs::promotion_chooses_first_alias_and_rewrites_remaining_aliases` |
| 13 | `crates/focal_fs/src/scan.rs::tests::spec_13_and_24_cycle_detection_records_manual_cycles` | `crates/focal_fs/tests/graph.rs::linking_is_idempotent_and_rejects_cycles`<br>`crates/focal_fs/tests/graph.rs::ancestors_descendants_deduplicate_shared_paths_and_ignore_manual_cycle_start` |
| 14 | `crates/focal_fs/src/model.rs::tests::spec_14_traversal_options_default_has_no_depth_limit` | `crates/focal_fs/tests/graph.rs::question_answer_nodes_and_traversal_are_deterministic`<br>`crates/focal_fs/tests/graph.rs::ancestors_descendants_deduplicate_shared_paths_and_ignore_manual_cycle_start` |
| 15 | `crates/focal_fs/src/markdown.rs::tests::spec_08_statement_markdown_round_trips_without_heading_management`<br>`crates/focal_fs/src/markdown.rs::tests::spec_08_context_markdown_round_trips_without_heading_management` | `crates/focal_fs/tests/graph.rs::init_open_add_read_and_update_nodes`<br>`crates/focal_fs/tests/graph.rs::question_answer_updates_keep_identity_paths_and_managed_sections`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_crud_namespace_sort_and_stable_filename` |
| 16 | `crates/focal_fs/src/fs_utils.rs::tests::spec_06_directory_names_use_readable_unique_slugs_and_authoritative_ids` | `crates/focal_fs/tests/graph.rs::root_and_child_question_answer_nodes_support_empty_answers`<br>`crates/focal_fs/tests/graph.rs::rejects_invalid_inputs_and_ids`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_crud_namespace_sort_and_stable_filename` |
| 17 | `crates/focal_fs/src/scan.rs::tests::spec_19_graph_index_sorts_nodes_and_edges_for_deterministic_discovery` | `crates/focal_fs/tests/graph.rs::init_open_add_read_and_update_nodes`<br>`crates/focal_fs/tests/graph.rs::linking_is_idempotent_and_rejects_cycles`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_crud_namespace_sort_and_stable_filename` |
| 18 | `crates/focal_fs/src/fs_utils.rs::tests::spec_18_safe_rename_moves_directories_inside_graph_root` | `crates/focal_fs/tests/graph.rs::unlinking_canonical_parent_promotes_alias_and_preserves_subtree`<br>`crates/focal_fs/tests/graph.rs::promotion_chooses_first_alias_and_rewrites_remaining_aliases` |
| 19 | `crates/focal_fs/src/model.rs::tests::spec_19_index_edge_and_problem_types_are_constructible`<br>`crates/focal_fs/src/scan.rs::tests::spec_19_graph_index_sorts_nodes_and_edges_for_deterministic_discovery`<br>`crates/focal_fs/src/scan.rs::tests::spec_19_graph_index_sorts_contexts_and_keeps_context_problems`<br>`crates/focal_types/src/lib.rs::tests::graph_index_and_errors_include_context_variants` | `crates/focal_fs/tests/graph.rs::rebuild_index_reports_sorted_nodes_edges_and_alias_edges`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_crud_namespace_sort_and_stable_filename` |
| 20 | `crates/focal_fs/src/error.rs::tests::spec_20_graph_error_display_and_source_preserve_io_error`<br>`crates/focal_fs/src/error.rs::tests::spec_20_path_errors_include_repair_context`<br>`crates/focal_types/src/lib.rs::tests::graph_index_and_errors_include_context_variants` | `crates/focal_fs/tests/graph.rs::rejects_invalid_inputs_and_ids`<br>`crates/focal_fs/tests/graph.rs::duplicate_canonical_errors_are_preserved`<br>`crates/focal_fs/tests/graph.rs::context_documents_reject_invalid_inputs_and_report_corrupt_files`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_reject_invalid_inputs_and_report_bad_rows` |
| 21 | `crates/focal_fs/src/fs_utils.rs::tests::spec_21_atomic_write_replaces_contents_without_temp_files` | `crates/focal_fs/tests/graph.rs::init_open_add_read_and_update_nodes`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_sqlite/tests/graph.rs::failed_multi_row_write_rolls_back_transaction` |
| 22 | `crates/focal_fs/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_fs/tests/graph.rs::multiple_open_graph_handles_can_read_the_same_graph` |
| 23 | `crates/focal_fs/src/fs_utils.rs::tests::spec_05_and_23_symlinks_are_relative_directory_links_inside_graph` | `crates/focal_fs/tests/graph.rs::symlink_targets_outside_graph_are_rejected` |
| 24 | `crates/focal_fs/src/scan.rs::tests::spec_13_and_24_cycle_detection_records_manual_cycles`<br>`crates/focal_fs/src/scan.rs::tests::spec_19_graph_index_sorts_contexts_and_keeps_context_problems` | `crates/focal_fs/tests/graph.rs::validation_reports_manual_filesystem_problems`<br>`crates/focal_fs/tests/graph.rs::validation_rejects_bad_directory_suffix_and_duplicate_metadata`<br>`crates/focal_fs/tests/graph.rs::context_documents_reject_invalid_inputs_and_report_corrupt_files`<br>`crates/focal_sqlite/tests/graph.rs::open_graph_requires_context_schema_table`<br>`crates/focal_sqlite/tests/graph.rs::open_graph_rejects_context_schema_without_required_constraints`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_reject_invalid_inputs_and_report_bad_rows`<br>`crates/focal_sqlite/tests/graph.rs::rebuild_index_reports_sqlite_storage_problems` |
| 25 | `crates/focal_fs/src/fs_utils.rs::tests::spec_25_safe_remove_rejects_paths_outside_graph_root` | `crates/focal_fs/tests/graph.rs::symlink_targets_outside_graph_are_rejected`<br>`crates/focal_fs/tests/graph.rs::recursive_delete_removes_private_subtree_without_following_outside_symlink`<br>`crates/focal_fs/tests/graph.rs::context_directory_and_files_do_not_follow_symlinks` |
| 26 | `crates/focal_fs/src/lib.rs::tests::spec_26_spec_tracks_unit_and_integration_test_traceability` | `crates/focal_fs/tests/graph.rs::spec_traceability_table_points_each_section_to_tests`<br>`crates/focal_core/tests/fs.rs::fs_backend_dispatches_shared_operations`<br>`crates/focal_core/tests/fs.rs::fs_backend_dispatches_context_operations`<br>`crates/focal_core/tests/fs.rs::fs_backend_error_wrapper_converts_with_from`<br>`crates/focal_core/tests/sqlite.rs::sqlite_backend_dispatches_shared_operations`<br>`crates/focal_core/tests/sqlite.rs::sqlite_backend_dispatches_context_operations`<br>`crates/focal_core/tests/sqlite.rs::sqlite_backend_error_wrapper_converts_with_from`<br>`crates/focal_core/tests/disabled_sqlite.rs::disabled_sqlite_backend_returns_disabled_error_for_every_operation`<br>`crates/focal_core/tests/source.rs::public_core_code_avoids_panic_style_recoverable_paths` |
| 27 | `crates/focal_fs/src/model.rs::tests::spec_09_model_types_are_plain_constructible_values` | `crates/focal_fs/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 28 | `crates/focal_fs/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_fs/tests/graph.rs::example_usage_from_spec_runs_as_documented`<br>`crates/focal_fs/tests/graph.rs::context_documents_crud_list_sort_and_preserve_stable_filename`<br>`crates/focal_fs/tests/graph.rs::rebuild_index_reports_sorted_nodes_edges_and_alias_edges`<br>`crates/focal_sqlite/tests/graph.rs::context_documents_crud_namespace_sort_and_stable_filename` |

## 27. Example Usage

```rust
let graph = init_graph("./ideas")?;

let context_id = add_context_document(
    &graph,
    NewContextDocument {
        title: "Raw planning notes".to_string(),
        markdown: "A messy prompt, unresolved questions, and examples.".to_string(),
    },
)?;

let rust_id = add_root_node(
    &graph,
    NewNode {
        kind: NodeKind::Statement,
        title: "Rust keeps local tools simple".to_string(),
        content: NodeContent::Statement {
            body: "Rust can manage local files without a background service.".to_string(),
        },
    },
)?;

let qa_id = add_child_node(
    &graph,
    &rust_id,
    NewNode {
        kind: NodeKind::QuestionAnswer,
        title: "Why use symlinks?".to_string(),
        content: NodeContent::QuestionAnswer {
            question: "Why use symlinks for shared children?".to_string(),
            answer: "They preserve one canonical Markdown file while allowing multiple parents.".to_string(),
        },
    },
)?;

let descendants = list_descendants(
    &graph,
    &rust_id,
    TraversalOptions {
        max_depth: None,
    },
)?;

assert_eq!(descendants[0].id, qa_id);

let contexts = list_context_documents(&graph)?;
assert_eq!(contexts[0].id, context_id);
```

## 28. Acceptance Criteria

The shared graph behavior is acceptable when:

- A graph can be initialized in empty backend storage.
- For `focal-fs`, a graph can be initialized in an empty directory.
- For `focal-fs`, graph initialization creates top-level `roots/` and `context/` directories.
- For `focal-fs`, opening an otherwise valid graph root auto-creates a missing top-level `context/` directory.
- For `focal-fs`, no `.idea-graph/VERSION` file is required.
- Multiple original idea context Markdown documents can be added, read, updated, listed, and deleted.
- Statement and question-answer nodes can be added, read, updated, and deleted.
- Generated node IDs are UUID strings.
- Generated context document IDs are UUID strings.
- Context documents are graph-level only and are not linked to individual nodes.
- Context document title edits do not rename filesystem files or SQLite logical filenames.
- For `focal-fs`, context documents live under `<graph-root>/context/` as Markdown files.
- For `focal-fs`, every node is represented by a directory containing `node.md` and `children/`.
- For `focal-fs`, root nodes live under `roots/`.
- For `focal-fs`, child nodes live under a parent's `children/`.
- For `focal-fs`, shared children are represented with symlink entries.
- For `focal-sqlite`, nodes and placements are stored in SQLite instead of the filesystem.
- For `focal-sqlite`, context documents are stored in rows scoped to the graph namespace.
- For `focal-sqlite`, shared children are represented with alias placement rows.
- `focal-core` exposes a concrete `Backend` enum with filesystem and SQLite variants.
- `focal-core` always includes filesystem support and gates SQLite support behind a `sqlite` Cargo feature.
- `focal-core` keeps `Backend::Sqlite` present when the `sqlite` feature is disabled by using an empty SQLite placeholder that returns disabled-SQLite errors for graph operations.
- `focal-core` exposes the shared graph operations as plain free functions that take `Backend`.
- Applications can add, read, edit, link, unlink, delete, list, traverse, validate, and manage context documents through `focal-core` without backend-specific dispatch code.
- `focal-core` handles missing values and backend failures with `Result<T, focal_core::Error>` instead of panics or public `Option` return values.
- `focal_core::Error` preserves backend failures as typed variants for `focal_fs::Error` and, when enabled, `focal_sqlite::Error`.
- Canonical node promotion works when deleting or unlinking canonical parents with remaining aliases.
- The library can list roots, children, parents, ancestors, and descendants.
- Traversal is deterministic, breadth-first, and deduplicated.
- Link operations reject cycles.
- Delete operations do not remove shared descendants unless explicitly targeted.
- Markdown remains human-readable after library edits.
- The graph can be validated for common backend consistency problems.
- Destructive operations are constrained to the graph root for `focal-fs` and to the configured SQLite graph namespace for `focal-sqlite`.
- The first `focal-fs` version supports macOS and Linux only.
