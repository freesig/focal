//! SQLite-backed idea graph library.
//!
//! This crate stores idea graphs in named SQLite namespaces using a
//! caller-provided `rusqlite::Connection`. Graph handles borrow the connection;
//! callers remain responsible for opening, configuring, and closing it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub use focal_types::{
    ContextDocument, ContextDocumentPatch, ContextId, ContextSummary, DeleteMode, GraphEdge,
    GraphError, GraphIndex, GraphProblem, NewContextDocument, NewNode, Node, NodeContent, NodeId,
    NodeKind, NodePatch, NodeSummary, OrphanPolicy, TraversalOptions,
};
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

#[derive(Debug)]
pub struct Error {
    source: GraphError,
}

impl Error {
    pub fn as_graph_error(&self) -> &GraphError {
        &self.source
    }

    pub fn into_graph_error(self) -> GraphError {
        self.source
    }
}

impl From<GraphError> for Error {
    fn from(source: GraphError) -> Self {
        Self { source }
    }
}

impl From<Error> for GraphError {
    fn from(error: Error) -> Self {
        error.source
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, formatter)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

const MAX_SLUG_BYTES: usize = 80;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS focal_graphs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at_unix INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS focal_nodes (
    graph_id INTEGER NOT NULL,
    id TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    reviewed INTEGER NOT NULL,
    statement_body TEXT,
    qa_question TEXT,
    qa_answer TEXT,
    created_at_unix INTEGER NOT NULL,
    updated_at_unix INTEGER NOT NULL,
    PRIMARY KEY (graph_id, id),
    FOREIGN KEY (graph_id) REFERENCES focal_graphs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS focal_qa_alternative_answers (
    graph_id INTEGER NOT NULL,
    node_id TEXT NOT NULL,
    answer_order INTEGER NOT NULL,
    answer TEXT NOT NULL,
    PRIMARY KEY (graph_id, node_id, answer_order),
    FOREIGN KEY (graph_id) REFERENCES focal_graphs(id) ON DELETE CASCADE,
    FOREIGN KEY (graph_id, node_id) REFERENCES focal_nodes(graph_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS focal_context_documents (
    graph_id INTEGER NOT NULL,
    id TEXT NOT NULL,
    slug TEXT NOT NULL,
    filename TEXT NOT NULL,
    title TEXT NOT NULL,
    markdown TEXT NOT NULL,
    created_at_unix INTEGER NOT NULL,
    updated_at_unix INTEGER NOT NULL,
    PRIMARY KEY (graph_id, id),
    UNIQUE (graph_id, filename),
    FOREIGN KEY (graph_id) REFERENCES focal_graphs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS focal_placements (
    id INTEGER PRIMARY KEY,
    graph_id INTEGER NOT NULL,
    node_id TEXT NOT NULL,
    parent_id TEXT,
    slug TEXT NOT NULL,
    logical_path TEXT NOT NULL,
    is_canonical INTEGER NOT NULL,
    FOREIGN KEY (graph_id) REFERENCES focal_graphs(id) ON DELETE CASCADE,
    FOREIGN KEY (graph_id, node_id) REFERENCES focal_nodes(graph_id, id) ON DELETE CASCADE,
    FOREIGN KEY (graph_id, parent_id) REFERENCES focal_nodes(graph_id, id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS focal_placements_logical_path_unique
    ON focal_placements(graph_id, logical_path);

CREATE UNIQUE INDEX IF NOT EXISTS focal_placements_edge_unique
    ON focal_placements(graph_id, parent_id, node_id)
    WHERE parent_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS focal_placements_root_unique
    ON focal_placements(graph_id, node_id)
    WHERE parent_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS focal_placements_canonical_unique
    ON focal_placements(graph_id, node_id)
    WHERE is_canonical = 1;
"#;

pub struct IdeaGraph<'conn> {
    connection: &'conn mut Connection,
    graph_name: String,
}

impl<'conn> IdeaGraph<'conn> {
    fn connection(&self) -> &Connection {
        &*self.connection
    }

    fn connection_mut(&mut self) -> &mut Connection {
        &mut *self.connection
    }
}

pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, GraphError> {
    let connection = Connection::open(path).map_err(storage_error)?;
    enable_foreign_keys(&connection)?;
    Ok(connection)
}

pub fn init_graph<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<IdeaGraph<'conn>, GraphError> {
    validate_graph_name(graph_name)?;
    enable_foreign_keys(connection)?;
    let tx = connection.transaction().map_err(storage_error)?;
    tx.execute_batch(SCHEMA).map_err(storage_error)?;
    let now = unix_to_i64(now_unix())?;
    tx.execute(
        "INSERT OR IGNORE INTO focal_graphs (name, created_at_unix) VALUES (?1, ?2)",
        params![graph_name, now],
    )
    .map_err(storage_error)?;
    tx.commit().map_err(storage_error)?;
    Ok(IdeaGraph {
        connection,
        graph_name: graph_name.to_string(),
    })
}

pub fn open_graph<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<IdeaGraph<'conn>, GraphError> {
    validate_graph_name(graph_name)?;
    enable_foreign_keys(connection)?;
    ensure_schema_exists(connection)?;
    require_graph_id(connection, graph_name)?;
    Ok(IdeaGraph {
        connection,
        graph_name: graph_name.to_string(),
    })
}

pub fn add_context_document(
    graph: &mut IdeaGraph<'_>,
    context: NewContextDocument,
) -> Result<ContextId, GraphError> {
    validate_graph_name(&graph.graph_name)?;
    validate_new_context_document(&context)?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let id = generate_unique_context_id(&tx, graph_id)?;
    let (slug, filename) = unique_context_filename(&tx, graph_id, &context.title, &id)?;
    let now = now_unix();
    insert_context_document(&tx, graph_id, &id, &slug, &filename, &context, now, now)?;
    tx.commit().map_err(storage_error)?;
    Ok(id)
}

pub fn read_context_document(
    graph: &IdeaGraph<'_>,
    context_id: &str,
) -> Result<ContextDocument, GraphError> {
    validate_context_id(context_id)?;
    let graph_id = graph_id_for_graph(graph)?;
    let context_scan = load_context_scan(graph.connection(), graph_id)?;
    let context = require_context(&context_scan, context_id)?;
    Ok(context_document_from_scanned(context))
}

pub fn update_context_document(
    graph: &mut IdeaGraph<'_>,
    context_id: &str,
    patch: ContextDocumentPatch,
) -> Result<ContextDocument, GraphError> {
    validate_context_id(context_id)?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let context_scan = load_context_scan(&tx, graph_id)?;
    let context = require_context(&context_scan, context_id)?;
    let title = match patch.title {
        Some(title) => {
            validate_title(&title)?;
            title
        }
        None => context.title.clone(),
    };
    let markdown = patch.markdown.unwrap_or_else(|| context.markdown.clone());
    let updated_at_unix = now_unix().max(context.updated_at_unix.saturating_add(1));
    update_context_row(
        &tx,
        graph_id,
        context_id,
        &title,
        &markdown,
        updated_at_unix,
    )?;
    let updated = ContextDocument {
        id: context.id.clone(),
        title,
        filename: context.filename.clone(),
        markdown,
        created_at_unix: context.created_at_unix,
        updated_at_unix,
        path: context.path.clone(),
    };
    tx.commit().map_err(storage_error)?;
    Ok(updated)
}

pub fn delete_context_document(
    graph: &mut IdeaGraph<'_>,
    context_id: &str,
) -> Result<(), GraphError> {
    validate_context_id(context_id)?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let context_scan = load_context_scan(&tx, graph_id)?;
    require_context(&context_scan, context_id)?;
    let rows = tx
        .execute(
            "DELETE FROM focal_context_documents WHERE graph_id = ?1 AND id = ?2",
            params![graph_id, context_id],
        )
        .map_err(storage_error)?;
    if rows == 0 {
        return Err(GraphError::ContextNotFound(context_id.to_string()));
    }
    tx.commit().map_err(storage_error)
}

pub fn list_context_documents(graph: &IdeaGraph<'_>) -> Result<Vec<ContextSummary>, GraphError> {
    let graph_id = graph_id_for_graph(graph)?;
    let context_scan = load_context_scan(graph.connection(), graph_id)?;
    if let Some(error) = first_context_problem_error(&context_scan) {
        return Err(error);
    }
    let mut summaries = context_scan
        .contexts
        .values()
        .map(context_summary_from_scanned)
        .collect::<Vec<_>>();
    sort_context_summaries(&mut summaries);
    Ok(summaries)
}

pub fn add_root_node(graph: &mut IdeaGraph<'_>, node: NewNode) -> Result<NodeId, GraphError> {
    validate_graph_name(&graph.graph_name)?;
    validate_new_node(&node, &PathBuf::from("roots"))?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let id = generate_unique_node_id(&tx, graph_id)?;
    let now = now_unix();
    insert_node(&tx, graph_id, &id, &node, now, now)?;
    let (slug, logical_path) = unique_root_path(&tx, graph_id, &node.title, &id)?;
    insert_placement(&tx, graph_id, &id, None, &slug, &logical_path, true)?;
    tx.commit().map_err(storage_error)?;
    Ok(id)
}

pub fn add_child_node(
    graph: &mut IdeaGraph<'_>,
    parent_id: &str,
    node: NewNode,
) -> Result<NodeId, GraphError> {
    validate_node_id(parent_id)?;
    validate_new_node(&node, &PathBuf::from(parent_id))?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let scan = load_scan(&tx, graph_id)?;
    let parent = require_parent_node(&scan, parent_id)?;
    let parent_path = canonical_path(parent)
        .ok_or_else(|| GraphError::ParentNotFound(parent_id.to_string()))?
        .to_string_lossy()
        .into_owned();
    let id = generate_unique_node_id(&tx, graph_id)?;
    let now = now_unix();
    insert_node(&tx, graph_id, &id, &node, now, now)?;
    let (slug, logical_path) = unique_child_path(&tx, graph_id, &parent_path, &node.title, &id)?;
    insert_placement(
        &tx,
        graph_id,
        &id,
        Some(parent_id),
        &slug,
        &logical_path,
        true,
    )?;
    tx.commit().map_err(storage_error)?;
    Ok(id)
}

pub fn read_node(graph: &IdeaGraph<'_>, node_id: &str) -> Result<Node, GraphError> {
    validate_node_id(node_id)?;
    let graph_id = graph_id_for_graph(graph)?;
    let scan = load_scan(graph.connection(), graph_id)?;
    let node = canonical_node(&scan, node_id)?;
    node_from_scanned(node)
}

pub fn update_node(
    graph: &mut IdeaGraph<'_>,
    node_id: &str,
    patch: NodePatch,
) -> Result<Node, GraphError> {
    validate_node_id(node_id)?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let scan = load_scan(&tx, graph_id)?;
    let node = canonical_node(&scan, node_id)?;
    let canonical = canonical_path(node)
        .ok_or_else(|| GraphError::NodeNotFound(node_id.to_string()))?
        .to_path_buf();
    let title = match patch.title {
        Some(title) => {
            validate_title(&title)?;
            title
        }
        None => node.title.clone(),
    };
    let content = match patch.content {
        Some(content) => {
            validate_content_matches(&node.kind, &content, &canonical)?;
            validate_content_values(&content, &canonical)?;
            content
        }
        None => node.content.clone(),
    };
    let reviewed = patch.reviewed.unwrap_or(node.reviewed);
    let updated_at_unix = now_unix().max(node.updated_at_unix.saturating_add(1));
    update_node_row(
        &tx,
        graph_id,
        node_id,
        &title,
        reviewed,
        &content,
        updated_at_unix,
    )?;
    let updated = Node {
        id: node.id.clone(),
        kind: node.kind.clone(),
        title,
        reviewed,
        content,
        created_at_unix: node.created_at_unix,
        updated_at_unix,
        canonical_path: canonical,
        alias_paths: node.alias_paths(),
    };
    tx.commit().map_err(storage_error)?;
    Ok(updated)
}

pub fn delete_node(
    graph: &mut IdeaGraph<'_>,
    node_id: &str,
    mode: DeleteMode,
) -> Result<(), GraphError> {
    validate_node_id(node_id)?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let scan = load_scan(&tx, graph_id)?;
    canonical_node(&scan, node_id)?;

    if mode == DeleteMode::FailIfHasChildren && !child_edges(&scan, node_id).is_empty() {
        return Err(GraphError::NodeHasChildren(node_id.to_string()));
    }

    let delete_set = match mode {
        DeleteMode::FailIfHasChildren => BTreeSet::from([node_id.to_string()]),
        DeleteMode::Recursive => compute_delete_set(&scan, node_id),
    };
    perform_delete_set(&tx, graph_id, &scan, &delete_set)?;
    tx.commit().map_err(storage_error)
}

pub fn link_existing_node(
    graph: &mut IdeaGraph<'_>,
    parent_id: &str,
    child_id: &str,
) -> Result<(), GraphError> {
    validate_node_id(parent_id)?;
    validate_node_id(child_id)?;
    if parent_id == child_id {
        return Err(GraphError::CycleDetected);
    }

    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let scan = load_scan(&tx, graph_id)?;
    let parent = require_parent_node(&scan, parent_id)?;
    let child = require_child_node(&scan, child_id)?;

    if find_placement(&scan, parent_id, child_id).is_some() {
        tx.commit().map_err(storage_error)?;
        return Ok(());
    }
    if reaches(&scan, child_id, parent_id) {
        return Err(GraphError::CycleDetected);
    }

    let parent_path = canonical_path(parent)
        .ok_or_else(|| GraphError::ParentNotFound(parent_id.to_string()))?
        .to_string_lossy()
        .into_owned();
    let child_canonical = canonical_placement(child)
        .ok_or_else(|| GraphError::ChildNotFound(child_id.to_string()))?;

    if parent_edges(&scan, child_id).is_empty() && child_canonical.parent_id.is_none() {
        move_root_node_under_parent(&tx, graph_id, child, parent_id, &parent_path)?;
    } else {
        let (slug, logical_path) =
            unique_child_path(&tx, graph_id, &parent_path, &child.title, child_id)?;
        insert_placement(
            &tx,
            graph_id,
            child_id,
            Some(parent_id),
            &slug,
            &logical_path,
            false,
        )?;
    }

    tx.commit().map_err(storage_error)
}

pub fn unlink_child(
    graph: &mut IdeaGraph<'_>,
    parent_id: &str,
    child_id: &str,
    orphan_policy: OrphanPolicy,
) -> Result<(), GraphError> {
    validate_node_id(parent_id)?;
    validate_node_id(child_id)?;
    let graph_name = graph.graph_name.clone();
    let tx = graph
        .connection_mut()
        .transaction()
        .map_err(storage_error)?;
    let graph_id = require_graph_id(&tx, &graph_name)?;
    let scan = load_scan(&tx, graph_id)?;
    require_parent_node(&scan, parent_id)?;
    let child = require_child_node(&scan, child_id)?;
    let placement = find_placement(&scan, parent_id, child_id)
        .ok_or_else(|| GraphError::ChildNotFound(child_id.to_string()))?;

    let remaining_parents = parent_edges(&scan, child_id)
        .into_iter()
        .filter(|candidate| candidate.row_id != placement.row_id)
        .collect::<Vec<_>>();
    let canonical_is_root = canonical_placement(child).is_some_and(|candidate| {
        candidate.parent_id.is_none() && candidate.row_id != placement.row_id
    });
    let would_orphan = remaining_parents.is_empty() && !canonical_is_root;

    if would_orphan && orphan_policy == OrphanPolicy::FailIfWouldOrphan {
        return Err(GraphError::WouldOrphanNode(child_id.to_string()));
    }
    if would_orphan && orphan_policy == OrphanPolicy::DeleteIfNoParents {
        perform_delete_set(&tx, graph_id, &scan, &compute_delete_set(&scan, child_id))?;
        tx.commit().map_err(storage_error)?;
        return Ok(());
    }

    if !placement.is_canonical {
        delete_placement(&tx, graph_id, placement.row_id)?;
        tx.commit().map_err(storage_error)?;
        return Ok(());
    }

    if !remaining_parents.is_empty() {
        let alias = remaining_parents
            .into_iter()
            .filter(|candidate| !candidate.is_canonical)
            .min_by_key(|candidate| path_sort_key(&candidate.logical_path))
            .ok_or_else(|| GraphError::AliasConflict(placement.logical_path.clone()))?;
        promote_to_alias(&tx, graph_id, child, alias)?;
        tx.commit().map_err(storage_error)?;
        return Ok(());
    }

    if orphan_policy == OrphanPolicy::MoveToRoots {
        move_node_to_roots(&tx, graph_id, child)?;
        tx.commit().map_err(storage_error)?;
        return Ok(());
    }

    delete_placement(&tx, graph_id, placement.row_id)?;
    tx.commit().map_err(storage_error)
}

pub fn list_roots(graph: &IdeaGraph<'_>) -> Result<Vec<NodeSummary>, GraphError> {
    let graph_id = graph_id_for_graph(graph)?;
    let scan = load_scan(graph.connection(), graph_id)?;
    let mut by_id = BTreeMap::new();
    for placement in scan
        .placements
        .iter()
        .filter(|placement| placement.parent_id.is_none())
    {
        if let Some(summary) = summary_for(&scan, &placement.node_id, !placement.is_canonical) {
            by_id.entry(placement.node_id.clone()).or_insert(summary);
        }
    }
    let mut summaries = by_id.into_values().collect::<Vec<_>>();
    sort_summaries(&mut summaries);
    Ok(summaries)
}

pub fn list_children(graph: &IdeaGraph<'_>, node_id: &str) -> Result<Vec<NodeSummary>, GraphError> {
    validate_node_id(node_id)?;
    let graph_id = graph_id_for_graph(graph)?;
    let scan = load_scan(graph.connection(), graph_id)?;
    canonical_node(&scan, node_id)?;
    let mut summaries = Vec::new();
    for placement in child_edges(&scan, node_id) {
        let child = canonical_node(&scan, &placement.node_id)?;
        if let Some(summary) = node_summary(child, !placement.is_canonical) {
            summaries.push(summary);
        }
    }
    sort_summaries(&mut summaries);
    Ok(summaries)
}

pub fn list_parents(graph: &IdeaGraph<'_>, node_id: &str) -> Result<Vec<NodeSummary>, GraphError> {
    validate_node_id(node_id)?;
    let graph_id = graph_id_for_graph(graph)?;
    let scan = load_scan(graph.connection(), graph_id)?;
    canonical_node(&scan, node_id)?;
    if let Some(error) = entry_problem_error_for_node_id(&scan, node_id) {
        return Err(error);
    }
    let mut summaries = Vec::new();
    for placement in parent_edges(&scan, node_id) {
        if let Some(parent_id) = placement.parent_id.as_deref() {
            let parent = canonical_node(&scan, parent_id)?;
            if let Some(summary) = node_summary(parent, false) {
                summaries.push(summary);
            }
        }
    }
    sort_summaries(&mut summaries);
    Ok(summaries)
}

pub fn list_ancestors(
    graph: &IdeaGraph<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError> {
    traverse(graph, node_id, options, TraversalDirection::Ancestors)
}

pub fn list_descendants(
    graph: &IdeaGraph<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError> {
    traverse(graph, node_id, options, TraversalDirection::Descendants)
}

pub fn rebuild_index(graph: &IdeaGraph<'_>) -> Result<GraphIndex, GraphError> {
    let graph_id = graph_id_for_graph(graph)?;
    let scan = load_scan(graph.connection(), graph_id)?;
    Ok(graph_index(&scan))
}

#[derive(Debug, Clone)]
struct ScannedContext {
    id: String,
    title: String,
    filename: String,
    markdown: String,
    created_at_unix: u64,
    updated_at_unix: u64,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct ScannedNode {
    id: String,
    kind: NodeKind,
    title: String,
    reviewed: bool,
    content: NodeContent,
    created_at_unix: u64,
    updated_at_unix: u64,
    canonical_placements: Vec<ScannedPlacement>,
    alias_placements: Vec<ScannedPlacement>,
}

impl ScannedNode {
    fn alias_paths(&self) -> Vec<PathBuf> {
        self.alias_placements
            .iter()
            .map(|placement| placement.logical_path.clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
struct ScannedPlacement {
    row_id: i64,
    node_id: String,
    parent_id: Option<String>,
    logical_path: PathBuf,
    is_canonical: bool,
}

#[derive(Debug, Clone)]
struct ScanResult {
    contexts: BTreeMap<String, ScannedContext>,
    nodes: BTreeMap<String, ScannedNode>,
    placements: Vec<ScannedPlacement>,
    problems: Vec<GraphProblem>,
    node_problems: BTreeMap<String, GraphProblem>,
}

#[derive(Debug, Clone)]
struct ContextScan {
    contexts: BTreeMap<String, ScannedContext>,
    problems: Vec<GraphProblem>,
    context_problems: BTreeMap<String, GraphProblem>,
}

#[derive(Debug)]
struct RawNode {
    id: String,
    kind: String,
    title: String,
    reviewed: i64,
    statement_body: Option<String>,
    qa_question: Option<String>,
    qa_answer: Option<String>,
    created_at_unix: i64,
    updated_at_unix: i64,
}

#[derive(Debug)]
struct RawAlternativeAnswer {
    node_id: String,
    answer_order: i64,
    answer: String,
}

#[derive(Debug)]
struct RawContext {
    id: String,
    slug: String,
    filename: String,
    title: String,
    markdown: String,
    created_at_unix: i64,
    updated_at_unix: i64,
}

#[derive(Debug, Clone)]
struct RawPlacement {
    row_id: i64,
    node_id: String,
    parent_id: Option<String>,
    slug: String,
    logical_path: String,
    is_canonical: i64,
}

fn load_scan(connection: &Connection, graph_id: i64) -> Result<ScanResult, GraphError> {
    let context_scan = load_context_scan(connection, graph_id)?;
    let raw_nodes = query_raw_nodes(connection, graph_id)?;
    let raw_alternative_answers = query_raw_alternative_answers(connection, graph_id)?;
    let raw_placements = query_raw_placements(connection, graph_id)?;
    let raw_node_ids = raw_nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();

    let mut result = ScanResult {
        contexts: context_scan.contexts,
        nodes: BTreeMap::new(),
        placements: Vec::new(),
        problems: context_scan.problems,
        node_problems: BTreeMap::new(),
    };
    let alternative_answers_by_node = group_alternative_answers(
        &mut result,
        raw_alternative_answers,
        &raw_node_ids,
        &raw_placements,
    );

    let mut seen_nodes = BTreeSet::new();
    for raw_node in raw_nodes {
        let path = diagnostic_path_for_node(&raw_placements, &raw_node.id);
        if !seen_nodes.insert(raw_node.id.clone()) {
            add_node_problem(
                &mut result,
                &raw_node.id,
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: format!("duplicate node id `{}`", raw_node.id),
                },
            );
            continue;
        }

        let alternative_answers = alternative_answers_by_node
            .get(&raw_node.id)
            .cloned()
            .unwrap_or_default();
        match scanned_node_from_raw(raw_node, alternative_answers, &path) {
            Ok(node) => {
                result.nodes.insert(node.id.clone(), node);
            }
            Err((id, problem)) => add_node_problem(&mut result, &id, problem),
        }
    }

    let mut edge_keys = BTreeSet::<(Option<String>, String)>::new();
    for raw_placement in raw_placements {
        match scanned_placement_from_raw(&raw_placement, &raw_node_ids) {
            Ok(placement) => {
                let key = (placement.parent_id.clone(), placement.node_id.clone());
                if !edge_keys.insert(key) {
                    add_node_problem(
                        &mut result,
                        &placement.node_id,
                        GraphProblem::InvalidMarkdown {
                            path: placement.logical_path.clone(),
                            reason: "duplicate placement for parent-child edge".to_string(),
                        },
                    );
                }

                if let Some(node) = result.nodes.get_mut(&placement.node_id) {
                    if placement.is_canonical {
                        node.canonical_placements.push(placement.clone());
                        node.canonical_placements
                            .sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
                    } else {
                        node.alias_placements.push(placement.clone());
                        node.alias_placements
                            .sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
                    }
                }
                result.placements.push(placement);
            }
            Err((node_id, problem)) => add_node_problem(&mut result, &node_id, problem),
        }
    }

    let ids = result.nodes.keys().cloned().collect::<Vec<_>>();
    for id in ids {
        let Some(node) = result.nodes.get(&id) else {
            continue;
        };
        if node.canonical_placements.len() > 1 {
            let problem = GraphProblem::DuplicateCanonicalNode {
                id: id.clone(),
                paths: node
                    .canonical_placements
                    .iter()
                    .map(|placement| placement.logical_path.clone())
                    .collect(),
            };
            add_node_problem(&mut result, &id, problem);
        } else if node.canonical_placements.is_empty() {
            add_node_problem(
                &mut result,
                &id,
                GraphProblem::InvalidMarkdown {
                    path: PathBuf::from(format!("nodes/{id}")),
                    reason: "node has no canonical placement".to_string(),
                },
            );
        }
    }

    validate_child_paths(&mut result);
    detect_cycles(&mut result);
    Ok(result)
}

fn load_context_scan(connection: &Connection, graph_id: i64) -> Result<ContextScan, GraphError> {
    let raw_contexts = query_raw_contexts(connection, graph_id)?;
    let mut paths_by_id = BTreeMap::<String, Vec<PathBuf>>::new();
    for raw in &raw_contexts {
        paths_by_id
            .entry(raw.id.clone())
            .or_default()
            .push(logical_context_path(&raw.filename));
    }
    for paths in paths_by_id.values_mut() {
        paths.sort();
    }

    let mut result = ContextScan {
        contexts: BTreeMap::new(),
        problems: Vec::new(),
        context_problems: BTreeMap::new(),
    };
    let mut seen_contexts = BTreeSet::new();
    for raw_context in raw_contexts {
        if !seen_contexts.insert(raw_context.id.clone()) {
            let paths = paths_by_id
                .get(&raw_context.id)
                .cloned()
                .unwrap_or_else(|| vec![logical_context_path(&raw_context.filename)]);
            add_context_problem(
                &mut result,
                &raw_context.id,
                GraphProblem::DuplicateContextDocument {
                    id: raw_context.id.clone(),
                    paths,
                },
            );
            continue;
        }

        match scanned_context_from_raw(raw_context) {
            Ok(context) => {
                result.contexts.insert(context.id.clone(), context);
            }
            Err((id, problem)) => add_context_problem(&mut result, &id, problem),
        }
    }

    Ok(result)
}

fn group_alternative_answers(
    result: &mut ScanResult,
    raw_answers: Vec<RawAlternativeAnswer>,
    raw_node_ids: &BTreeSet<String>,
    raw_placements: &[RawPlacement],
) -> BTreeMap<String, Vec<String>> {
    let mut answers_by_node = BTreeMap::<String, Vec<(i64, String)>>::new();
    let mut seen_orders = BTreeSet::<(String, i64)>::new();

    for raw in raw_answers {
        let path = diagnostic_path_for_node(raw_placements, &raw.node_id);
        if let Err(error) = validate_node_id(&raw.node_id) {
            add_node_problem(
                result,
                &raw.node_id,
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: error.to_string(),
                },
            );
            continue;
        }
        if !raw_node_ids.contains(&raw.node_id) {
            add_node_problem(
                result,
                &raw.node_id,
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: "alternative answer references missing node".to_string(),
                },
            );
            continue;
        }
        if raw.answer_order < 0 {
            add_node_problem(
                result,
                &raw.node_id,
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: format!("invalid alternative answer order `{}`", raw.answer_order),
                },
            );
            continue;
        }
        if !seen_orders.insert((raw.node_id.clone(), raw.answer_order)) {
            add_node_problem(
                result,
                &raw.node_id,
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: format!("duplicate alternative answer order `{}`", raw.answer_order),
                },
            );
            continue;
        }
        if raw.answer.trim().is_empty() {
            add_node_problem(
                result,
                &raw.node_id,
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: "alternative answer must not be empty".to_string(),
                },
            );
            continue;
        }

        answers_by_node
            .entry(raw.node_id)
            .or_default()
            .push((raw.answer_order, raw.answer));
    }

    answers_by_node
        .into_iter()
        .map(|(node_id, mut answers)| {
            answers.sort_by_key(|(answer_order, _)| *answer_order);
            (
                node_id,
                answers
                    .into_iter()
                    .map(|(_, answer)| answer)
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn scanned_context_from_raw(
    raw_context: RawContext,
) -> Result<ScannedContext, (String, GraphProblem)> {
    let id = raw_context.id;
    let path = logical_context_path(&raw_context.filename);
    if let Err(error) = validate_context_id(&id) {
        return Err((
            id,
            GraphProblem::InvalidContextMarkdown {
                path,
                reason: error.to_string(),
            },
        ));
    }
    if raw_context.slug.trim().is_empty() || raw_context.slug.chars().any(char::is_control) {
        return Err((
            id,
            GraphProblem::InvalidContextMarkdown {
                path,
                reason: "context slug must not be empty or contain control characters".to_string(),
            },
        ));
    }
    if let Err(reason) = validate_context_filename(&raw_context.filename, &id) {
        return Err((id, GraphProblem::InvalidContextMarkdown { path, reason }));
    }
    if let Err(error) = validate_title(&raw_context.title) {
        return Err((
            id,
            GraphProblem::InvalidContextMarkdown {
                path,
                reason: error.to_string(),
            },
        ));
    }
    let created_at_unix =
        non_negative_context_unix(raw_context.created_at_unix, &path, "created_at_unix")
            .map_err(|problem| (id.clone(), problem))?;
    let updated_at_unix =
        non_negative_context_unix(raw_context.updated_at_unix, &path, "updated_at_unix")
            .map_err(|problem| (id.clone(), problem))?;

    Ok(ScannedContext {
        id,
        title: raw_context.title,
        filename: raw_context.filename,
        markdown: raw_context.markdown,
        created_at_unix,
        updated_at_unix,
        path,
    })
}

fn scanned_node_from_raw(
    raw_node: RawNode,
    alternative_answers: Vec<String>,
    path: &Path,
) -> Result<ScannedNode, (String, GraphProblem)> {
    let id = raw_node.id;
    if let Err(error) = validate_node_id(&id) {
        return Err((
            id,
            GraphProblem::InvalidMarkdown {
                path: path.to_path_buf(),
                reason: error.to_string(),
            },
        ));
    }
    if let Err(error) = validate_title(&raw_node.title) {
        return Err((
            id,
            GraphProblem::InvalidMarkdown {
                path: path.to_path_buf(),
                reason: error.to_string(),
            },
        ));
    }
    let kind = match raw_node.kind.as_str() {
        "statement" => NodeKind::Statement,
        "qa" => NodeKind::QuestionAnswer,
        other => {
            return Err((
                id,
                GraphProblem::InvalidMarkdown {
                    path: path.to_path_buf(),
                    reason: format!("unsupported kind `{other}`"),
                },
            ));
        }
    };
    let created_at_unix = non_negative_unix(raw_node.created_at_unix, path, "created_at_unix")
        .map_err(|problem| (id.clone(), problem))?;
    let updated_at_unix = non_negative_unix(raw_node.updated_at_unix, path, "updated_at_unix")
        .map_err(|problem| (id.clone(), problem))?;
    let reviewed = match raw_node.reviewed {
        0 => false,
        1 => true,
        other => {
            return Err((
                id,
                GraphProblem::InvalidMarkdown {
                    path: path.to_path_buf(),
                    reason: format!("invalid reviewed value `{other}`"),
                },
            ));
        }
    };
    let content = match kind {
        NodeKind::Statement => match (
            raw_node.statement_body,
            raw_node.qa_question,
            raw_node.qa_answer,
            alternative_answers.is_empty(),
        ) {
            (Some(body), None, None, true) => NodeContent::Statement { body },
            _ => {
                return Err((
                    id,
                    GraphProblem::InvalidMarkdown {
                        path: path.to_path_buf(),
                        reason: "statement node content fields are inconsistent".to_string(),
                    },
                ));
            }
        },
        NodeKind::QuestionAnswer => match (
            raw_node.statement_body,
            raw_node.qa_question,
            raw_node.qa_answer,
        ) {
            (None, Some(question), Some(answer)) if !question.trim().is_empty() => {
                NodeContent::QuestionAnswer {
                    question,
                    answer,
                    alternative_answers,
                }
            }
            (None, Some(_), Some(_)) => {
                return Err((
                    id,
                    GraphProblem::InvalidMarkdown {
                        path: path.to_path_buf(),
                        reason: "question must not be empty".to_string(),
                    },
                ));
            }
            _ => {
                return Err((
                    id,
                    GraphProblem::InvalidMarkdown {
                        path: path.to_path_buf(),
                        reason: "question-answer node content fields are inconsistent".to_string(),
                    },
                ));
            }
        },
    };

    Ok(ScannedNode {
        id,
        kind,
        title: raw_node.title,
        reviewed,
        content,
        created_at_unix,
        updated_at_unix,
        canonical_placements: Vec::new(),
        alias_placements: Vec::new(),
    })
}

fn scanned_placement_from_raw(
    raw: &RawPlacement,
    raw_node_ids: &BTreeSet<String>,
) -> Result<ScannedPlacement, (String, GraphProblem)> {
    let path = PathBuf::from(&raw.logical_path);
    if let Err(error) = validate_node_id(&raw.node_id) {
        return Err((
            raw.node_id.clone(),
            GraphProblem::InvalidMarkdown {
                path,
                reason: error.to_string(),
            },
        ));
    }
    if !raw_node_ids.contains(&raw.node_id) {
        return Err((
            raw.node_id.clone(),
            GraphProblem::InvalidMarkdown {
                path,
                reason: format!("placement references missing node `{}`", raw.node_id),
            },
        ));
    }
    if let Some(parent_id) = raw.parent_id.as_deref() {
        if let Err(error) = validate_node_id(parent_id) {
            return Err((
                raw.node_id.clone(),
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: error.to_string(),
                },
            ));
        }
        if !raw_node_ids.contains(parent_id) {
            return Err((
                raw.node_id.clone(),
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: format!("placement references missing parent `{parent_id}`"),
                },
            ));
        }
    }
    if raw.slug.trim().is_empty() || raw.slug.chars().any(char::is_control) {
        return Err((
            raw.node_id.clone(),
            GraphProblem::InvalidMarkdown {
                path,
                reason: "placement slug must not be empty or contain control characters"
                    .to_string(),
            },
        ));
    }
    let is_canonical = match raw.is_canonical {
        0 => false,
        1 => true,
        other => {
            return Err((
                raw.node_id.clone(),
                GraphProblem::InvalidMarkdown {
                    path,
                    reason: format!("invalid canonical flag `{other}`"),
                },
            ));
        }
    };
    if let Err(reason) = validate_logical_path(&raw.logical_path, &raw.node_id) {
        return Err((
            raw.node_id.clone(),
            GraphProblem::InvalidMarkdown { path, reason },
        ));
    }

    Ok(ScannedPlacement {
        row_id: raw.row_id,
        node_id: raw.node_id.clone(),
        parent_id: raw.parent_id.clone(),
        logical_path: path,
        is_canonical,
    })
}

fn query_raw_nodes(connection: &Connection, graph_id: i64) -> Result<Vec<RawNode>, GraphError> {
    let mut statement = connection
        .prepare(
            "SELECT id, kind, title, reviewed, statement_body, qa_question, qa_answer, \
             created_at_unix, updated_at_unix \
             FROM focal_nodes WHERE graph_id = ?1 ORDER BY id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![graph_id], |row| {
            Ok(RawNode {
                id: row.get(0)?,
                kind: row.get(1)?,
                title: row.get(2)?,
                reviewed: row.get(3)?,
                statement_body: row.get(4)?,
                qa_question: row.get(5)?,
                qa_answer: row.get(6)?,
                created_at_unix: row.get(7)?,
                updated_at_unix: row.get(8)?,
            })
        })
        .map_err(storage_error)?;
    let mut nodes = Vec::new();
    for row in rows {
        nodes.push(row.map_err(storage_error)?);
    }
    Ok(nodes)
}

fn query_raw_alternative_answers(
    connection: &Connection,
    graph_id: i64,
) -> Result<Vec<RawAlternativeAnswer>, GraphError> {
    let mut statement = connection
        .prepare(
            "SELECT node_id, answer_order, answer \
             FROM focal_qa_alternative_answers \
             WHERE graph_id = ?1 \
             ORDER BY node_id, answer_order",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![graph_id], |row| {
            Ok(RawAlternativeAnswer {
                node_id: row.get(0)?,
                answer_order: row.get(1)?,
                answer: row.get(2)?,
            })
        })
        .map_err(storage_error)?;
    let mut answers = Vec::new();
    for row in rows {
        answers.push(row.map_err(storage_error)?);
    }
    Ok(answers)
}

fn query_raw_contexts(
    connection: &Connection,
    graph_id: i64,
) -> Result<Vec<RawContext>, GraphError> {
    let mut statement = connection
        .prepare(
            "SELECT id, slug, filename, title, markdown, created_at_unix, updated_at_unix \
             FROM focal_context_documents WHERE graph_id = ?1 ORDER BY title, id, filename",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![graph_id], |row| {
            Ok(RawContext {
                id: row.get(0)?,
                slug: row.get(1)?,
                filename: row.get(2)?,
                title: row.get(3)?,
                markdown: row.get(4)?,
                created_at_unix: row.get(5)?,
                updated_at_unix: row.get(6)?,
            })
        })
        .map_err(storage_error)?;
    let mut contexts = Vec::new();
    for row in rows {
        contexts.push(row.map_err(storage_error)?);
    }
    Ok(contexts)
}

fn query_raw_placements(
    connection: &Connection,
    graph_id: i64,
) -> Result<Vec<RawPlacement>, GraphError> {
    let mut statement = connection
        .prepare(
            "SELECT id, node_id, parent_id, slug, logical_path, is_canonical \
             FROM focal_placements WHERE graph_id = ?1 ORDER BY logical_path, id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![graph_id], |row| {
            Ok(RawPlacement {
                row_id: row.get(0)?,
                node_id: row.get(1)?,
                parent_id: row.get(2)?,
                slug: row.get(3)?,
                logical_path: row.get(4)?,
                is_canonical: row.get(5)?,
            })
        })
        .map_err(storage_error)?;
    let mut placements = Vec::new();
    for row in rows {
        placements.push(row.map_err(storage_error)?);
    }
    Ok(placements)
}

fn validate_child_paths(result: &mut ScanResult) {
    let canonical_paths = result
        .nodes
        .iter()
        .filter_map(|(id, node)| canonical_path(node).map(|path| (id.clone(), path.to_path_buf())))
        .collect::<BTreeMap<_, _>>();

    let placements = result.placements.clone();
    for placement in placements {
        if let Some(parent_id) = placement.parent_id.as_deref() {
            let Some(parent_path) = canonical_paths.get(parent_id) else {
                continue;
            };
            let expected_prefix = PathBuf::from(parent_path).join("children");
            let actual_parent = placement.logical_path.parent().map(Path::to_path_buf);
            if actual_parent.as_ref() != Some(&expected_prefix) {
                add_node_problem(
                    result,
                    &placement.node_id,
                    GraphProblem::InvalidMarkdown {
                        path: placement.logical_path.clone(),
                        reason: "placement path does not match parent canonical path".to_string(),
                    },
                );
            }
        } else if placement.logical_path.parent() != Some(Path::new("roots")) {
            add_node_problem(
                result,
                &placement.node_id,
                GraphProblem::InvalidMarkdown {
                    path: placement.logical_path.clone(),
                    reason: "root placement path must be directly under roots".to_string(),
                },
            );
        }
    }
}

fn add_node_problem(result: &mut ScanResult, node_id: &str, problem: GraphProblem) {
    result
        .node_problems
        .entry(node_id.to_string())
        .or_insert_with(|| problem.clone());
    result.problems.push(problem);
}

fn add_context_problem(result: &mut ContextScan, context_id: &str, problem: GraphProblem) {
    result
        .context_problems
        .entry(context_id.to_string())
        .or_insert_with(|| problem.clone());
    result.problems.push(problem);
}

fn graph_id_for_graph(graph: &IdeaGraph<'_>) -> Result<i64, GraphError> {
    validate_graph_name(&graph.graph_name)?;
    require_graph_id(graph.connection(), &graph.graph_name)
}

fn require_graph_id(connection: &Connection, graph_name: &str) -> Result<i64, GraphError> {
    validate_graph_name(graph_name)?;
    connection
        .query_row(
            "SELECT id FROM focal_graphs WHERE name = ?1",
            params![graph_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| {
            GraphError::InvalidGraphRoot(format!("SQLite graph `{graph_name}` not found"))
        })
}

fn enable_foreign_keys(connection: &Connection) -> Result<(), GraphError> {
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(storage_error)
}

fn ensure_schema_exists(connection: &Connection) -> Result<(), GraphError> {
    for table in [
        "focal_graphs",
        "focal_nodes",
        "focal_context_documents",
        "focal_qa_alternative_answers",
        "focal_placements",
    ] {
        let exists = connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !exists {
            return Err(GraphError::InvalidGraphRoot(format!(
                "missing SQLite schema table `{table}`"
            )));
        }
    }
    ensure_table_columns(
        connection,
        "focal_graphs",
        &["id", "name", "created_at_unix"],
    )?;
    ensure_table_columns(
        connection,
        "focal_nodes",
        &[
            "graph_id",
            "id",
            "kind",
            "title",
            "reviewed",
            "statement_body",
            "qa_question",
            "qa_answer",
            "created_at_unix",
            "updated_at_unix",
        ],
    )?;
    ensure_table_columns(
        connection,
        "focal_qa_alternative_answers",
        &["graph_id", "node_id", "answer_order", "answer"],
    )?;
    ensure_table_columns(
        connection,
        "focal_context_documents",
        &[
            "graph_id",
            "id",
            "slug",
            "filename",
            "title",
            "markdown",
            "created_at_unix",
            "updated_at_unix",
        ],
    )?;
    ensure_table_columns(
        connection,
        "focal_placements",
        &[
            "id",
            "graph_id",
            "node_id",
            "parent_id",
            "slug",
            "logical_path",
            "is_canonical",
        ],
    )?;
    ensure_table_primary_key(connection, "focal_graphs", &["id"])?;
    ensure_table_primary_key(connection, "focal_nodes", &["graph_id", "id"])?;
    ensure_table_primary_key(
        connection,
        "focal_qa_alternative_answers",
        &["graph_id", "node_id", "answer_order"],
    )?;
    ensure_table_primary_key(connection, "focal_context_documents", &["graph_id", "id"])?;
    ensure_table_primary_key(connection, "focal_placements", &["id"])?;
    ensure_unique_key(connection, "focal_graphs", &["name"])?;
    ensure_unique_key(
        connection,
        "focal_context_documents",
        &["graph_id", "filename"],
    )?;
    Ok(())
}

fn ensure_table_columns(
    connection: &Connection,
    table: &str,
    required_columns: &[&str],
) -> Result<(), GraphError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?;
    let mut columns = BTreeSet::new();
    for row in rows {
        columns.insert(row.map_err(storage_error)?);
    }
    for column in required_columns {
        if !columns.contains(*column) {
            return Err(GraphError::InvalidGraphRoot(format!(
                "missing SQLite schema column `{table}.{column}`"
            )));
        }
    }
    Ok(())
}

fn ensure_table_primary_key(
    connection: &Connection,
    table: &str,
    required_columns: &[&str],
) -> Result<(), GraphError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
        })
        .map_err(storage_error)?;
    let mut key_columns = Vec::new();
    for row in rows {
        let (column, position) = row.map_err(storage_error)?;
        if position > 0 {
            key_columns.push((position, column));
        }
    }
    key_columns.sort_by_key(|(position, _)| *position);
    let actual = key_columns
        .into_iter()
        .map(|(_, column)| column)
        .collect::<Vec<_>>();
    if actual != required_columns {
        return Err(invalid_schema(format!(
            "`{table}` primary key must be ({})",
            required_columns.join(", ")
        )));
    }
    Ok(())
}

fn ensure_unique_key(
    connection: &Connection,
    table: &str,
    required_columns: &[&str],
) -> Result<(), GraphError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA index_list({table})"))
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })
        .map_err(storage_error)?;
    for row in rows {
        let (index_name, unique) = row.map_err(storage_error)?;
        if unique == 0 {
            continue;
        }
        if index_columns(connection, &index_name)? == required_columns {
            return Ok(());
        }
    }
    Err(invalid_schema(format!(
        "`{table}` must have a unique key on ({})",
        required_columns.join(", ")
    )))
}

