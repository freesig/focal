# Idea Graph Rust Library Specification

## 1. Purpose

Build a small Rust library for storing and navigating an idea graph on disk.

The graph is represented as plain folders, Markdown files, and symbolic links. The library provides simple Rust functions for reading, adding, editing, linking, unlinking, deleting, and traversing nodes.

The implementation should be understandable, dependency-light, and usable from other Rust applications without requiring a daemon, database, async runtime, CLI, or web server.

## 2. Goals

- Store every idea node as human-readable Markdown.
- Represent parent-child structure using the filesystem.
- Keep each node in its own folder.
- Keep each node's children in a dedicated child subfolder.
- Use symlinks when a node has more than one parent.
- Support statement nodes and question-answer nodes.
- Support graph navigation from any node.
- Support listing ancestors and descendants.
- Preserve stable node identity when folders or titles change.
- Keep the graph usable by humans editing files manually, while making library-created changes consistent.

## 3. Non-Goals

- No database backend.
- No network service.
- No required CLI.
- No rich Markdown rendering.
- No collaborative multi-writer synchronization.
- No automatic semantic analysis of ideas.
- No support for arbitrary graph cycles in the first version.

## 4. Terminology

- **Graph root**: The root directory containing the whole idea graph.
- **Node**: One idea entry. A node is either a statement or a question-answer pair.
- **Node directory**: The directory that contains one node's `node.md` file and `children/` directory.
- **Canonical node directory**: The real directory that owns the node's Markdown and children.
- **Alias node directory**: A symlink to a canonical node directory.
- **Parent edge**: A relationship from one node to one child node.
- **Child edge**: The same relationship viewed from the parent node.
- **Root node**: A node with no parent.
- **Ancestor**: A parent, parent's parent, and so on.
- **Descendant**: A child, child's child, and so on.
- **Slug**: A filesystem-safe title fragment used in directory names.
- **Node ID**: Stable, filesystem-safe identifier for a node. The ID must not change when the title, Markdown, or folder slug changes.

## 5. Filesystem Layout

The graph root uses this layout:

```text
<graph-root>/
  roots/
    <slug>--<node-id>/
      node.md
      children/
        <child-slug>--<child-id>/
          node.md
          children/
        <linked-child-slug>--<linked-child-id> -> ../../somewhere/<canonical-child-dir>
```

Required directories:

- `<graph-root>/roots/`

Required files:

- `<node-dir>/node.md`

Required child directory:

- `<node-dir>/children/`

Optional metadata directory:

- `<graph-root>/.idea-graph/`

No `VERSION` file is required in the first version. The `.idea-graph/` directory is reserved for optional metadata such as lock files, but a graph is identified by the presence of `roots/` and valid node directories.

Every real node directory must contain both `node.md` and `children/`.

Every symlink node entry must point to a canonical node directory, not directly to `node.md`.

The library must create relative symlinks where possible so a graph can be moved as a folder without breaking links.

## 6. Node Directory Naming

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

Node IDs must be:

- Stable for the lifetime of a node.
- Unique within a graph.
- Safe for use in directory names.
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

## 8. Markdown Format

Each node is stored in `node.md`.

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

## 9. Rust Data Model

The public API should expose plain Rust types.

```rust
use std::path::PathBuf;

pub type NodeId = String;

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
```

The implementation may add private fields, but public types should stay small and easy to construct in tests.

## 10. Public API

The library should provide an opaque graph handle and simple functions.

