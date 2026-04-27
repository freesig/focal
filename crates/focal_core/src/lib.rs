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

pub use error::GraphError;
pub use model::{
    DeleteMode, GraphEdge, GraphIndex, GraphProblem, IdeaGraph, NewNode, Node, NodeContent, NodeId,
    NodeKind, NodePatch, NodeSummary, OrphanPolicy, TraversalOptions,
};
pub use ops::{
    add_child_node, add_root_node, delete_node, init_graph, link_existing_node, list_ancestors,
    list_children, list_descendants, list_parents, list_roots, open_graph, read_node,
    rebuild_index, unlink_child, update_node,
};
