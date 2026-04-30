use std::path::PathBuf;

pub use focal_types::{
    ContextDocument, ContextDocumentPatch, ContextId, ContextSummary, DeleteMode, GraphEdge,
    GraphIndex, GraphProblem, NewContextDocument, NewNode, Node, NodeContent, NodeId, NodeKind,
    NodePatch, NodeSummary, OrphanPolicy, TraversalOptions,
};

#[derive(Debug, Clone)]
pub struct IdeaGraph {
    pub(crate) root: PathBuf,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn spec_09_model_types_are_plain_constructible_values() {
        let id = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let content = NodeContent::Statement {
            body: "Body".to_string(),
        };
        let node = Node {
            id: id.clone(),
            kind: NodeKind::Statement,
            title: "Title".to_string(),
            content: content.clone(),
            created_at_unix: 1,
            updated_at_unix: 2,
            canonical_path: PathBuf::from("roots/title--id"),
            alias_paths: vec![PathBuf::from("roots/alias--id")],
        };
        let new_node = NewNode {
            kind: NodeKind::Statement,
            title: node.title.clone(),
            content: content.clone(),
        };
        let patch = NodePatch {
            title: Some("New title".to_string()),
            content: Some(content.clone()),
        };
        let summary = NodeSummary {
            id,
            kind: NodeKind::Statement,
            title: node.title.clone(),
            canonical_path: node.canonical_path.clone(),
            is_alias: false,
        };

        assert_eq!(node.content, content);
        assert_eq!(new_node.kind, NodeKind::Statement);
        assert_eq!(patch.title.as_deref(), Some("New title"));
        assert_eq!(summary.title, "Title");
    }

    #[test]
    fn spec_11_delete_and_orphan_modes_are_copyable_contract_values() {
        let delete_mode = DeleteMode::Recursive;
        let copied_delete_mode = delete_mode;
        let orphan_policy = OrphanPolicy::FailIfWouldOrphan;
        let copied_orphan_policy = orphan_policy;

        assert_eq!(delete_mode, copied_delete_mode);
        assert_eq!(orphan_policy, copied_orphan_policy);
        assert_ne!(DeleteMode::FailIfHasChildren, DeleteMode::Recursive);
        assert_ne!(OrphanPolicy::MoveToRoots, OrphanPolicy::DeleteIfNoParents);
    }

    #[test]
    fn spec_14_traversal_options_default_has_no_depth_limit() {
        assert_eq!(
            TraversalOptions::default(),
            TraversalOptions { max_depth: None }
        );
    }

    #[test]
    fn spec_19_index_edge_and_problem_types_are_constructible() {
        let edge = GraphEdge {
            parent_id: "parent".to_string(),
            child_id: "child".to_string(),
            path: PathBuf::from("roots/parent/children/child"),
            is_symlink: true,
        };
        let problem = GraphProblem::BrokenSymlink {
            path: PathBuf::from("roots/parent/children/missing"),
        };
        let index = GraphIndex {
            contexts: Vec::new(),
            nodes: Vec::new(),
            edges: vec![edge.clone()],
            problems: vec![problem.clone()],
        };

        assert_eq!(index.edges, vec![edge]);
        assert_eq!(index.problems, vec![problem]);
    }
}