```rust
use std::path::Path;

pub struct IdeaGraph {
    root: std::path::PathBuf,
}

pub fn init_graph(root: impl AsRef<Path>) -> Result<IdeaGraph, GraphError>;

pub fn open_graph(root: impl AsRef<Path>) -> Result<IdeaGraph, GraphError>;

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

`rebuild_index` may be public even if the first implementation does not persist an index. It gives callers a way to validate and inspect the graph.

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

When unlinking a child would leave it with no parents, move the child's canonical directory under `roots/`.

### `DeleteIfNoParents`

When unlinking a child would leave it with no parents, delete the child according to `DeleteMode::Recursive`.

### `FailIfWouldOrphan`

When unlinking a child would leave it with no parents, abort with `GraphError::WouldOrphanNode`.

## 12. Symlink Semantics

A node with one parent may be represented as a real directory under that parent's `children/`.

A node with multiple parents must have:

- One canonical real directory.
- One symlink entry for each additional parent.

When `link_existing_node(parent_id, child_id)` is called:

- If the edge already exists, return success without changing the filesystem.
- If `parent_id == child_id`, return `GraphError::CycleDetected`.
- If the link would make an ancestor a descendant of itself, return `GraphError::CycleDetected`.
- Create a symlink in `<parent-dir>/children/`.
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
- Traversal through symlinks must behave the same as traversal through real directories.
- If malformed manual edits create a missing target, traversal should return `GraphError::BrokenSymlink` unless the caller later gets an option to ignore broken links.

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
- Do not rename the canonical node directory.
- Do not rename alias symlinks.
- Keep the node ID unchanged.
- Keep parent and child relationships unchanged.
- The directory slug may become stale after a title edit.

Markdown rewrite rules:

- Metadata should be rewritten in a stable field order.
- Existing content should be preserved where possible.
- For statement nodes, preserve body Markdown.
- For question-answer nodes, preserve question and answer Markdown except for the managed `## Question` and `## Answer` section headings.

## 16. Add Semantics

Adding a root node:

- Creates a canonical node directory under `<graph-root>/roots/`.
- Writes `node.md`.
- Creates `children/`.
- Returns the generated node ID.

Adding a child node:

- Finds the parent's canonical directory.
- Creates a canonical node directory under `<parent-dir>/children/`.
- Writes `node.md`.
- Creates `children/`.
- Returns the generated node ID.

Input validation:

- Title must not be empty after trimming.
- Statement body may be empty, but should be allowed.
- Question text must not be empty after trimming.
- Answer text may be empty if the caller wants to capture unanswered questions, but the `## Answer` section must exist.
- Node ID must be unique.

## 17. Read Semantics

`read_node` must:

- Locate a node by ID anywhere in the graph.
- Parse metadata.
- Parse content according to `kind`.
- Return canonical path and alias paths.
- Return an error if multiple real canonical directories claim the same ID.
- Return an error if the node is found only as a broken symlink.

`list_roots` must:

- Return all nodes under `<graph-root>/roots/`.
- Include canonical root directories.
- Include promoted roots.
- Exclude broken symlinks unless returning a validation error.

## 18. Limited Movement and Promotion

The first version does not expose general node move behavior.

Automatic movement is limited to:

- Promotion when deleting or unlinking a canonical parent while alias parents remain.
- Moving an orphaned canonical node to `roots/` when `OrphanPolicy::MoveToRoots` is explicitly requested.

Promotion requirements:

- If a canonical node directory is removed from one parent while alias parents remain, choose the lexicographically first alias path as the new canonical location.
- Replace that alias symlink with the real canonical directory.
- Rewrite other aliases to point to the new canonical directory.
- Preserve the promoted node's `node.md` and full child subtree.
- Keep the promoted node ID unchanged.
- Keep child node IDs unchanged.
- Keep Markdown unchanged except for `updated_at_unix` if the operation intentionally records structural edits.
- Title edits must not trigger directory moves in the first version.

## 19. Indexing and Discovery

The library must discover nodes by scanning the filesystem on demand.

The first version must not require an in-memory index or a persistent index for normal operation.

`GraphIndex` is a transient validation and inspection result returned by `rebuild_index`.

```rust
#[derive(Debug, Clone)]
pub struct GraphIndex {
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
    DuplicateCanonicalNode { id: NodeId, paths: Vec<PathBuf> },
    MissingNodeMarkdown { path: PathBuf },
    MissingChildrenDirectory { path: PathBuf },
    InvalidMarkdown { path: PathBuf, reason: String },
    CycleDetected { node_id: NodeId },
}
```

Scan rules:

- Scan `roots/` recursively.
- Record real directories as canonical candidates.
- Record symlink entries as edges to canonical targets.
- Do not recursively descend into symlink entries after recording the edge.
- Deduplicate by node ID.
- Validate that every real node directory has `node.md` and `children/`.

## 20. Error Handling

All public functions return `Result<T, GraphError>`.

