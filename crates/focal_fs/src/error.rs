pub use focal_types::GraphError;

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::io;
    use std::path::PathBuf;

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
