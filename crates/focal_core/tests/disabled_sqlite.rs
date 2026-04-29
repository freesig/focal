#![cfg(not(feature = "sqlite"))]

use focal_core::{
    self as core, DeleteMode, Error, GraphIndex, NewNode, Node, NodeContent, NodeId, NodeKind,
    NodePatch, NodeSummary, OrphanPolicy, TraversalOptions,
};
use std::fmt::Debug;

const PARENT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const CHILD_ID: &str = "7d9f2e5c-0f22-4c18-a0be-9f23e772a0bc";

fn statement() -> NewNode {
    NewNode {
        kind: NodeKind::Statement,
        title: "Title".to_string(),
        content: NodeContent::Statement {
            body: String::new(),
        },
    }
}

fn assert_disabled<T: Debug>(result: Result<T, Error>) {
    match result {
        Err(Error::Sqlite(error)) => assert_eq!(error.graph_name, "main"),
        other => panic!("expected disabled sqlite error, got {other:?}"),
    }
}

#[test]
fn disabled_sqlite_backend_returns_disabled_error_for_every_operation() {
    let mut backend = core::disabled_sqlite("main");

    assert_disabled::<NodeId>(core::add_root_node(&mut backend, statement()));
    assert_disabled::<NodeId>(core::add_child_node(&mut backend, PARENT_ID, statement()));
    assert_disabled::<Node>(core::read_node(&backend, CHILD_ID));
    assert_disabled::<Node>(core::update_node(
        &mut backend,
        CHILD_ID,
        NodePatch::default(),
    ));
    assert_disabled::<()>(core::delete_node(
        &mut backend,
        CHILD_ID,
        DeleteMode::FailIfHasChildren,
    ));
    assert_disabled::<()>(core::link_existing_node(&mut backend, PARENT_ID, CHILD_ID));
    assert_disabled::<()>(core::unlink_child(
        &mut backend,
        PARENT_ID,
        CHILD_ID,
        OrphanPolicy::FailIfWouldOrphan,
    ));
    assert_disabled::<Vec<NodeSummary>>(core::list_roots(&backend));
    assert_disabled::<Vec<NodeSummary>>(core::list_children(&backend, PARENT_ID));
    assert_disabled::<Vec<NodeSummary>>(core::list_parents(&backend, CHILD_ID));
    assert_disabled::<Vec<NodeSummary>>(core::list_ancestors(
        &backend,
        CHILD_ID,
        TraversalOptions::default(),
    ));
    assert_disabled::<Vec<NodeSummary>>(core::list_descendants(
        &backend,
        PARENT_ID,
        TraversalOptions::default(),
    ));
    assert_disabled::<GraphIndex>(core::rebuild_index(&backend));
}