```rust
#[derive(Debug)]
pub enum GraphError {
    Io(std::io::Error),
    InvalidGraphRoot(String),
    NodeNotFound(String),
    ParentNotFound(String),
    ChildNotFound(String),
    DuplicateNodeId(String),
    InvalidNodeId(String),
    InvalidTitle,
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
    DuplicateCanonicalNode { id: String, paths: Vec<std::path::PathBuf> },
}
```

The implementation should provide `Display` and `std::error::Error` implementations.

`GraphError::Io` should preserve the original `std::io::Error`.

## 21. Atomicity and Consistency

The first version uses best-effort filesystem operations.

Requirements:

- Create directories before writing `node.md`.
- Create symlinks only after the target node exists.
- Do not require rollback for multi-step directory or symlink operations.
- Do not require transactional guarantees.
- After an operation fails, callers should use `rebuild_index` to validate and repair the graph.
- Error values should include enough path information to help manual repair where possible.
- The implementation may use atomic file writes for Markdown, but atomic file writes are not required by this version of the spec.

## 22. Concurrency

The first version only needs to support one writer process at a time.

Requirements:

- Multiple readers are acceptable.
- Concurrent writes are not guaranteed safe.
- The library should document this limitation.
- If a simple lock is implemented, use an exclusive lock file created with `create_new(true)` under `.idea-graph/write.lock`.
- Stale lock handling can be deferred.

## 23. Platform Requirements

Supported targets:

- Unix-like systems with directory symlink support.

Supported platforms:

- macOS
- Linux

Windows support:

- Windows is unsupported in the first version.
- The library may return `GraphError::SymlinkUnsupported` on unsupported platforms.

The code should isolate platform-specific symlink creation behind a small internal function.

## 24. Validation

The library should expose a validation path through `rebuild_index`.

Validation should detect:

- Missing `roots/`.
- Node directories without `node.md`.
- Node directories without `children/`.
- Invalid Markdown metadata.
- Duplicate node IDs.
- Broken symlinks.
- Cycles.
- Edges pointing outside the graph root.

By default, symlink targets outside the graph root must be rejected.

## 25. Security and Safety

The library must treat the graph root as a boundary.

Requirements:

- Do not follow symlinks outside the graph root.
- Reject `..` path traversal in generated or user-supplied IDs.
- Never delete paths outside the graph root.
- Canonicalize paths before destructive operations.
- Validate that a delete target is inside the graph root.
- Avoid shelling out to system commands.

## 26. Testing Requirements

The implementation should include tests for the following behavior.

Initialization:

- `init_graph` creates `roots/`.
- `init_graph` does not require `.idea-graph/VERSION`.
- `open_graph` fails for a non-graph directory.

Adding:

- Add a root statement node.
- Add a root question-answer node.
- Add a child statement node.
- Add a child question-answer node.
- Verify generated node IDs are UUID strings.
- Reject empty titles.
- Reject empty questions for `qa` nodes.

Reading:

- Read a statement node from its ID.
- Read a question-answer node from its ID.
- Read a node through a symlink parent and get the canonical content.

Editing:

- Update statement body.
- Update question text.
- Update answer text.
- Update title and verify metadata updates.
- Verify title edits do not require a top-level `# Heading`.
- Verify title edits do not rename the node directory.
- Verify node ID remains stable across edits.

Linking:

- Link an existing child under a second parent.
- Verify a symlink is created.
- Verify `list_parents` returns both parents.
- Verify duplicate link is idempotent.
- Reject a link that would create a cycle.

Unlinking:

- Remove a symlink parent and keep the canonical node.
- Remove a canonical parent while an alias parent remains and verify promotion.
- Verify promoted nodes preserve `node.md` and the full child subtree.
- Verify promotion rewrites remaining alias symlinks.
- Verify promotion with multiple aliases chooses the lexicographically first alias path.
- Move an orphaned node to roots when requested.
- Fail when unlinking would orphan a node and `FailIfWouldOrphan` is used.

Deleting:

- Delete a leaf node.
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
- Detect broken symlinks.
- Detect missing `children/`.
- Detect malformed `node.md`.
- Detect manual cycle and avoid infinite traversal.

Safety:

- Reject symlink targets outside graph root.
- Never delete outside graph root.

## Spec Test Traceability

Each row points to at least one in-crate unit test and one public API integration test for the numbered section.

