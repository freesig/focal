use std::fs;
use std::path::Path;

use focal_core::{
    DeleteMode, GraphError, GraphProblem, NewNode, NodeContent, NodeKind, NodePatch, OrphanPolicy,
    TraversalOptions, add_child_node, add_root_node, delete_node, init_graph, link_existing_node,
    list_ancestors, list_children, list_descendants, list_parents, list_roots, open_graph,
    read_node, rebuild_index, unlink_child, update_node,
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

fn qa(title: &str, question: &str, answer: &str) -> NewNode {
    NewNode {
        kind: NodeKind::QuestionAnswer,
        title: title.to_string(),
        content: NodeContent::QuestionAnswer {
            question: question.to_string(),
            answer: answer.to_string(),
        },
    }
}

fn assert_uuid_shape(id: &str) {
    assert_eq!(id.len(), 36);
    assert_eq!(&id[8..9], "-");
    assert_eq!(&id[13..14], "-");
    assert_eq!(&id[18..19], "-");
    assert_eq!(&id[23..24], "-");
    assert!(id.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-'));
    assert!(id.chars().all(|ch| !ch.is_ascii_uppercase()));
}

#[test]
fn init_open_add_read_and_update_nodes() {
    let temp = tempdir().unwrap();
    let graph_path = temp.path().join("ideas");
    let graph = init_graph(&graph_path).unwrap();

    assert!(graph_path.join("roots").is_dir());
    open_graph(&graph_path).unwrap();
    assert!(matches!(
        open_graph(temp.path().join("not-a-graph")),
        Err(GraphError::Io(_)) | Err(GraphError::InvalidGraphRoot(_))
    ));

    let id = add_root_node(&graph, statement("Rust keeps local tools simple", "Body")).unwrap();
    assert_uuid_shape(&id);

    let node = read_node(&graph, &id).unwrap();
    assert_eq!(node.id, id);
    assert_eq!(node.title, "Rust keeps local tools simple");
    assert!(
        node.canonical_path
            .ends_with(format!("rust-keeps-local-tools-simple--{}", node.id))
    );
    assert!(node.canonical_path.join("node.md").is_file());
    assert!(node.canonical_path.join("children").is_dir());
    assert_eq!(
        node.content,
        NodeContent::Statement {
            body: "Body".to_string()
        }
    );

    let original_path = node.canonical_path.clone();
    let updated = update_node(
        &graph,
        &id,
        NodePatch {
            title: Some("A better title".to_string()),
            content: Some(NodeContent::Statement {
                body: "Updated body".to_string(),
            }),
        },
    )
    .unwrap();
    assert_eq!(updated.id, id);
    assert_eq!(updated.title, "A better title");
    assert_eq!(updated.canonical_path, original_path);
    assert!(updated.updated_at_unix > node.updated_at_unix);
    assert_eq!(
        updated.content,
        NodeContent::Statement {
            body: "Updated body".to_string()
        }
    );

    let markdown = fs::read_to_string(updated.canonical_path.join("node.md")).unwrap();
    assert!(markdown.contains("title: A better title"));
    assert!(!markdown.contains("# A better title"));
}

#[test]
fn question_answer_nodes_and_traversal_are_deterministic() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let beta = add_child_node(&graph, &root, statement("Beta", "")).unwrap();
    let alpha = add_child_node(&graph, &root, qa("Alpha", "Why?", "Because.")).unwrap();
    let gamma = add_child_node(&graph, &alpha, statement("Gamma", "")).unwrap();

    let alpha_node = read_node(&graph, &alpha).unwrap();
    assert_eq!(
        alpha_node.content,
        NodeContent::QuestionAnswer {
            question: "Why?".to_string(),
            answer: "Because.".to_string()
        }
    );

    let children = list_children(&graph, &root).unwrap();
    assert_eq!(
        children
            .iter()
            .map(|node| node.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Alpha", "Beta"]
    );

    let descendants = list_descendants(&graph, &root, TraversalOptions { max_depth: None })
        .unwrap()
        .into_iter()
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(descendants, vec![alpha.clone(), beta.clone(), gamma]);

    let shallow = list_descendants(&graph, &root, TraversalOptions { max_depth: Some(1) })
        .unwrap()
        .into_iter()
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(shallow, vec![alpha, beta]);
}

#[test]
fn linking_is_idempotent_and_rejects_cycles() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let parent = add_root_node(&graph, statement("Parent", "")).unwrap();
    let other_parent = add_root_node(&graph, statement("Other Parent", "")).unwrap();
    let child = add_child_node(&graph, &parent, statement("Child", "")).unwrap();

    link_existing_node(&graph, &other_parent, &child).unwrap();
    link_existing_node(&graph, &other_parent, &child).unwrap();

    let parents = list_parents(&graph, &child).unwrap();
    assert_eq!(parents.len(), 2);

    let child_node = read_node(&graph, &child).unwrap();
    assert_eq!(child_node.alias_paths.len(), 1);
    assert!(
        fs::symlink_metadata(&child_node.alias_paths[0])
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_link(&child_node.alias_paths[0])
            .unwrap()
            .is_relative()
    );

    assert!(matches!(
        link_existing_node(&graph, &child, &parent),
        Err(GraphError::CycleDetected)
    ));
}

#[test]
fn unlinking_canonical_parent_promotes_alias_and_preserves_subtree() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let old_parent = add_root_node(&graph, statement("Old Parent", "")).unwrap();
    let new_parent = add_root_node(&graph, statement("New Parent", "")).unwrap();
    let child = add_child_node(&graph, &old_parent, statement("Child", "content")).unwrap();
    let grandchild = add_child_node(&graph, &child, statement("Grandchild", "")).unwrap();

    link_existing_node(&graph, &new_parent, &child).unwrap();
    let before = read_node(&graph, &child).unwrap();
    let alias_path = before.alias_paths[0].clone();

    unlink_child(&graph, &old_parent, &child, OrphanPolicy::MoveToRoots).unwrap();

    let after = read_node(&graph, &child).unwrap();
    assert_eq!(after.canonical_path, alias_path);
    assert!(after.alias_paths.is_empty());
    assert_eq!(
        list_children(&graph, &new_parent)
            .unwrap()
            .into_iter()
            .map(|node| (node.id, node.is_alias))
            .collect::<Vec<_>>(),
        vec![(child.clone(), false)]
    );
    assert_eq!(list_children(&graph, &old_parent).unwrap().len(), 0);
    assert_eq!(list_children(&graph, &child).unwrap()[0].id, grandchild);
}