fn index_columns(connection: &Connection, index_name: &str) -> Result<Vec<String>, GraphError> {
    let mut statement = connection
        .prepare(&format!(
            "PRAGMA index_info({})",
            quote_identifier(index_name)
        ))
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(2)?))
        })
        .map_err(storage_error)?;
    let mut columns = Vec::new();
    for row in rows {
        columns.push(row.map_err(storage_error)?);
    }
    columns.sort_by_key(|(position, _)| *position);
    Ok(columns
        .into_iter()
        .map(|(_, column)| column)
        .collect::<Vec<_>>())
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn invalid_schema(message: String) -> GraphError {
    GraphError::InvalidGraphRoot(format!("invalid SQLite schema: {message}"))
}

fn insert_node(
    connection: &Connection,
    graph_id: i64,
    id: &str,
    node: &NewNode,
    created_at_unix: u64,
    updated_at_unix: u64,
) -> Result<(), GraphError> {
    let (kind, statement_body, qa_question, qa_answer) = node_fields(node);
    connection
        .execute(
            "INSERT INTO focal_nodes \
             (graph_id, id, kind, title, reviewed, statement_body, qa_question, qa_answer, \
              created_at_unix, updated_at_unix) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                graph_id,
                id,
                kind,
                node.title,
                0_i64,
                statement_body,
                qa_question,
                qa_answer,
                unix_to_i64(created_at_unix)?,
                unix_to_i64(updated_at_unix)?,
            ],
        )
        .map_err(storage_error)?;
    insert_alternative_answers(connection, graph_id, id, &node.content)?;
    Ok(())
}

