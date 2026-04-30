//! Filesystem-backed idea graph library.
//!
//! This crate stores idea nodes as Markdown files in directories and represents
//! shared parent-child relationships with directory symlinks. It is intended
//! for single-writer local use on Unix-like platforms.

mod error;
mod fs_utils;
mod markdown;
mod model;
mod ops;
mod scan;

pub use error::{Error, GraphError};
pub use model::{
    ContextDocument, ContextDocumentPatch, ContextId, ContextSummary, DeleteMode, GraphEdge,
    GraphIndex, GraphProblem, IdeaGraph, NewContextDocument, NewNode, Node, NodeContent, NodeId,
    NodeKind, NodePatch, NodeSummary, OrphanPolicy, TraversalOptions,
};
pub use ops::{
    add_child_node, add_context_document, add_root_node, delete_context_document, delete_node,
    init_graph, link_existing_node, list_ancestors, list_children, list_context_documents,
    list_descendants, list_parents, list_roots, open_graph, read_context_document, read_node,
    rebuild_index, unlink_child, update_context_document, update_node,
};

#[cfg(test)]
mod tests {
    #[test]
    fn spec_01_02_03_and_22_crate_surface_stays_dependency_light_library_only() {
        let manifest = include_str!("../Cargo.toml");
        let docs = include_str!("lib.rs");

        assert!(!manifest.contains("[[bin]]"));
        assert!(!manifest.contains("tokio"));
        assert!(!manifest.contains("axum"));
        assert!(!manifest.contains("actix"));
        assert!(!manifest.contains("sqlx"));
        assert!(!manifest.contains("rusqlite"));
        assert!(!manifest.contains("clap"));
        assert!(docs.contains("single-writer local use"));
        assert!(docs.contains("Filesystem-backed idea graph library"));
    }

    #[test]
    fn spec_26_spec_tracks_unit_and_integration_test_traceability() {
        let spec = include_str!("../../../spec/SPEC.md");

        assert!(spec.contains("| Section | Unit tests | Integration tests |"));
        for section in 1..=28 {
            assert!(
                spec.contains(&format!("| {section} |")),
                "missing traceability row for spec section {section}"
            );
        }
    }
}