#[test]
fn unlink_orphan_policies_move_fail_and_delete() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let parent = add_root_node(&graph, statement("Parent", "")).unwrap();
    let child = add_child_node(&graph, &parent, statement("Child", "")).unwrap();

    assert!(matches!(
        unlink_child(&graph, &parent, &child, OrphanPolicy::FailIfWouldOrphan),
        Err(GraphError::WouldOrphanNode(id)) if id == child
    ));
    assert_eq!(list_children(&graph, &parent).unwrap().len(), 1);

    unlink_child(&graph, &parent, &child, OrphanPolicy::MoveToRoots).unwrap();
    assert!(
        list_roots(&graph)
            .unwrap()
            .iter()
            .any(|node| node.id == child)
    );
    assert!(list_parents(&graph, &child).unwrap().is_empty());

    let second_parent = add_root_node(&graph, statement("Second Parent", "")).unwrap();
    let second_child =
        add_child_node(&graph, &second_parent, statement("Second Child", "")).unwrap();
    unlink_child(
        &graph,
        &second_parent,
        &second_child,
        OrphanPolicy::DeleteIfNoParents,
    )
    .unwrap();
    assert!(matches!(
        read_node(&graph, &second_child),
        Err(GraphError::NodeNotFound(_))
    ));
}

#[test]
fn recursive_delete_preserves_shared_descendants_by_promotion() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let deleting_root = add_root_node(&graph, statement("Deleting Root", "")).unwrap();
    let outside_root = add_root_node(&graph, statement("Outside Root", "")).unwrap();
    let shared = add_child_node(&graph, &deleting_root, statement("Shared", "")).unwrap();

    link_existing_node(&graph, &outside_root, &shared).unwrap();
    let alias_path = read_node(&graph, &shared).unwrap().alias_paths[0].clone();

    delete_node(&graph, &deleting_root, DeleteMode::Recursive).unwrap();

    assert!(matches!(
        read_node(&graph, &deleting_root),
        Err(GraphError::NodeNotFound(_))
    ));
    let shared_node = read_node(&graph, &shared).unwrap();
    assert_eq!(shared_node.canonical_path, alias_path);
    assert_eq!(list_parents(&graph, &shared).unwrap()[0].id, outside_root);
}