| Section | Unit tests | Integration tests |
|---|---|---|
| 1 | `crates/focal_core/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_core/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 2 | `crates/focal_core/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_core/tests/graph.rs::init_open_add_read_and_update_nodes` |
| 3 | `crates/focal_core/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_core/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 4 | `crates/focal_core/src/model.rs::tests::spec_09_model_types_are_plain_constructible_values` | `crates/focal_core/tests/graph.rs::linking_is_idempotent_and_rejects_cycles` |
| 5 | `crates/focal_core/src/fs_utils.rs::tests::spec_05_and_23_symlinks_are_relative_directory_links_inside_graph` | `crates/focal_core/tests/graph.rs::init_open_add_read_and_update_nodes` |
| 6 | `crates/focal_core/src/fs_utils.rs::tests::spec_06_directory_names_use_readable_unique_slugs_and_authoritative_ids` | `crates/focal_core/tests/graph.rs::broken_symlink_name_is_not_reused_for_new_link` |
| 7 | `crates/focal_core/src/fs_utils.rs::tests::spec_07_node_id_validation_accepts_uuid_strings_only` | `crates/focal_core/tests/graph.rs::rejects_invalid_inputs_and_ids` |
| 8 | `crates/focal_core/src/markdown.rs::tests::spec_08_statement_markdown_round_trips_without_heading_management`<br>`crates/focal_core/src/markdown.rs::tests::spec_08_question_answer_markdown_requires_managed_sections` | `crates/focal_core/tests/graph.rs::root_and_child_question_answer_nodes_support_empty_answers`<br>`crates/focal_core/tests/graph.rs::question_answer_updates_keep_identity_paths_and_managed_sections` |
| 9 | `crates/focal_core/src/model.rs::tests::spec_09_model_types_are_plain_constructible_values` | `crates/focal_core/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 10 | `crates/focal_core/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_core/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 11 | `crates/focal_core/src/model.rs::tests::spec_11_delete_and_orphan_modes_are_copyable_contract_values` | `crates/focal_core/tests/graph.rs::delete_modes_handle_leaf_and_non_leaf_nodes`<br>`crates/focal_core/tests/graph.rs::unlink_orphan_policies_move_fail_and_delete` |
| 12 | `crates/focal_core/src/fs_utils.rs::tests::spec_05_and_23_symlinks_are_relative_directory_links_inside_graph` | `crates/focal_core/tests/graph.rs::linking_is_idempotent_and_rejects_cycles`<br>`crates/focal_core/tests/graph.rs::promotion_chooses_first_alias_and_rewrites_remaining_aliases` |
| 13 | `crates/focal_core/src/scan.rs::tests::spec_13_and_24_cycle_detection_records_manual_cycles` | `crates/focal_core/tests/graph.rs::linking_is_idempotent_and_rejects_cycles`<br>`crates/focal_core/tests/graph.rs::ancestors_descendants_deduplicate_shared_paths_and_ignore_manual_cycle_start` |
| 14 | `crates/focal_core/src/model.rs::tests::spec_14_traversal_options_default_has_no_depth_limit` | `crates/focal_core/tests/graph.rs::question_answer_nodes_and_traversal_are_deterministic`<br>`crates/focal_core/tests/graph.rs::ancestors_descendants_deduplicate_shared_paths_and_ignore_manual_cycle_start` |
| 15 | `crates/focal_core/src/markdown.rs::tests::spec_08_statement_markdown_round_trips_without_heading_management` | `crates/focal_core/tests/graph.rs::init_open_add_read_and_update_nodes`<br>`crates/focal_core/tests/graph.rs::question_answer_updates_keep_identity_paths_and_managed_sections` |
| 16 | `crates/focal_core/src/fs_utils.rs::tests::spec_06_directory_names_use_readable_unique_slugs_and_authoritative_ids` | `crates/focal_core/tests/graph.rs::root_and_child_question_answer_nodes_support_empty_answers`<br>`crates/focal_core/tests/graph.rs::rejects_invalid_inputs_and_ids` |
| 17 | `crates/focal_core/src/scan.rs::tests::spec_19_graph_index_sorts_nodes_and_edges_for_deterministic_discovery` | `crates/focal_core/tests/graph.rs::init_open_add_read_and_update_nodes`<br>`crates/focal_core/tests/graph.rs::linking_is_idempotent_and_rejects_cycles` |
| 18 | `crates/focal_core/src/fs_utils.rs::tests::spec_18_safe_rename_moves_directories_inside_graph_root` | `crates/focal_core/tests/graph.rs::unlinking_canonical_parent_promotes_alias_and_preserves_subtree`<br>`crates/focal_core/tests/graph.rs::promotion_chooses_first_alias_and_rewrites_remaining_aliases` |
| 19 | `crates/focal_core/src/model.rs::tests::spec_19_index_edge_and_problem_types_are_constructible`<br>`crates/focal_core/src/scan.rs::tests::spec_19_graph_index_sorts_nodes_and_edges_for_deterministic_discovery` | `crates/focal_core/tests/graph.rs::rebuild_index_reports_sorted_nodes_edges_and_alias_edges` |
| 20 | `crates/focal_core/src/error.rs::tests::spec_20_graph_error_display_and_source_preserve_io_error`<br>`crates/focal_core/src/error.rs::tests::spec_20_path_errors_include_repair_context` | `crates/focal_core/tests/graph.rs::rejects_invalid_inputs_and_ids`<br>`crates/focal_core/tests/graph.rs::duplicate_canonical_errors_are_preserved` |
| 21 | `crates/focal_core/src/fs_utils.rs::tests::spec_21_atomic_write_replaces_contents_without_temp_files` | `crates/focal_core/tests/graph.rs::init_open_add_read_and_update_nodes` |
| 22 | `crates/focal_core/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_core/tests/graph.rs::multiple_open_graph_handles_can_read_the_same_graph` |
| 23 | `crates/focal_core/src/fs_utils.rs::tests::spec_05_and_23_symlinks_are_relative_directory_links_inside_graph` | `crates/focal_core/tests/graph.rs::symlink_targets_outside_graph_are_rejected` |
| 24 | `crates/focal_core/src/scan.rs::tests::spec_13_and_24_cycle_detection_records_manual_cycles` | `crates/focal_core/tests/graph.rs::validation_reports_manual_filesystem_problems`<br>`crates/focal_core/tests/graph.rs::validation_rejects_bad_directory_suffix_and_duplicate_metadata` |
| 25 | `crates/focal_core/src/fs_utils.rs::tests::spec_25_safe_remove_rejects_paths_outside_graph_root` | `crates/focal_core/tests/graph.rs::symlink_targets_outside_graph_are_rejected`<br>`crates/focal_core/tests/graph.rs::recursive_delete_removes_private_subtree_without_following_outside_symlink` |
| 26 | `crates/focal_core/src/lib.rs::tests::spec_26_spec_tracks_unit_and_integration_test_traceability` | `crates/focal_core/tests/graph.rs::spec_traceability_table_points_each_section_to_tests` |
| 27 | `crates/focal_core/src/model.rs::tests::spec_09_model_types_are_plain_constructible_values` | `crates/focal_core/tests/graph.rs::example_usage_from_spec_runs_as_documented` |
| 28 | `crates/focal_core/src/lib.rs::tests::spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only` | `crates/focal_core/tests/graph.rs::example_usage_from_spec_runs_as_documented`<br>`crates/focal_core/tests/graph.rs::rebuild_index_reports_sorted_nodes_edges_and_alias_edges` |

## 27. Example Usage

```rust
let graph = init_graph("./ideas")?;

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
```

## 28. Acceptance Criteria

The library is acceptable when:

- A graph can be initialized in an empty directory.
- No `.idea-graph/VERSION` file is required.
- Statement and question-answer nodes can be added, read, updated, and deleted.
- Generated node IDs are UUID strings.
- Every node is represented by a directory containing `node.md` and `children/`.
- Root nodes live under `roots/`.
- Child nodes live under a parent's `children/`.
- Shared children are represented with symlink entries.
- Canonical node promotion works when deleting or unlinking canonical parents with remaining aliases.
- The library can list roots, children, parents, ancestors, and descendants.
- Traversal is deterministic, breadth-first, and deduplicated.
- Link operations reject cycles.
- Delete operations do not remove shared descendants unless explicitly targeted.
- Markdown remains human-readable after library edits.
- The graph can be validated for common manual-edit problems.
- Destructive operations are constrained to the graph root.
- The first version supports macOS and Linux only.
