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

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::io;

    use super::*;

    #[test]
    fn spec_20_graph_error_display_and_source_preserve_io_error() {
        let error = GraphError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));

        assert!(error.to_string().contains("denied"));
        assert!(error.source().is_some());
    }

    #[test]
    fn spec_20_path_errors_include_repair_context() {
        let path = PathBuf::from("/tmp/ideas/roots/broken/node.md");
        let error = GraphError::InvalidMarkdown {
            path: path.clone(),
            reason: "missing metadata block".to_string(),
        };

        assert!(error.to_string().contains(path.to_string_lossy().as_ref()));
        assert!(error.to_string().contains("missing metadata block"));
    }
}