#[test]
fn delete_modes_handle_leaf_and_non_leaf_nodes() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let child = add_child_node(&graph, &root, statement("Child", "")).unwrap();

    assert!(matches!(
        delete_node(&graph, &root, DeleteMode::FailIfHasChildren),
        Err(GraphError::NodeHasChildren(id)) if id == root
    ));

    delete_node(&graph, &child, DeleteMode::FailIfHasChildren).unwrap();
    assert!(matches!(
        read_node(&graph, &child),
        Err(GraphError::NodeNotFound(_))
    ));
}

#[test]
fn validation_reports_manual_filesystem_problems() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let child = add_child_node(&graph, &root, statement("Child", "")).unwrap();
    let malformed = add_root_node(&graph, statement("Malformed", "")).unwrap();
    let root_node = read_node(&graph, &root).unwrap();
    let child_node = read_node(&graph, &child).unwrap();
    let malformed_node = read_node(&graph, &malformed).unwrap();

    copy_dir_all(
        &child_node.canonical_path,
        root_node
            .canonical_path
            .join("children")
            .join(format!("duplicate--{child}")),
    );
    fs::remove_dir_all(child_node.canonical_path.join("children")).unwrap();
    fs::write(
        malformed_node.canonical_path.join("node.md"),
        "not front matter",
    )
    .unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(
        temp.path().join("missing-target"),
        root_node
            .canonical_path
            .join("children")
            .join("broken--550e8400-e29b-41d4-a716-446655440000"),
    )
    .unwrap();

    let problems = rebuild_index(&graph).unwrap().problems;
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { path, .. } if path.ends_with("node.md")
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::MissingChildrenDirectory { path } if path == &child_node.canonical_path
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::DuplicateCanonicalNode { id, .. } if id == &child
    )));
    #[cfg(unix)]
    assert!(
        problems
            .iter()
            .any(|problem| matches!(problem, GraphProblem::BrokenSymlink { .. }))
    );
    #[cfg(unix)]
    assert!(matches!(
        list_descendants(&graph, &root, TraversalOptions::default()),
        Err(GraphError::BrokenSymlink(_))
    ));
}

#[test]
fn rejects_invalid_inputs_and_ids() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();

    assert!(matches!(
        add_root_node(&graph, statement("  ", "")),
        Err(GraphError::InvalidTitle)
    ));
    assert!(matches!(
        update_node(
            &graph,
            &root,
            NodePatch {
                title: Some("bad\ntitle".to_string()),
                content: None,
            },
        ),
        Err(GraphError::InvalidTitle)
    ));
    assert!(matches!(
        add_root_node(&graph, qa("Unanswered", "", "")),
        Err(GraphError::InvalidMarkdown { reason, .. }) if reason == "question must not be empty"
    ));
    assert!(matches!(
        add_root_node(
            &graph,
            NewNode {
                kind: NodeKind::Statement,
                title: "Mismatch".to_string(),
                content: NodeContent::QuestionAnswer {
                    question: "Why?".to_string(),
                    answer: String::new(),
                },
            },
        ),
        Err(GraphError::InvalidMarkdown { reason, .. })
            if reason == "node kind does not match node content"
    ));
    assert!(matches!(
        read_node(&graph, "../not-an-id"),
        Err(GraphError::InvalidNodeId(_))
    ));
}