fn update_node_row(
    connection: &Connection,
    graph_id: i64,
    node_id: &str,
    title: &str,
    reviewed: bool,
    content: &NodeContent,
    updated_at_unix: u64,
) -> Result<(), GraphError> {
    let (statement_body, qa_question, qa_answer) = content_fields(content);
    let rows = connection
        .execute(
            "UPDATE focal_nodes \
             SET title = ?1, reviewed = ?2, statement_body = ?3, qa_question = ?4, qa_answer = ?5, \
                 updated_at_unix = ?6 \
             WHERE graph_id = ?7 AND id = ?8",
            params![
                title,
                if reviewed { 1_i64 } else { 0_i64 },
                statement_body,
                qa_question,
                qa_answer,
                unix_to_i64(updated_at_unix)?,
                graph_id,
                node_id,
            ],
        )
        .map_err(storage_error)?;
    if rows == 0 {
        return Err(GraphError::NodeNotFound(node_id.to_string()));
    }
    replace_alternative_answers(connection, graph_id, node_id, content)?;
    Ok(())
}

fn insert_context_document(
    connection: &Connection,
    graph_id: i64,
    id: &str,
    slug: &str,
    filename: &str,
    context: &NewContextDocument,
    created_at_unix: u64,
    updated_at_unix: u64,
) -> Result<(), GraphError> {
    connection
        .execute(
            "INSERT INTO focal_context_documents \
             (graph_id, id, slug, filename, title, markdown, created_at_unix, updated_at_unix) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                graph_id,
                id,
                slug,
                filename,
                context.title,
                context.markdown,
                unix_to_i64(created_at_unix)?,
                unix_to_i64(updated_at_unix)?,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn update_context_row(
    connection: &Connection,
    graph_id: i64,
    context_id: &str,
    title: &str,
    markdown: &str,
    updated_at_unix: u64,
) -> Result<(), GraphError> {
    let rows = connection
        .execute(
            "UPDATE focal_context_documents \
             SET title = ?1, markdown = ?2, updated_at_unix = ?3 \
             WHERE graph_id = ?4 AND id = ?5",
            params![
                title,
                markdown,
                unix_to_i64(updated_at_unix)?,
                graph_id,
                context_id,
            ],
        )
        .map_err(storage_error)?;
    if rows == 0 {
        return Err(GraphError::ContextNotFound(context_id.to_string()));
    }
    Ok(())
}

fn insert_placement(
    connection: &Connection,
    graph_id: i64,
    node_id: &str,
    parent_id: Option<&str>,
    slug: &str,
    logical_path: &str,
    is_canonical: bool,
) -> Result<(), GraphError> {
    connection
        .execute(
            "INSERT INTO focal_placements \
             (graph_id, node_id, parent_id, slug, logical_path, is_canonical) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                graph_id,
                node_id,
                parent_id,
                slug,
                logical_path,
                canonical_i64(is_canonical),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn replace_alternative_answers(
    connection: &Connection,
    graph_id: i64,
    node_id: &str,
    content: &NodeContent,
) -> Result<(), GraphError> {
    connection
        .execute(
            "DELETE FROM focal_qa_alternative_answers WHERE graph_id = ?1 AND node_id = ?2",
            params![graph_id, node_id],
        )
        .map_err(storage_error)?;
    insert_alternative_answers(connection, graph_id, node_id, content)
}

fn insert_alternative_answers(
    connection: &Connection,
    graph_id: i64,
    node_id: &str,
    content: &NodeContent,
) -> Result<(), GraphError> {
    let NodeContent::QuestionAnswer {
        alternative_answers,
        ..
    } = content
    else {
        return Ok(());
    };

    for (index, answer) in alternative_answers.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO focal_qa_alternative_answers \
                 (graph_id, node_id, answer_order, answer) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![graph_id, node_id, usize_to_i64(index)?, answer],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn delete_placement(connection: &Connection, graph_id: i64, row_id: i64) -> Result<(), GraphError> {
    connection
        .execute(
            "DELETE FROM focal_placements WHERE graph_id = ?1 AND id = ?2",
            params![graph_id, row_id],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn node_fields(node: &NewNode) -> (&'static str, Option<&str>, Option<&str>, Option<&str>) {
    match &node.content {
        NodeContent::Statement { body } => ("statement", Some(body.as_str()), None, None),
        NodeContent::QuestionAnswer {
            question, answer, ..
        } => ("qa", None, Some(question.as_str()), Some(answer.as_str())),
    }
}

fn content_fields(content: &NodeContent) -> (Option<&str>, Option<&str>, Option<&str>) {
    match content {
        NodeContent::Statement { body } => (Some(body.as_str()), None, None),
        NodeContent::QuestionAnswer {
            question, answer, ..
        } => (None, Some(question.as_str()), Some(answer.as_str())),
    }
}

fn generate_unique_node_id(connection: &Connection, graph_id: i64) -> Result<String, GraphError> {
    loop {
        let id = Uuid::new_v4().to_string();
        let exists = connection
            .query_row(
                "SELECT 1 FROM focal_nodes WHERE graph_id = ?1 AND id = ?2",
                params![graph_id, id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !exists {
            return Ok(id);
        }
    }
}

fn generate_unique_context_id(
    connection: &Connection,
    graph_id: i64,
) -> Result<String, GraphError> {
    loop {
        let id = Uuid::new_v4().to_string();
        let exists = connection
            .query_row(
                "SELECT 1 FROM focal_context_documents WHERE graph_id = ?1 AND id = ?2",
                params![graph_id, id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !exists {
            return Ok(id);
        }
    }
}

fn unique_context_filename(
    connection: &Connection,
    graph_id: i64,
    title: &str,
    context_id: &str,
) -> Result<(String, String), GraphError> {
    let base_slug = slugify(title);
    let mut suffix = 1usize;
    loop {
        let slug = if suffix == 1 {
            base_slug.clone()
        } else {
            format!("{base_slug}-{suffix}")
        };
        let filename = format!("{slug}--{context_id}.md");
        let exists = connection
            .query_row(
                "SELECT 1 FROM focal_context_documents WHERE graph_id = ?1 AND filename = ?2",
                params![graph_id, filename],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !exists {
            return Ok((slug, filename));
        }
        suffix = suffix
            .checked_add(1)
            .ok_or_else(|| GraphError::AliasConflict(PathBuf::from("context")))?;
    }
}

fn unique_root_path(
    connection: &Connection,
    graph_id: i64,
    title: &str,
    node_id: &str,
) -> Result<(String, String), GraphError> {
    unique_path(connection, graph_id, "roots", title, node_id)
}

fn unique_child_path(
    connection: &Connection,
    graph_id: i64,
    parent_path: &str,
    title: &str,
    node_id: &str,
) -> Result<(String, String), GraphError> {
    unique_path(
        connection,
        graph_id,
        &format!("{parent_path}/children"),
        title,
        node_id,
    )
}

fn unique_path(
    connection: &Connection,
    graph_id: i64,
    container: &str,
    title: &str,
    node_id: &str,
) -> Result<(String, String), GraphError> {
    let base_slug = slugify(title);
    let mut suffix = 1usize;
    loop {
        let slug = if suffix == 1 {
            base_slug.clone()
        } else {
            format!("{base_slug}-{suffix}")
        };
        let path = format!("{container}/{slug}--{node_id}");
        let exists = connection
            .query_row(
                "SELECT 1 FROM focal_placements WHERE graph_id = ?1 AND logical_path = ?2",
                params![graph_id, path],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if !exists {
            return Ok((slug, path));
        }
        suffix = suffix
            .checked_add(1)
            .ok_or_else(|| GraphError::AliasConflict(PathBuf::from(container)))?;
    }
}

fn move_root_node_under_parent(
    connection: &Connection,
    graph_id: i64,
    node: &ScannedNode,
    parent_id: &str,
    parent_path: &str,
) -> Result<(), GraphError> {
    let canonical =
        canonical_placement(node).ok_or_else(|| GraphError::NodeNotFound(node.id.clone()))?;
    let old_path = canonical.logical_path.to_string_lossy().into_owned();
    let (slug, new_path) =
        unique_child_path(connection, graph_id, parent_path, &node.title, &node.id)?;
    connection
        .execute(
            "UPDATE focal_placements SET parent_id = ?1, slug = ?2 \
             WHERE graph_id = ?3 AND id = ?4",
            params![parent_id, slug, graph_id, canonical.row_id],
        )
        .map_err(storage_error)?;
    rewrite_subtree_paths(connection, graph_id, &old_path, &new_path, None)
}

fn move_node_to_roots(
    connection: &Connection,
    graph_id: i64,
    node: &ScannedNode,
) -> Result<(), GraphError> {
    let canonical =
        canonical_placement(node).ok_or_else(|| GraphError::NodeNotFound(node.id.clone()))?;
    if canonical.parent_id.is_none() {
        return Ok(());
    }
    let old_path = canonical.logical_path.to_string_lossy().into_owned();
    let (slug, new_path) = unique_root_path(connection, graph_id, &node.title, &node.id)?;
    connection
        .execute(
            "UPDATE focal_placements SET parent_id = NULL, slug = ?1 \
             WHERE graph_id = ?2 AND id = ?3",
            params![slug, graph_id, canonical.row_id],
        )
        .map_err(storage_error)?;
    rewrite_subtree_paths(connection, graph_id, &old_path, &new_path, None)
}

fn promote_to_alias(
    connection: &Connection,
    graph_id: i64,
    node: &ScannedNode,
    alias: &ScannedPlacement,
) -> Result<(), GraphError> {
    let canonical =
        canonical_placement(node).ok_or_else(|| GraphError::NodeNotFound(node.id.clone()))?;
    let old_path = canonical.logical_path.to_string_lossy().into_owned();
    let new_path = alias.logical_path.to_string_lossy().into_owned();
    delete_placement(connection, graph_id, canonical.row_id)?;
    connection
        .execute(
            "UPDATE focal_placements SET is_canonical = 1 WHERE graph_id = ?1 AND id = ?2",
            params![graph_id, alias.row_id],
        )
        .map_err(storage_error)?;
    rewrite_subtree_paths(
        connection,
        graph_id,
        &old_path,
        &new_path,
        Some(canonical.row_id),
    )
}

fn rewrite_subtree_paths(
    connection: &Connection,
    graph_id: i64,
    old_path: &str,
    new_path: &str,
    skipped_row_id: Option<i64>,
) -> Result<(), GraphError> {
    let pattern = format!("{old_path}/%");
    let mut statement = connection
        .prepare(
            "SELECT id, logical_path FROM focal_placements \
             WHERE graph_id = ?1 AND (logical_path = ?2 OR logical_path LIKE ?3) \
             ORDER BY length(logical_path), logical_path",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![graph_id, old_path, pattern], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_error)?;
    let mut rewrites = Vec::new();
    for row in rows {
        let (row_id, path) = row.map_err(storage_error)?;
        if skipped_row_id == Some(row_id) {
            continue;
        }
        rewrites.push((row_id, path_after_subtree_move(&path, old_path, new_path)));
    }
    drop(statement);

    for (row_id, path) in rewrites {
        connection
            .execute(
                "UPDATE focal_placements SET logical_path = ?1 \
                 WHERE graph_id = ?2 AND id = ?3",
                params![path, graph_id, row_id],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn perform_delete_set(
    connection: &Connection,
    graph_id: i64,
    scan: &ScanResult,
    delete_set: &BTreeSet<String>,
) -> Result<(), GraphError> {
    let delete_paths = delete_set
        .iter()
        .filter_map(|id| {
            scan.nodes
                .get(id)
                .and_then(canonical_path)
                .map(Path::to_path_buf)
        })
        .collect::<Vec<_>>();

    for (id, alias) in promotion_targets_for_delete(scan, delete_set, &delete_paths) {
        let node = canonical_node(scan, &id)?;
        promote_to_alias(connection, graph_id, node, &alias)?;
    }

    for id in delete_set {
        connection
            .execute(
                "DELETE FROM focal_placements \
                 WHERE graph_id = ?1 AND (node_id = ?2 OR parent_id = ?2)",
                params![graph_id, id],
            )
            .map_err(storage_error)?;
    }
    for id in delete_set {
        connection
            .execute(
                "DELETE FROM focal_nodes WHERE graph_id = ?1 AND id = ?2",
                params![graph_id, id],
            )
            .map_err(storage_error)?;
    }

    Ok(())
}

fn promotion_targets_for_delete(
    scan: &ScanResult,
    delete_set: &BTreeSet<String>,
    delete_paths: &[PathBuf],
) -> Vec<(String, ScannedPlacement)> {
    let mut candidates = BTreeMap::<String, ScannedPlacement>::new();

    for placement in &scan.placements {
        let Some(parent_id) = placement.parent_id.as_deref() else {
            continue;
        };
        if !delete_set.contains(parent_id) || delete_set.contains(&placement.node_id) {
            continue;
        }
        let Some(node) = scan.nodes.get(&placement.node_id) else {
            continue;
        };
        let Some(canonical_path) = canonical_path(node) else {
            continue;
        };
        if !is_path_inside_any(canonical_path, delete_paths) {
            continue;
        }
        let Some(alias) = node
            .alias_placements
            .iter()
            .filter(|candidate| !is_path_inside_any(&candidate.logical_path, delete_paths))
            .min_by_key(|candidate| path_sort_key(&candidate.logical_path))
            .cloned()
        else {
            continue;
        };
        candidates.entry(placement.node_id.clone()).or_insert(alias);
    }

    let mut targets = candidates.into_iter().collect::<Vec<_>>();
    targets.sort_by_key(|(id, _)| {
        scan.nodes
            .get(id)
            .and_then(canonical_path)
            .map(|path| path.components().count())
            .unwrap_or(usize::MAX)
    });

    let mut selected = Vec::<(String, ScannedPlacement)>::new();
    let mut selected_paths = Vec::<PathBuf>::new();
    for (id, alias) in targets {
        let Some(canonical_path) = scan.nodes.get(&id).and_then(canonical_path) else {
            continue;
        };
        if selected_paths
            .iter()
            .any(|selected_path| canonical_path.starts_with(selected_path))
        {
            continue;
        }
        selected_paths.push(canonical_path.to_path_buf());
        selected.push((id, alias));
    }

    selected
}

fn compute_delete_set(scan: &ScanResult, target_id: &str) -> BTreeSet<String> {
    let mut delete_set = BTreeSet::from([target_id.to_string()]);
    let mut changed = true;

    while changed {
        changed = false;
        for placement in &scan.placements {
            let Some(parent_id) = placement.parent_id.as_deref() else {
                continue;
            };
            if !delete_set.contains(parent_id) || delete_set.contains(&placement.node_id) {
                continue;
            }
            let has_root_entry = scan.placements.iter().any(|candidate| {
                candidate.node_id == placement.node_id && candidate.parent_id.is_none()
            });
            if has_root_entry {
                continue;
            }
            let parents = parent_edges(scan, &placement.node_id);
            if parents.iter().all(|parent| {
                parent
                    .parent_id
                    .as_deref()
                    .is_some_and(|candidate| delete_set.contains(candidate))
            }) {
                delete_set.insert(placement.node_id.clone());
                changed = true;
            }
        }
    }

    delete_set
}

fn require_context<'a>(
    scan: &'a ContextScan,
    context_id: &str,
) -> Result<&'a ScannedContext, GraphError> {
    validate_context_id(context_id)?;
    if let Some(problem) = scan.context_problems.get(context_id) {
        return Err(graph_problem_to_error(problem));
    }
    scan.contexts
        .get(context_id)
        .ok_or_else(|| GraphError::ContextNotFound(context_id.to_string()))
}

fn context_document_from_scanned(context: &ScannedContext) -> ContextDocument {
    ContextDocument {
        id: context.id.clone(),
        title: context.title.clone(),
        filename: context.filename.clone(),
        markdown: context.markdown.clone(),
        created_at_unix: context.created_at_unix,
        updated_at_unix: context.updated_at_unix,
        path: context.path.clone(),
    }
}

fn context_summary_from_scanned(context: &ScannedContext) -> ContextSummary {
    ContextSummary {
        id: context.id.clone(),
        title: context.title.clone(),
        filename: context.filename.clone(),
        path: context.path.clone(),
    }
}

fn first_context_problem_error(scan: &ContextScan) -> Option<GraphError> {
    scan.problems.iter().find_map(|problem| match problem {
        GraphProblem::DuplicateContextDocument { .. }
        | GraphProblem::InvalidContextMarkdown { .. } => Some(graph_problem_to_error(problem)),
        _ => None,
    })
}

fn canonical_node<'a>(scan: &'a ScanResult, id: &str) -> Result<&'a ScannedNode, GraphError> {
    validate_node_id(id)?;
    let Some(node) = scan.nodes.get(id) else {
        if let Some(problem) = scan.node_problems.get(id) {
            return Err(graph_problem_to_error(problem));
        }
        return Err(GraphError::NodeNotFound(id.to_string()));
    };
    if node.canonical_placements.len() > 1 {
        return Err(GraphError::DuplicateCanonicalNode {
            id: id.to_string(),
            paths: node
                .canonical_placements
                .iter()
                .map(|placement| placement.logical_path.clone())
                .collect(),
        });
    }
    if let Some(problem) = scan.node_problems.get(id) {
        return Err(graph_problem_to_error(problem));
    }
    if node.canonical_placements.is_empty() {
        return Err(GraphError::InvalidMarkdown {
            path: PathBuf::from(format!("nodes/{id}")),
            reason: "node has no canonical placement".to_string(),
        });
    }
    Ok(node)
}

fn require_parent_node<'a>(
    scan: &'a ScanResult,
    parent_id: &str,
) -> Result<&'a ScannedNode, GraphError> {
    match canonical_node(scan, parent_id) {
        Ok(node) => Ok(node),
        Err(GraphError::NodeNotFound(_)) => Err(GraphError::ParentNotFound(parent_id.to_string())),
        Err(error) => Err(error),
    }
}

fn require_child_node<'a>(
    scan: &'a ScanResult,
    child_id: &str,
) -> Result<&'a ScannedNode, GraphError> {
    match canonical_node(scan, child_id) {
        Ok(node) => Ok(node),
        Err(GraphError::NodeNotFound(_)) => Err(GraphError::ChildNotFound(child_id.to_string())),
        Err(error) => Err(error),
    }
}

fn canonical_placement(node: &ScannedNode) -> Option<&ScannedPlacement> {
    node.canonical_placements.first()
}

fn canonical_path(node: &ScannedNode) -> Option<&Path> {
    canonical_placement(node).map(|placement| placement.logical_path.as_path())
}

fn node_summary(node: &ScannedNode, is_alias: bool) -> Option<NodeSummary> {
    Some(NodeSummary {
        id: node.id.clone(),
        kind: node.kind.clone(),
        title: node.title.clone(),
        reviewed: node.reviewed,
        canonical_path: canonical_path(node)?.to_path_buf(),
        is_alias,
    })
}

fn summary_for(scan: &ScanResult, id: &str, is_alias: bool) -> Option<NodeSummary> {
    node_summary(scan.nodes.get(id)?, is_alias)
}

fn node_from_scanned(node: &ScannedNode) -> Result<Node, GraphError> {
    Ok(Node {
        id: node.id.clone(),
        kind: node.kind.clone(),
        title: node.title.clone(),
        reviewed: node.reviewed,
        content: node.content.clone(),
        created_at_unix: node.created_at_unix,
        updated_at_unix: node.updated_at_unix,
        canonical_path: canonical_path(node)
            .ok_or_else(|| GraphError::NodeNotFound(node.id.clone()))?
            .to_path_buf(),
        alias_paths: node.alias_paths(),
    })
}

fn child_edges<'a>(scan: &'a ScanResult, parent_id: &str) -> Vec<&'a ScannedPlacement> {
    scan.placements
        .iter()
        .filter(|placement| placement.parent_id.as_deref() == Some(parent_id))
        .collect()
}

fn parent_edges<'a>(scan: &'a ScanResult, child_id: &str) -> Vec<&'a ScannedPlacement> {
    scan.placements
        .iter()
        .filter(|placement| placement.node_id == child_id && placement.parent_id.is_some())
        .collect()
}

fn find_placement<'a>(
    scan: &'a ScanResult,
    parent_id: &str,
    child_id: &str,
) -> Option<&'a ScannedPlacement> {
    scan.placements.iter().find(|placement| {
        placement.parent_id.as_deref() == Some(parent_id) && placement.node_id == child_id
    })
}

fn reaches(scan: &ScanResult, start_id: &str, target_id: &str) -> bool {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([start_id.to_string()]);

    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        for placement in child_edges(scan, &id) {
            if placement.node_id == target_id {
                return true;
            }
            queue.push_back(placement.node_id.clone());
        }
    }

    false
}

#[derive(Debug, Clone, Copy)]
enum TraversalDirection {
    Ancestors,
    Descendants,
}

fn traverse(
    graph: &IdeaGraph<'_>,
    node_id: &str,
    options: TraversalOptions,
    direction: TraversalDirection,
) -> Result<Vec<NodeSummary>, GraphError> {
    validate_node_id(node_id)?;
    let graph_id = graph_id_for_graph(graph)?;
    let scan = load_scan(graph.connection(), graph_id)?;
    canonical_node(&scan, node_id)?;

    if options.max_depth == Some(0) {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let mut visited = BTreeSet::from([node_id.to_string()]);
    let mut queue = VecDeque::new();
    for neighbor in sorted_neighbors(&scan, node_id, direction)? {
        queue.push_back((neighbor.id.clone(), neighbor, 1usize));
    }

    while let Some((id, summary, depth)) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        results.push(summary);

        if options
            .max_depth
            .is_some_and(|max_depth| depth >= max_depth)
        {
            continue;
        }
        for neighbor in sorted_neighbors(&scan, &id, direction)? {
            if !visited.contains(&neighbor.id) {
                queue.push_back((neighbor.id.clone(), neighbor, depth + 1));
            }
        }
    }

    Ok(results)
}

fn sorted_neighbors(
    scan: &ScanResult,
    node_id: &str,
    direction: TraversalDirection,
) -> Result<Vec<NodeSummary>, GraphError> {
    let mut summaries = match direction {
        TraversalDirection::Ancestors => {
            if let Some(error) = entry_problem_error_for_node_id(scan, node_id) {
                return Err(error);
            }
            parent_edges(scan, node_id)
                .into_iter()
                .filter_map(|placement| {
                    placement
                        .parent_id
                        .as_deref()
                        .and_then(|parent_id| summary_for(scan, parent_id, false))
                })
                .collect::<Vec<_>>()
        }
        TraversalDirection::Descendants => {
            canonical_node(scan, node_id)?;
            let mut summaries = Vec::new();
            for placement in child_edges(scan, node_id) {
                let node = canonical_node(scan, &placement.node_id)?;
                if let Some(summary) = node_summary(node, !placement.is_canonical) {
                    summaries.push(summary);
                }
            }
            summaries
        }
    };
    sort_summaries(&mut summaries);
    Ok(summaries)
}

fn graph_index(scan: &ScanResult) -> GraphIndex {
    let mut contexts = scan
        .contexts
        .values()
        .map(context_summary_from_scanned)
        .collect::<Vec<_>>();
    sort_context_summaries(&mut contexts);

    let mut nodes = scan
        .nodes
        .values()
        .filter_map(|node| node_summary(node, false))
        .collect::<Vec<_>>();
    sort_summaries(&mut nodes);

    let mut edges = scan
        .placements
        .iter()
        .filter_map(|placement| {
            Some(GraphEdge {
                parent_id: placement.parent_id.clone()?,
                child_id: placement.node_id.clone(),
                path: placement.logical_path.clone(),
                is_symlink: !placement.is_canonical,
            })
        })
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        left.parent_id
            .cmp(&right.parent_id)
            .then(left.child_id.cmp(&right.child_id))
            .then(left.path.cmp(&right.path))
    });

    GraphIndex {
        contexts,
        nodes,
        edges,
        problems: scan.problems.clone(),
    }
}

