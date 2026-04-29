use focal_core::{
    self as core, DeleteMode, Error, GraphError, NewNode, NodeContent, NodeKind, NodePatch,
    OrphanPolicy, TraversalOptions,
};
use tempfile::tempdir;

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
fn fs_backend_dispatches_shared_operations() {
    let temp = tempdir().unwrap();
    let mut backend = core::init_fs(temp.path()).unwrap();

    let root = core::add_root_node(&mut backend, statement("Root", "root")).unwrap();
    let child = core::add_child_node(&mut backend, &root, statement("Child", "child")).unwrap();
    let other_parent = core::add_root_node(&mut backend, statement("Other Parent", "")).unwrap();

    core::link_existing_node(&mut backend, &other_parent, &child).unwrap();
    core::link_existing_node(&mut backend, &other_parent, &child).unwrap();

    let parents = core::list_parents(&backend, &child).unwrap();
    assert_eq!(parents.len(), 2);
    assert_eq!(core::list_children(&backend, &root).unwrap()[0].id, child);

    let descendants =
        core::list_descendants(&backend, &root, TraversalOptions { max_depth: Some(1) }).unwrap();
    assert_eq!(
        descendants.iter().map(|node| &node.id).collect::<Vec<_>>(),
        [&child]
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
        },
    )
    .unwrap();
    assert_eq!(updated.id, before.id);
    assert_eq!(updated.canonical_path, before.canonical_path);
    assert_eq!(updated.title, "Updated Child");

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
        Err(Error::Fs(error)) if matches!(error.as_graph_error(), GraphError::NodeNotFound(_))
    ));
}

#[test]
fn fs_backend_errors_are_typed_core_errors() {
    let temp = tempdir().unwrap();
    let backend = core::init_fs(temp.path()).unwrap();
    let missing = "550e8400-e29b-41d4-a716-446655440000";

    match core::read_node(&backend, missing) {
        Err(Error::Fs(error)) => {
            assert!(matches!(
                error.as_graph_error(),
                GraphError::NodeNotFound(id) if id == missing
            ));
        }
        other => panic!("expected typed filesystem error, got {other:?}"),
    }
}

#[test]
fn fs_backend_error_wrapper_converts_with_from() {
    let backend_error = focal_fs::Error::from(GraphError::NodeNotFound("missing".to_string()));
    let core_error = Error::from(backend_error);

    assert!(matches!(
        core_error,
        Error::Fs(error)
            if matches!(error.as_graph_error(), GraphError::NodeNotFound(id) if id == "missing")
    ));
}