#[test]
fn validation_rejects_bad_directory_suffix_and_duplicate_metadata() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let root_node = read_node(&graph, &root).unwrap();

    copy_dir_all(
        &root_node.canonical_path,
        temp.path().join("roots").join("missing-id-suffix"),
    );

    let duplicate_metadata_id = "550e8400-e29b-41d4-a716-446655440000";
    let malformed_dir = temp
        .path()
        .join("roots")
        .join(format!("duplicate-metadata--{duplicate_metadata_id}"));
    fs::create_dir_all(malformed_dir.join("children")).unwrap();
    fs::write(
        malformed_dir.join("node.md"),
        format!(
            "---\nid: {duplicate_metadata_id}\nkind: statement\nkind: statement\ntitle: Broken\ncreated_at_unix: 1\nupdated_at_unix: 1\n---\n\nBody"
        ),
    )
    .unwrap();

    let problems = rebuild_index(&graph).unwrap().problems;
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { reason, .. }
            if reason == "node directory id suffix does not match metadata id"
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { reason, .. }
            if reason == "duplicate metadata field `kind`"
    )));
}

#[test]
#[cfg(unix)]
fn symlink_targets_outside_graph_are_rejected() {
    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let root_node = read_node(&graph, &root).unwrap();
    let outside_link = root_node
        .canonical_path
        .join("children")
        .join("outside--550e8400-e29b-41d4-a716-446655440000");

    std::os::unix::fs::symlink(outside.path(), &outside_link).unwrap();

    let problems = rebuild_index(&graph).unwrap().problems;
    assert!(problems.iter().any(
        |problem| matches!(problem, GraphProblem::BrokenSymlink { path } if path == &outside_link)
    ));
    assert!(matches!(
        list_descendants(&graph, &root, TraversalOptions::default()),
        Err(GraphError::BrokenSymlink(path)) if path == outside_link
    ));
}

#[test]
#[cfg(unix)]
fn symlinked_roots_directory_is_rejected_before_scanning_or_writing() {
    let graph_temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let graph = init_graph(graph_temp.path()).unwrap();
    let outside_graph = init_graph(outside.path()).unwrap();
    let outside_id = add_root_node(&outside_graph, statement("Outside", "secret")).unwrap();
    let roots = graph_temp.path().join("roots");

    fs::remove_dir_all(&roots).unwrap();
    std::os::unix::fs::symlink(outside.path().join("roots"), &roots).unwrap();

    assert!(matches!(
        list_roots(&graph),
        Err(GraphError::InvalidGraphRoot(_))
    ));
    assert!(matches!(
        add_root_node(&graph, statement("Should Not Escape", "")),
        Err(GraphError::InvalidGraphRoot(_))
    ));
    assert_eq!(
        fs::read_dir(outside.path().join("roots")).unwrap().count(),
        1
    );
    assert_eq!(
        read_node(&outside_graph, &outside_id).unwrap().title,
        "Outside"
    );
}

#[test]
#[cfg(unix)]
fn symlinked_children_directory_is_not_scanned_or_written_through() {
    let graph_temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let graph = init_graph(graph_temp.path()).unwrap();
    let outside_graph = init_graph(outside.path()).unwrap();
    let outside_id = add_root_node(&outside_graph, statement("Outside", "secret")).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let root_node = read_node(&graph, &root).unwrap();
    let children = root_node.canonical_path.join("children");

    fs::remove_dir_all(&children).unwrap();
    std::os::unix::fs::symlink(outside.path().join("roots"), &children).unwrap();

    let index = rebuild_index(&graph).unwrap();
    assert!(
        index
            .problems
            .iter()
            .any(|problem| matches!(problem, GraphProblem::MissingChildrenDirectory { path } if path == &root_node.canonical_path))
    );
    assert!(!index.nodes.iter().any(|node| node.id == outside_id));
    assert!(list_children(&graph, &root).unwrap().is_empty());
    assert!(matches!(
        add_child_node(&graph, &root, statement("Should Not Escape", "")),
        Err(GraphError::MissingChildrenDirectory(path)) if path == root_node.canonical_path
    ));
    assert_eq!(
        fs::read_dir(outside.path().join("roots")).unwrap().count(),
        1
    );
}