fn sort_context_summaries(summaries: &mut [ContextSummary]) {
    summaries.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then(left.id.cmp(&right.id))
            .then(left.path.cmp(&right.path))
    });
}

fn sort_summaries(summaries: &mut [NodeSummary]) {
    summaries.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then(left.id.cmp(&right.id))
            .then(left.canonical_path.cmp(&right.canonical_path))
    });
}

fn detect_cycles(result: &mut ScanResult) {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for placement in &result.placements {
        if let Some(parent_id) = placement.parent_id.as_deref() {
            adjacency
                .entry(parent_id)
                .or_default()
                .push(placement.node_id.as_str());
        }
    }
    for children in adjacency.values_mut() {
        children.sort_unstable();
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut cycles = BTreeSet::new();
    let ids = result.nodes.keys().map(String::as_str).collect::<Vec<_>>();
    for id in ids {
        detect_cycle_from(id, &adjacency, &mut visiting, &mut visited, &mut cycles);
    }

    result.problems.extend(
        cycles
            .into_iter()
            .map(|node_id| GraphProblem::CycleDetected { node_id }),
    );
}

fn detect_cycle_from<'a>(
    id: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
    cycles: &mut BTreeSet<String>,
) {
    if visited.contains(id) {
        return;
    }
    if !visiting.insert(id) {
        cycles.insert(id.to_string());
        return;
    }

    if let Some(children) = adjacency.get(id) {
        for child in children {
            detect_cycle_from(child, adjacency, visiting, visited, cycles);
        }
    }

    visiting.remove(id);
    visited.insert(id);
}

