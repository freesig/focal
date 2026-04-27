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

#[derive(Debug, Clone)]
pub struct IdeaGraph {
    pub(crate) root: PathBuf,
}