#[test]
#[cfg(unix)]
fn symlinked_node_markdown_is_not_read_from_outside_graph() {
    let graph_temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let graph = init_graph(graph_temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "internal body")).unwrap();
    let root_node = read_node(&graph, &root).unwrap();
    let node_file = root_node.canonical_path.join("node.md");
    let outside_file = outside.path().join("outside.md");
    let outside_markdown = fs::read_to_string(&node_file)
        .unwrap()
        .replace("title: Root", "title: Outside")
        .replace("internal body", "outside body");

    fs::write(&outside_file, outside_markdown).unwrap();
    fs::remove_file(&node_file).unwrap();
    std::os::unix::fs::symlink(&outside_file, &node_file).unwrap();

    assert!(matches!(
        read_node(&graph, &root),
        Err(GraphError::BrokenSymlink(path)) if path == node_file
    ));
    assert!(rebuild_index(&graph).unwrap().problems.iter().any(
        |problem| matches!(problem, GraphProblem::BrokenSymlink { path } if path == &node_file)
    ));
}

#[test]
fn promotion_chooses_first_alias_and_rewrites_remaining_aliases() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let old_parent = add_root_node(&graph, statement("Old Parent", "")).unwrap();
    let alpha_parent = add_root_node(&graph, statement("Alpha Parent", "")).unwrap();
    let beta_parent = add_root_node(&graph, statement("Beta Parent", "")).unwrap();
    let child = add_child_node(&graph, &old_parent, statement("Child", "body")).unwrap();

    link_existing_node(&graph, &beta_parent, &child).unwrap();
    link_existing_node(&graph, &alpha_parent, &child).unwrap();
    let before = read_node(&graph, &child).unwrap();
    assert_eq!(before.alias_paths.len(), 2);
    let expected_canonical = before.alias_paths[0].clone();
    let remaining_alias = before.alias_paths[1].clone();

    unlink_child(&graph, &old_parent, &child, OrphanPolicy::MoveToRoots).unwrap();

    let after = read_node(&graph, &child).unwrap();
    assert_eq!(after.canonical_path, expected_canonical);
    assert_eq!(after.alias_paths, vec![remaining_alias.clone()]);
    let target = fs::read_link(&remaining_alias).unwrap();
    let resolved = if target.is_absolute() {
        target
    } else {
        remaining_alias.parent().unwrap().join(target)
    };
    assert_eq!(
        resolved.canonicalize().unwrap(),
        expected_canonical.canonicalize().unwrap()
    );
}

#[test]
#[cfg(unix)]
fn broken_symlink_name_is_not_reused_for_new_link() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let parent = add_root_node(&graph, statement("Parent", "")).unwrap();
    let other_parent = add_root_node(&graph, statement("Other Parent", "")).unwrap();
    let child = add_child_node(&graph, &parent, statement("Child", "")).unwrap();
    let other_parent_node = read_node(&graph, &other_parent).unwrap();
    let stale_alias = other_parent_node
        .canonical_path
        .join("children")
        .join(format!("child--{child}"));

    std::os::unix::fs::symlink(temp.path().join("missing"), &stale_alias).unwrap();
    link_existing_node(&graph, &other_parent, &child).unwrap();

    assert!(
        fs::symlink_metadata(&stale_alias)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::symlink_metadata(
            other_parent_node
                .canonical_path
                .join("children")
                .join(format!("child-2--{child}"))
        )
        .unwrap()
        .file_type()
        .is_symlink()
    );
}

#[test]
fn duplicate_canonical_errors_are_preserved() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let parent = add_root_node(&graph, statement("Parent", "")).unwrap();
    let parent_node = read_node(&graph, &parent).unwrap();

    copy_dir_all(
        &parent_node.canonical_path,
        temp.path()
            .join("roots")
            .join(format!("duplicate-parent--{parent}")),
    );

    assert!(matches!(
        add_child_node(&graph, &parent, statement("Child", "")),
        Err(GraphError::DuplicateCanonicalNode { id, .. }) if id == parent
    ));
}