fn entry_problem_error_for_node_id(scan: &ScanResult, id: &str) -> Option<GraphError> {
    scan.node_problems
        .get(id)
        .map(graph_problem_to_error)
        .or_else(|| {
            scan.problems
                .iter()
                .find(|problem| graph_problem_node_id(problem).as_deref() == Some(id))
                .map(graph_problem_to_error)
        })
}

fn graph_problem_node_id(problem: &GraphProblem) -> Option<String> {
    match problem {
        GraphProblem::BrokenSymlink { path }
        | GraphProblem::MissingNodeMarkdown { path }
        | GraphProblem::MissingChildrenDirectory { path }
        | GraphProblem::InvalidMarkdown { path, .. } => problem_path_node_id(path),
        GraphProblem::DuplicateCanonicalNode { id, .. }
        | GraphProblem::CycleDetected { node_id: id } => Some(id.clone()),
        GraphProblem::DuplicateContextDocument { .. }
        | GraphProblem::InvalidContextMarkdown { .. } => None,
    }
}

fn problem_path_node_id(path: &Path) -> Option<String> {
    let value = path.to_string_lossy();
    if let Some(id) = value.strip_prefix("nodes/") {
        return Some(id.to_string());
    }
    node_id_from_logical_path(&value)
}

fn graph_problem_to_error(problem: &GraphProblem) -> GraphError {
    match problem {
        GraphProblem::BrokenSymlink { path } => GraphError::BrokenSymlink(path.clone()),
        GraphProblem::DuplicateContextDocument { id, paths } => {
            GraphError::DuplicateContextDocument {
                id: id.clone(),
                paths: paths.clone(),
            }
        }
        GraphProblem::DuplicateCanonicalNode { id, paths } => GraphError::DuplicateCanonicalNode {
            id: id.clone(),
            paths: paths.clone(),
        },
        GraphProblem::InvalidContextMarkdown { path, reason } => {
            GraphError::InvalidContextMarkdown {
                path: path.clone(),
                reason: reason.clone(),
            }
        }
        GraphProblem::MissingNodeMarkdown { path } => GraphError::MissingNodeMarkdown(path.clone()),
        GraphProblem::MissingChildrenDirectory { path } => {
            GraphError::MissingChildrenDirectory(path.clone())
        }
        GraphProblem::InvalidMarkdown { path, reason } => GraphError::InvalidMarkdown {
            path: path.clone(),
            reason: reason.clone(),
        },
        GraphProblem::CycleDetected { .. } => GraphError::CycleDetected,
    }
}

