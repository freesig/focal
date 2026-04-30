//! Shared public types for Focal storage backends.

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TraversalOptions {
    pub max_depth: Option<usize>,
}

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
    InvalidContextMarkdown { path: PathBuf, reason: String },
    MissingNodeMarkdown { path: PathBuf },
    MissingChildrenDirectory { path: PathBuf },
    InvalidMarkdown { path: PathBuf, reason: String },
    CycleDetected { node_id: NodeId },
}

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("invalid graph root: {0}")]
    InvalidGraphRoot(String),
    #[error("context document not found: {0}")]
    ContextNotFound(String),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("parent not found: {0}")]
    ParentNotFound(String),
    #[error("child not found: {0}")]
    ChildNotFound(String),
    #[error("duplicate node id: {0}")]
    DuplicateNodeId(String),
    #[error("duplicate context id: {0}")]
    DuplicateContextId(String),
    #[error("invalid node id: {0}")]
    InvalidNodeId(String),
    #[error("invalid context id: {0}")]
    InvalidContextId(String),
    #[error("invalid title")]
    InvalidTitle,
    #[error("invalid context markdown at {path}: {reason}")]
    InvalidContextMarkdown { path: PathBuf, reason: String },
    #[error("invalid markdown at {path}: {reason}")]
    InvalidMarkdown { path: PathBuf, reason: String },
    #[error("missing node.md at {0}")]
    MissingNodeMarkdown(PathBuf),
    #[error("missing children directory at {0}")]
    MissingChildrenDirectory(PathBuf),
    #[error("broken symlink at {0}")]
    BrokenSymlink(PathBuf),
    #[error("symlink unsupported: {0}")]
    SymlinkUnsupported(String),
    #[error("cycle detected")]
    CycleDetected,
    #[error("node has children: {0}")]
    NodeHasChildren(String),
    #[error("operation would orphan node: {0}")]
    WouldOrphanNode(String),
    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("alias conflict: {0}")]
    AliasConflict(PathBuf),
    #[error("duplicate canonical node {id}: {paths:?}")]
    DuplicateCanonicalNode { id: String, paths: Vec<PathBuf> },
    #[error("duplicate context document {id}: {paths:?}")]
    DuplicateContextDocument { id: String, paths: Vec<PathBuf> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_models_are_plain_constructible_values() {
        let id: ContextId = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let document = ContextDocument {
            id: id.clone(),
            title: "Raw notes".to_string(),
            filename: format!("raw-notes--{id}.md"),
            markdown: "Body".to_string(),
            created_at_unix: 1,
            updated_at_unix: 2,
            path: PathBuf::from(format!("context/raw-notes--{id}.md")),
        };
        let summary = ContextSummary {
            id: id.clone(),
            title: document.title.clone(),
            filename: document.filename.clone(),
            path: document.path.clone(),
        };

        assert_eq!(document.id, id);
        assert_eq!(summary.filename, document.filename);
        assert_eq!(
            NewContextDocument {
                title: "Raw notes".to_string(),
                markdown: String::new(),
            }
            .markdown,
            ""
        );
        assert_eq!(ContextDocumentPatch::default().title, None);
    }

    #[test]
    fn graph_index_and_errors_include_context_variants() {
        let id = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let path = PathBuf::from(format!("context/raw--{id}.md"));
        let index = GraphIndex {
            contexts: vec![ContextSummary {
                id: id.clone(),
                title: "Raw".to_string(),
                filename: format!("raw--{id}.md"),
                path: path.clone(),
            }],
            nodes: Vec::new(),
            edges: Vec::new(),
            problems: vec![
                GraphProblem::DuplicateContextDocument {
                    id: id.clone(),
                    paths: vec![path.clone()],
                },
                GraphProblem::InvalidContextMarkdown {
                    path: path.clone(),
                    reason: "missing metadata".to_string(),
                },
            ],
        };

        assert_eq!(index.contexts[0].id, id);
        assert!(matches!(
            GraphError::ContextNotFound("missing".to_string()),
            GraphError::ContextNotFound(_)
        ));
        assert!(matches!(
            GraphError::InvalidContextId("bad".to_string()),
            GraphError::InvalidContextId(_)
        ));
        assert!(matches!(
            GraphError::DuplicateContextDocument {
                id: "dup".to_string(),
                paths: Vec::new()
            },
            GraphError::DuplicateContextDocument { .. }
        ));
        assert!(matches!(
            GraphError::InvalidContextMarkdown {
                path: PathBuf::from("context/bad.md"),
                reason: "bad".to_string()
            },
            GraphError::InvalidContextMarkdown { .. }
        ));
        assert!(matches!(
            GraphError::DuplicateContextId("dup".to_string()),
            GraphError::DuplicateContextId(_)
        ));
    }
}