#[test]
fn root_and_child_question_answer_nodes_support_empty_answers() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();

    let root = add_root_node(&graph, qa("Root question", "What is the root?", "")).unwrap();
    let child = add_child_node(
        &graph,
        &root,
        qa("Child question", "What follows?", "An answer."),
    )
    .unwrap();

    assert_uuid_shape(&root);
    assert_uuid_shape(&child);
    assert_eq!(
        read_node(&graph, &root).unwrap().content,
        NodeContent::QuestionAnswer {
            question: "What is the root?".to_string(),
            answer: String::new(),
        }
    );
    assert_eq!(
        list_children(&graph, &root)
            .unwrap()
            .into_iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![child]
    );
}

#[test]
fn question_answer_updates_keep_identity_paths_and_managed_sections() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let id = add_root_node(&graph, qa("Original title", "Old question?", "Old answer.")).unwrap();
    let before = read_node(&graph, &id).unwrap();

    let updated = update_node(
        &graph,
        &id,
        NodePatch {
            title: Some("New title".to_string()),
            content: Some(NodeContent::QuestionAnswer {
                question: "New question?".to_string(),
                answer: "New answer.".to_string(),
            }),
        },
    )
    .unwrap();

    assert_eq!(updated.id, id);
    assert_eq!(updated.canonical_path, before.canonical_path);
    assert_eq!(updated.alias_paths, before.alias_paths);
    assert!(updated.updated_at_unix > before.updated_at_unix);
    assert_eq!(
        updated.content,
        NodeContent::QuestionAnswer {
            question: "New question?".to_string(),
            answer: "New answer.".to_string(),
        }
    );

    let markdown = fs::read_to_string(updated.canonical_path.join("node.md")).unwrap();
    assert!(markdown.contains("title: New title"));
    assert!(markdown.contains("## Question\n\nNew question?"));
    assert!(markdown.contains("## Answer\n\nNew answer."));
    assert!(
        updated
            .canonical_path
            .ends_with(format!("original-title--{id}"))
    );
}

#[test]
#[cfg(unix)]
fn ancestors_descendants_deduplicate_shared_paths_and_ignore_manual_cycle_start() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let alpha = add_child_node(&graph, &root, statement("Alpha Branch", "")).unwrap();
    let beta = add_child_node(&graph, &root, statement("Beta Branch", "")).unwrap();
    let shared = add_child_node(&graph, &beta, statement("Shared", "")).unwrap();
    link_existing_node(&graph, &alpha, &shared).unwrap();

    assert_eq!(
        list_descendants(&graph, &root, TraversalOptions::default())
            .unwrap()
            .into_iter()
            .map(|node| node.title)
            .collect::<Vec<_>>(),
        vec!["Alpha Branch", "Beta Branch", "Shared"]
    );
    assert_eq!(
        list_ancestors(&graph, &shared, TraversalOptions::default())
            .unwrap()
            .into_iter()
            .map(|node| node.title)
            .collect::<Vec<_>>(),
        vec!["Alpha Branch", "Beta Branch", "Root"]
    );

    let root_node = read_node(&graph, &root).unwrap();
    let shared_node = read_node(&graph, &shared).unwrap();
    std::os::unix::fs::symlink(
        &root_node.canonical_path,
        shared_node
            .canonical_path
            .join("children")
            .join(format!("root-cycle--{root}")),
    )
    .unwrap();

    assert!(
        rebuild_index(&graph)
            .unwrap()
            .problems
            .iter()
            .any(|problem| matches!(problem, GraphProblem::CycleDetected { .. }))
    );
    let descendants = list_descendants(&graph, &root, TraversalOptions::default()).unwrap();
    assert!(!descendants.iter().any(|node| node.id == root));
    assert_eq!(
        descendants.iter().filter(|node| node.id == shared).count(),
        1
    );
}