fn validate_graph_name(graph_name: &str) -> Result<(), GraphError> {
    if graph_name.trim().is_empty() || graph_name.chars().any(char::is_control) {
        return Err(GraphError::InvalidGraphRoot(
            "graph name must not be empty or contain control characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_new_node(node: &NewNode, path: &Path) -> Result<(), GraphError> {
    validate_title(&node.title)?;
    validate_content_matches(&node.kind, &node.content, path)?;
    validate_content_values(&node.content, path)
}

fn validate_new_context_document(context: &NewContextDocument) -> Result<(), GraphError> {
    validate_title(&context.title)
}

fn validate_content_matches(
    kind: &NodeKind,
    content: &NodeContent,
    path: &Path,
) -> Result<(), GraphError> {
    match (kind, content) {
        (NodeKind::Statement, NodeContent::Statement { .. })
        | (NodeKind::QuestionAnswer, NodeContent::QuestionAnswer { .. }) => Ok(()),
        _ => Err(GraphError::InvalidMarkdown {
            path: path.to_path_buf(),
            reason: "node kind does not match node content".to_string(),
        }),
    }
}

fn validate_content_values(content: &NodeContent, path: &Path) -> Result<(), GraphError> {
    match content {
        NodeContent::Statement { .. } => Ok(()),
        NodeContent::QuestionAnswer { question, .. } if question.trim().is_empty() => {
            Err(GraphError::InvalidMarkdown {
                path: path.to_path_buf(),
                reason: "question must not be empty".to_string(),
            })
        }
        NodeContent::QuestionAnswer {
            alternative_answers,
            ..
        } if alternative_answers
            .iter()
            .any(|answer| answer.trim().is_empty()) =>
        {
            Err(GraphError::InvalidMarkdown {
                path: path.to_path_buf(),
                reason: "alternative answer must not be empty".to_string(),
            })
        }
        NodeContent::QuestionAnswer { .. } => Ok(()),
    }
}

fn validate_node_id(id: &str) -> Result<(), GraphError> {
    if !is_valid_uuid_id(id) {
        return Err(GraphError::InvalidNodeId(id.to_string()));
    }

    Ok(())
}

fn validate_context_id(id: &str) -> Result<(), GraphError> {
    if !is_valid_uuid_id(id) {
        return Err(GraphError::InvalidContextId(id.to_string()));
    }

    Ok(())
}

fn is_valid_uuid_id(id: &str) -> bool {
    let valid_shape = id.len() == 36
        && id.char_indices().all(|(index, ch)| match index {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_digit() || ('a'..='f').contains(&ch),
        });

    valid_shape && Uuid::parse_str(id).is_ok()
}

fn validate_title(title: &str) -> Result<(), GraphError> {
    if title.trim().is_empty() || title.chars().any(char::is_control) {
        return Err(GraphError::InvalidTitle);
    }
    Ok(())
}

fn validate_context_filename(filename: &str, context_id: &str) -> Result<(), String> {
    if filename.trim().is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.chars().any(char::is_control)
    {
        return Err("context filename must be a single safe path component".to_string());
    }
    if filename == "." || filename == ".." {
        return Err("context filename must be a single safe path component".to_string());
    }
    let Some(stem) = filename.strip_suffix(".md") else {
        return Err("context filename must end with `.md`".to_string());
    };
    let Some((_, suffix)) = stem.rsplit_once("--") else {
        return Err("context filename must end with `--<context-id>.md`".to_string());
    };
    if validate_context_id(suffix).is_err() {
        return Err("context filename must end with `--<context-id>.md`".to_string());
    }
    if suffix != context_id {
        return Err("context filename id suffix does not match context id".to_string());
    }
    Ok(())
}

fn validate_logical_path(path: &str, node_id: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("logical path must not be empty".to_string());
    }
    if path.starts_with('/') || path.contains('\\') || path.chars().any(char::is_control) {
        return Err("logical path must be a relative slash-separated path".to_string());
    }
    let mut saw_component = false;
    for component in path.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err("logical path contains an invalid component".to_string());
        }
        saw_component = true;
    }
    if !saw_component {
        return Err("logical path must contain at least one component".to_string());
    }
    match node_id_from_logical_path(path) {
        Some(suffix) if suffix == node_id => Ok(()),
        _ => Err("logical path node id suffix does not match placement node id".to_string()),
    }
}

