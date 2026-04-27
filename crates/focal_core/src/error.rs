use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid graph root: {0}")]
    InvalidGraphRoot(String),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("parent not found: {0}")]
    ParentNotFound(String),
    #[error("child not found: {0}")]
    ChildNotFound(String),
    #[error("duplicate node id: {0}")]
    DuplicateNodeId(String),
    #[error("invalid node id: {0}")]
    InvalidNodeId(String),
    #[error("invalid title")]
    InvalidTitle,
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
}