#[test]
fn rebuild_index_reports_sorted_nodes_edges_and_alias_edges() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let other = add_root_node(&graph, statement("Other", "")).unwrap();
    let child = add_child_node(&graph, &root, statement("Child", "")).unwrap();
    link_existing_node(&graph, &other, &child).unwrap();

    let index = rebuild_index(&graph).unwrap();

    assert!(index.problems.is_empty());
    assert_eq!(
        index
            .nodes
            .iter()
            .map(|node| node.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Child", "Other", "Root"]
    );
    assert!(
        index
            .edges
            .iter()
            .any(|edge| { edge.parent_id == root && edge.child_id == child && !edge.is_symlink })
    );
    assert!(
        index
            .edges
            .iter()
            .any(|edge| { edge.parent_id == other && edge.child_id == child && edge.is_symlink })
    );
}

#[test]
#[cfg(unix)]
fn recursive_delete_removes_private_subtree_without_following_outside_symlink() {
    let graph_temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_file = outside.path().join("keep.txt");
    fs::write(&outside_file, "do not remove").unwrap();

    let graph = init_graph(graph_temp.path()).unwrap();
    let root = add_root_node(&graph, statement("Root", "")).unwrap();
    let child = add_child_node(&graph, &root, statement("Child", "")).unwrap();
    let grandchild = add_child_node(&graph, &child, statement("Grandchild", "")).unwrap();
    let root_node = read_node(&graph, &root).unwrap();
    std::os::unix::fs::symlink(
        outside.path(),
        root_node
            .canonical_path
            .join("children")
            .join("outside--550e8400-e29b-41d4-a716-446655440000"),
    )
    .unwrap();

    assert!(
        rebuild_index(&graph)
            .unwrap()
            .problems
            .iter()
            .any(|problem| matches!(problem, GraphProblem::BrokenSymlink { .. }))
    );

    delete_node(&graph, &root, DeleteMode::Recursive).unwrap();

    for id in [root, child, grandchild] {
        assert!(matches!(
            read_node(&graph, &id),
            Err(GraphError::NodeNotFound(_))
        ));
    }
    assert_eq!(fs::read_to_string(outside_file).unwrap(), "do not remove");
}

#[test]
fn example_usage_from_spec_runs_as_documented() {
    let temp = tempdir().unwrap();
    let graph = init_graph(temp.path()).unwrap();

    let rust_id = add_root_node(
        &graph,
        NewNode {
            kind: NodeKind::Statement,
            title: "Rust keeps local tools simple".to_string(),
            content: NodeContent::Statement {
                body: "Rust can manage local files without a background service.".to_string(),
            },
        },
    )
    .unwrap();

    let qa_id = add_child_node(
        &graph,
        &rust_id,
        NewNode {
            kind: NodeKind::QuestionAnswer,
            title: "Why use symlinks?".to_string(),
            content: NodeContent::QuestionAnswer {
                question: "Why use symlinks for shared children?".to_string(),
                answer:
                    "They preserve one canonical Markdown file while allowing multiple parents."
                        .to_string(),
            },
        },
    )
    .unwrap();

    let descendants =
        list_descendants(&graph, &rust_id, TraversalOptions { max_depth: None }).unwrap();

    assert_eq!(descendants[0].id, qa_id);
}

#[test]
fn multiple_open_graph_handles_can_read_the_same_graph() {
    let temp = tempdir().unwrap();
    let writer = init_graph(temp.path()).unwrap();
    let id = add_root_node(&writer, statement("Root", "Readable")).unwrap();

    let reader_one = open_graph(temp.path()).unwrap();
    let reader_two = open_graph(temp.path()).unwrap();

    assert_eq!(read_node(&reader_one, &id).unwrap().title, "Root");
    assert_eq!(list_roots(&reader_two).unwrap()[0].id, id);
}

#[test]
fn spec_traceability_table_points_each_section_to_tests() {
    let spec = include_str!("../../../spec/SPEC.md");

    assert!(spec.contains("## Spec Test Traceability"));
    assert!(spec.contains("| Section | Unit tests | Integration tests |"));
    for section in 1..=28 {
        assert!(
            spec.contains(&format!("| {section} |")),
            "missing traceability row for spec section {section}"
        );
    }
}

fn copy_dir_all(source: impl AsRef<Path>, destination: impl AsRef<Path>) {
    let source = source.as_ref();
    let destination = destination.as_ref();
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_all(from, to);
        } else {
            fs::copy(from, to).unwrap();
        }
    }
}
