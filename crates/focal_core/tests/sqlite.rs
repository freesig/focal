#![cfg(feature = "sqlite")]

use focal_core::{
    self as core, ContextDocumentPatch, DeleteMode, Error, GraphError, NewContextDocument, NewNode,
    NodeContent, NodeKind, NodePatch, OrphanPolicy, TraversalOptions,
};
use rusqlite::Connection;

fn statement(title: &str, body: &str) -> NewNode {
    NewNode {
        kind: NodeKind::Statement,
        title: title.to_string(),
        content: NodeContent::Statement {
            body: body.to_string(),
        },
    }
}

#[test]
fn sqlite_backend_dispatches_shared_operations() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut backend = core::init_sqlite(&mut connection, "main").unwrap();

    let root = core::add_root_node(&mut backend, statement("Root", "root")).unwrap();
    let child = core::add_child_node(&mut backend, &root, statement("Child", "child")).unwrap();
    let other_parent = core::add_root_node(&mut backend, statement("Other Parent", "")).unwrap();

    core::link_existing_node(&mut backend, &other_parent, &child).unwrap();
    core::link_existing_node(&mut backend, &other_parent, &child).unwrap();

    assert_eq!(core::list_roots(&backend).unwrap().len(), 2);
    assert_eq!(core::list_parents(&backend, &child).unwrap().len(), 2);
    assert_eq!(core::list_children(&backend, &root).unwrap()[0].id, child);
    assert_eq!(
        core::list_descendants(&backend, &root, TraversalOptions { max_depth: Some(1) },)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        core::list_ancestors(&backend, &child, TraversalOptions::default())
            .unwrap()
            .len(),
        2
    );

    let before = core::read_node(&backend, &child).unwrap();
    let updated = core::update_node(
        &mut backend,
        &child,
        NodePatch {
            title: Some("Updated Child".to_string()),
            content: Some(NodeContent::Statement {
                body: "updated".to_string(),
            }),
            reviewed: None,
        },
    )
    .unwrap();
    assert_eq!(updated.id, before.id);
    assert_eq!(updated.canonical_path, before.canonical_path);

    let index = core::rebuild_index(&backend).unwrap();
    assert!(
        index
            .edges
            .iter()
            .any(|edge| edge.parent_id == other_parent && edge.child_id == child)
    );

    core::unlink_child(
        &mut backend,
        &other_parent,
        &child,
        OrphanPolicy::MoveToRoots,
    )
    .unwrap();
    assert_eq!(core::list_parents(&backend, &child).unwrap().len(), 1);
    core::delete_node(&mut backend, &root, DeleteMode::Recursive).unwrap();
    assert!(matches!(
        core::read_node(&backend, &root),
        Err(Error::Sqlite(error)) if matches!(error.as_graph_error(), GraphError::NodeNotFound(_))
    ));
}

#[test]
fn sqlite_backend_dispatches_context_operations() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut backend = core::init_sqlite(&mut connection, "main").unwrap();

    let id = core::add_context_document(
        &mut backend,
        NewContextDocument {
            title: "Raw notes".to_string(),
            markdown: "Body".to_string(),
        },
    )
    .unwrap();
    let before = core::read_context_document(&backend, &id).unwrap();
    let updated = core::update_context_document(
        &mut backend,
        &id,
        ContextDocumentPatch {
            title: Some("Renamed".to_string()),
            markdown: Some(String::new()),
        },
    )
    .unwrap();

    assert_eq!(updated.id, id);
    assert_eq!(updated.filename, before.filename);
    assert_eq!(core::list_context_documents(&backend).unwrap().len(), 1);
    assert_eq!(core::rebuild_index(&backend).unwrap().contexts.len(), 1);

    core::delete_context_document(&mut backend, &id).unwrap();
    assert!(matches!(
        core::read_context_document(&backend, &id),
        Err(Error::Sqlite(error))
            if matches!(error.as_graph_error(), GraphError::ContextNotFound(missing) if missing == &id)
    ));
}

#[test]
fn sqlite_backend_errors_are_typed_core_errors() {
    let mut connection = Connection::open_in_memory().unwrap();
    let backend = core::init_sqlite(&mut connection, "main").unwrap();
    let missing = "550e8400-e29b-41d4-a716-446655440000";

    match core::read_node(&backend, missing) {
        Err(Error::Sqlite(error)) => {
            assert!(matches!(
                error.as_graph_error(),
                GraphError::NodeNotFound(id) if id == missing
            ));
        }
        other => panic!("expected typed sqlite error, got {other:?}"),
    }
}

#[test]
fn sqlite_backend_error_wrapper_converts_with_from() {
    let backend_error = focal_sqlite::Error::from(GraphError::NodeNotFound("missing".to_string()));
    let core_error = Error::from(backend_error);

    assert!(matches!(
        core_error,
        Error::Sqlite(error)
            if matches!(error.as_graph_error(), GraphError::NodeNotFound(id) if id == "missing")
    ));
}
