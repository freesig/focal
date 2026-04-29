use std::fmt;

pub use focal_types::GraphError;

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

    #[test]
    fn backend_error_wrapper_preserves_graph_error() {
        let wrapped = Error::from(GraphError::NodeNotFound("missing".to_string()));

        assert!(matches!(
            wrapped.as_graph_error(),
            GraphError::NodeNotFound(id) if id == "missing"
        ));
        assert!(wrapped.to_string().contains("missing"));
    }
}