fn non_negative_unix(value: i64, path: &Path, field: &str) -> Result<u64, GraphProblem> {
    if value < 0 {
        return Err(GraphProblem::InvalidMarkdown {
            path: path.to_path_buf(),
            reason: format!("{field} must not be negative"),
        });
    }
    Ok(value as u64)
}

fn non_negative_context_unix(value: i64, path: &Path, field: &str) -> Result<u64, GraphProblem> {
    if value < 0 {
        return Err(GraphProblem::InvalidContextMarkdown {
            path: path.to_path_buf(),
            reason: format!("{field} must not be negative"),
        });
    }
    Ok(value as u64)
}

fn unix_to_i64(value: u64) -> Result<i64, GraphError> {
    i64::try_from(value).map_err(|_| {
        GraphError::Storage("unix timestamp does not fit in SQLite INTEGER".to_string())
    })
}

fn usize_to_i64(value: usize) -> Result<i64, GraphError> {
    i64::try_from(value)
        .map_err(|_| GraphError::Storage("answer order does not fit in SQLite INTEGER".to_string()))
}

fn now_unix() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_dash = false;

    for ch in title.trim().chars() {
        if slug.len() >= MAX_SLUG_BYTES {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_dash = false;
        } else if !previous_was_dash {
            slug.push('-');
            previous_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "node".to_string()
    } else {
        slug.to_string()
    }
}

fn node_id_from_logical_path(path: &str) -> Option<String> {
    let name = path.rsplit('/').next()?;
    let (_, id) = name.rsplit_once("--")?;
    if validate_node_id(id).is_ok() {
        Some(id.to_string())
    } else {
        None
    }
}

fn logical_context_path(filename: &str) -> PathBuf {
    PathBuf::from("context").join(filename)
}

fn path_after_subtree_move(path: &str, old_subtree_path: &str, new_subtree_path: &str) -> String {
    if path == old_subtree_path {
        return new_subtree_path.to_string();
    }
    let prefix = format!("{old_subtree_path}/");
    match path.strip_prefix(&prefix) {
        Some(relative) => format!("{new_subtree_path}/{relative}"),
        None => path.to_string(),
    }
}

fn is_path_inside_any(path: &Path, candidates: &[PathBuf]) -> bool {
    candidates
        .iter()
        .any(|candidate| path == candidate || path.starts_with(candidate))
}

fn path_sort_key(path: &Path) -> String {
    path.as_os_str().to_string_lossy().into_owned()
}

fn diagnostic_path_for_node(placements: &[RawPlacement], node_id: &str) -> PathBuf {
    placements
        .iter()
        .filter(|placement| placement.node_id == node_id)
        .map(|placement| PathBuf::from(&placement.logical_path))
        .min()
        .unwrap_or_else(|| PathBuf::from(format!("nodes/{node_id}")))
}

fn canonical_i64(is_canonical: bool) -> i64 {
    if is_canonical { 1 } else { 0 }
}

fn storage_error(error: rusqlite::Error) -> GraphError {
    GraphError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_generation_matches_filesystem_style() {
        assert_eq!(
            slugify(" Rust keeps local tools simple "),
            "rust-keeps-local-tools-simple"
        );
        assert_eq!(slugify("!!!"), "node");
    }

    #[test]
    fn logical_path_validation_checks_id_suffix() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validate_logical_path(&format!("roots/title--{id}"), id).is_ok());
        assert_eq!(
            validate_logical_path("roots/title--7d9f2e5c-0f22-4c18-a0be-9f23e772a0bc", id)
                .unwrap_err(),
            "logical path node id suffix does not match placement node id"
        );
    }
}
