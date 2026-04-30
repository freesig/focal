use focal_sqlite::{
    ContextDocumentPatch, DeleteMode, GraphError, GraphProblem, NewContextDocument, NewNode,
    NodeContent, NodeKind, NodePatch, OrphanPolicy, TraversalOptions, add_child_node,
    add_context_document, add_root_node, delete_context_document, delete_node, init_graph,
    link_existing_node, list_ancestors, list_children, list_context_documents, list_descendants,
    list_parents, list_roots, open_database, open_graph, read_context_document, read_node,
    rebuild_index, unlink_child, update_context_document, update_node,
};
use rusqlite::{Connection, params};
use tempfile::NamedTempFile;

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

fn graph_id(connection: &Connection, graph_name: &str) -> i64 {
    connection
        .query_row(
            "SELECT id FROM focal_graphs WHERE name = ?1",
            params![graph_name],
            |row| row.get(0),
        )
        .unwrap()
}

fn drop_placement_indexes(connection: &Connection) {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = OFF;
            DROP INDEX IF EXISTS focal_placements_logical_path_unique;
            DROP INDEX IF EXISTS focal_placements_edge_unique;
            DROP INDEX IF EXISTS focal_placements_root_unique;
            DROP INDEX IF EXISTS focal_placements_canonical_unique;
            ",
        )
        .unwrap();
}

#[test]
fn init_open_and_namespace_isolation() {
    let mut connection = Connection::open_in_memory().unwrap();

    assert!(matches!(
        open_graph(&mut connection, "main"),
        Err(GraphError::InvalidGraphRoot(_))
    ));

    {
        let mut graph = init_graph(&mut connection, "main").unwrap();
        let id = add_root_node(&mut graph, statement("Main Root", "main")).unwrap();
        assert_uuid_shape(&id);
    }

    {
        let graph = open_graph(&mut connection, "main").unwrap();
        assert_eq!(list_roots(&graph).unwrap()[0].title, "Main Root");
    }

    {
        let graph = init_graph(&mut connection, "other").unwrap();
        assert!(list_roots(&graph).unwrap().is_empty());
    }

    assert!(matches!(
        open_graph(&mut connection, "missing"),
        Err(GraphError::InvalidGraphRoot(_))
    ));
    assert!(matches!(
        init_graph(&mut connection, " \n"),
        Err(GraphError::InvalidGraphRoot(_))
    ));
}

#[test]
fn open_graph_requires_context_schema_table() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "
            CREATE TABLE focal_graphs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at_unix INTEGER NOT NULL
            );
            CREATE TABLE focal_nodes (
                graph_id INTEGER NOT NULL,
                id TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                statement_body TEXT,
                qa_question TEXT,
                qa_answer TEXT,
                created_at_unix INTEGER NOT NULL,
                updated_at_unix INTEGER NOT NULL
            );
            CREATE TABLE focal_placements (
                id INTEGER PRIMARY KEY,
                graph_id INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                parent_id TEXT,
                slug TEXT NOT NULL,
                logical_path TEXT NOT NULL,
                is_canonical INTEGER NOT NULL
            );
            INSERT INTO focal_graphs (name, created_at_unix) VALUES ('main', 1);
            ",
        )
        .unwrap();

    assert!(matches!(
        open_graph(&mut connection, "main"),
        Err(GraphError::InvalidGraphRoot(message))
            if message.contains("focal_context_documents")
    ));
}

#[test]
fn open_graph_rejects_context_schema_without_required_constraints() {
    let mut missing_primary_key = Connection::open_in_memory().unwrap();
    missing_primary_key
        .execute_batch(
            "
            CREATE TABLE focal_graphs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at_unix INTEGER NOT NULL
            );
            CREATE TABLE focal_nodes (
                graph_id INTEGER NOT NULL,
                id TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                statement_body TEXT,
                qa_question TEXT,
                qa_answer TEXT,
                created_at_unix INTEGER NOT NULL,
                updated_at_unix INTEGER NOT NULL,
                PRIMARY KEY (graph_id, id)
            );
            CREATE TABLE focal_context_documents (
                graph_id INTEGER NOT NULL,
                id TEXT NOT NULL,
                slug TEXT NOT NULL,
                filename TEXT NOT NULL,
                title TEXT NOT NULL,
                markdown TEXT NOT NULL,
                created_at_unix INTEGER NOT NULL,
                updated_at_unix INTEGER NOT NULL,
                UNIQUE (graph_id, filename)
            );
            CREATE TABLE focal_placements (
                id INTEGER PRIMARY KEY,
                graph_id INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                parent_id TEXT,
                slug TEXT NOT NULL,
                logical_path TEXT NOT NULL,
                is_canonical INTEGER NOT NULL
            );
            INSERT INTO focal_graphs (name, created_at_unix) VALUES ('main', 1);
            ",
        )
        .unwrap();

    assert!(matches!(
        open_graph(&mut missing_primary_key, "main"),
        Err(GraphError::InvalidGraphRoot(message))
            if message.contains("focal_context_documents")
                && message.contains("primary key")
    ));

    let mut missing_filename_unique = Connection::open_in_memory().unwrap();
    missing_filename_unique
        .execute_batch(
            "
            CREATE TABLE focal_graphs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at_unix INTEGER NOT NULL
            );
            CREATE TABLE focal_nodes (
                graph_id INTEGER NOT NULL,
                id TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                statement_body TEXT,
                qa_question TEXT,
                qa_answer TEXT,
                created_at_unix INTEGER NOT NULL,
                updated_at_unix INTEGER NOT NULL,
                PRIMARY KEY (graph_id, id)
            );
            CREATE TABLE focal_context_documents (
                graph_id INTEGER NOT NULL,
                id TEXT NOT NULL,
                slug TEXT NOT NULL,
                filename TEXT NOT NULL,
                title TEXT NOT NULL,
                markdown TEXT NOT NULL,
                created_at_unix INTEGER NOT NULL,
                updated_at_unix INTEGER NOT NULL,
                PRIMARY KEY (graph_id, id)
            );
            CREATE TABLE focal_placements (
                id INTEGER PRIMARY KEY,
                graph_id INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                parent_id TEXT,
                slug TEXT NOT NULL,
                logical_path TEXT NOT NULL,
                is_canonical INTEGER NOT NULL
            );
            INSERT INTO focal_graphs (name, created_at_unix) VALUES ('main', 1);
            ",
        )
        .unwrap();

    assert!(matches!(
        open_graph(&mut missing_filename_unique, "main"),
        Err(GraphError::InvalidGraphRoot(message))
            if message.contains("focal_context_documents")
                && message.contains("unique key")
                && message.contains("filename")
    ));
}

#[test]
fn context_documents_crud_namespace_sort_and_stable_filename() {
    let mut connection = Connection::open_in_memory().unwrap();
    let (root, alpha, beta, original_filename, original_path) = {
        let mut graph = init_graph(&mut connection, "main").unwrap();
        let root = add_root_node(&mut graph, statement("Root", "node body")).unwrap();
        let beta = add_context_document(
            &mut graph,
            NewContextDocument {
                title: "Beta notes".to_string(),
                markdown: "# Human heading\n\nBeta body".to_string(),
            },
        )
        .unwrap();
        let alpha = add_context_document(
            &mut graph,
            NewContextDocument {
                title: "Alpha notes".to_string(),
                markdown: String::new(),
            },
        )
        .unwrap();
        assert_uuid_shape(&beta);
        assert_uuid_shape(&alpha);

        let listed = list_context_documents(&graph).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|context| context.id.as_str())
                .collect::<Vec<_>>(),
            vec![alpha.as_str(), beta.as_str()]
        );
        assert!(listed[0].path.starts_with("context"));

        let before = read_context_document(&graph, &beta).unwrap();
        assert_eq!(before.markdown, "# Human heading\n\nBeta body");
        assert!(before.filename.ends_with(format!("--{beta}.md").as_str()));
        let original_filename = before.filename.clone();
        let original_path = before.path.clone();

        let updated = update_context_document(
            &mut graph,
            &beta,
            ContextDocumentPatch {
                title: Some("Renamed context".to_string()),
                markdown: Some("Updated body".to_string()),
            },
        )
        .unwrap();
        assert_eq!(updated.id, beta);
        assert_eq!(updated.filename, original_filename);
        assert_eq!(updated.path, original_path);
        assert_eq!(updated.markdown, "Updated body");
        assert!(updated.updated_at_unix > before.updated_at_unix);

        let index = rebuild_index(&graph).unwrap();
        assert_eq!(index.contexts.len(), 2);
        assert_eq!(read_node(&graph, &root).unwrap().title, "Root");

        (root, alpha, beta, updated.filename, updated.path)
    };

    {
        let mut other = init_graph(&mut connection, "other").unwrap();
        assert!(list_context_documents(&other).unwrap().is_empty());
        add_context_document(
            &mut other,
            NewContextDocument {
                title: "Beta notes".to_string(),
                markdown: "Other graph".to_string(),
            },
        )
        .unwrap();
        assert_eq!(list_context_documents(&other).unwrap().len(), 1);
    }

    {
        let mut graph = open_graph(&mut connection, "main").unwrap();
        assert_eq!(
            read_context_document(&graph, &beta).unwrap().filename,
            original_filename
        );
        assert_eq!(
            read_context_document(&graph, &beta).unwrap().path,
            original_path
        );
        delete_context_document(&mut graph, &beta).unwrap();
        assert!(matches!(
            read_context_document(&graph, &beta),
            Err(GraphError::ContextNotFound(id)) if id == beta
        ));
        assert_eq!(
            list_context_documents(&graph)
                .unwrap()
                .into_iter()
                .map(|context| context.id)
                .collect::<Vec<_>>(),
            vec![alpha]
        );
        assert_eq!(read_node(&graph, &root).unwrap().id, root);
    }
}

#[test]
fn context_documents_reject_invalid_inputs_and_report_bad_rows() {
    let mut connection = Connection::open_in_memory().unwrap();
    let valid = {
        let mut graph = init_graph(&mut connection, "main").unwrap();
        let valid = add_context_document(
            &mut graph,
            NewContextDocument {
                title: "Valid".to_string(),
                markdown: "Body".to_string(),
            },
        )
        .unwrap();
        assert!(matches!(
            add_context_document(
                &mut graph,
                NewContextDocument {
                    title: " \n\t ".to_string(),
                    markdown: String::new(),
                },
            ),
            Err(GraphError::InvalidTitle)
        ));
        assert!(matches!(
            read_context_document(&graph, "../not-an-id"),
            Err(GraphError::InvalidContextId(_))
        ));
        valid
    };
    let graph = graph_id(&connection, "main");
    let bad_id = "550e8400-e29b-41d4-a716-446655440000";
    let wrong_suffix = "7d9f2e5c-0f22-4c18-a0be-9f23e772a0bc";

    connection
        .execute(
            "INSERT INTO focal_context_documents \
             (graph_id, id, slug, filename, title, markdown, created_at_unix, updated_at_unix) \
             VALUES (?1, 'not-a-uuid', 'bad', 'bad--not-a-uuid.md', 'Bad ID', '', 1, 1)",
            params![graph],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO focal_context_documents \
             (graph_id, id, slug, filename, title, markdown, created_at_unix, updated_at_unix) \
             VALUES (?1, ?2, 'mismatch', ?3, 'Mismatch', '', 1, 1)",
            params![graph, bad_id, format!("mismatch--{wrong_suffix}.md")],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE focal_context_documents \
             SET title = '' WHERE graph_id = ?1 AND id = ?2",
            params![graph, valid],
        )
        .unwrap();

    let graph = open_graph(&mut connection, "main").unwrap();
    let problems = rebuild_index(&graph).unwrap().problems;
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidContextMarkdown { reason, .. }
            if reason.contains("invalid context id")
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidContextMarkdown { reason, .. }
            if reason == "context filename id suffix does not match context id"
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidContextMarkdown { reason, .. } if reason == "invalid title"
    )));
    assert!(matches!(
        list_context_documents(&graph),
        Err(GraphError::InvalidContextMarkdown { .. })
    ));
}

#[test]
fn add_read_update_and_path_stability() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let root = add_root_node(
        &mut graph,
        statement("Rust keeps local tools simple", "Body"),
    )
    .unwrap();
    let child = add_child_node(&mut graph, &root, qa("Why", "Why SQLite?", "")).unwrap();

    let root_node = read_node(&graph, &root).unwrap();
    assert_eq!(root_node.title, "Rust keeps local tools simple");
    assert!(
        root_node
            .canonical_path
            .ends_with(format!("rust-keeps-local-tools-simple--{root}"))
    );
    assert_eq!(
        read_node(&graph, &child).unwrap().content,
        NodeContent::QuestionAnswer {
            question: "Why SQLite?".to_string(),
            answer: String::new(),
        }
    );

    let original_path = root_node.canonical_path.clone();
    let updated = update_node(
        &mut graph,
        &root,
        NodePatch {
            title: Some("A better title".to_string()),
            content: Some(NodeContent::Statement {
                body: "Updated body".to_string(),
            }),
        },
    )
    .unwrap();

    assert_eq!(updated.id, root);
    assert_eq!(updated.title, "A better title");
    assert_eq!(updated.canonical_path, original_path);
    assert!(updated.updated_at_unix > root_node.updated_at_unix);
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
fn root_question_answer_nodes_update_without_path_changes() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let root = add_root_node(&mut graph, qa("Original title", "What is stored?", "")).unwrap();

    let before = read_node(&graph, &root).unwrap();
    assert_eq!(
        before.content,
        NodeContent::QuestionAnswer {
            question: "What is stored?".to_string(),
            answer: String::new(),
        }
    );
    assert_eq!(list_roots(&graph).unwrap()[0].id, root);

    let updated = update_node(
        &mut graph,
        &root,
        NodePatch {
            title: Some("Updated title".to_string()),
            content: Some(NodeContent::QuestionAnswer {
                question: "What changed?".to_string(),
                answer: "The row content changed.".to_string(),
            }),
        },
    )
    .unwrap();

    assert_eq!(updated.id, root);
    assert_eq!(updated.kind, NodeKind::QuestionAnswer);
    assert_eq!(updated.canonical_path, before.canonical_path);
    assert_eq!(updated.alias_paths, before.alias_paths);
    assert!(updated.updated_at_unix > before.updated_at_unix);
    assert_eq!(
        updated.content,
        NodeContent::QuestionAnswer {
            question: "What changed?".to_string(),
            answer: "The row content changed.".to_string(),
        }
    );
    assert_eq!(list_roots(&graph).unwrap()[0].title, "Updated title");
}

#[test]
fn rejects_empty_titles_and_questions() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();

    assert!(matches!(
        add_root_node(&mut graph, statement(" \n\t ", "")),
        Err(GraphError::InvalidTitle)
    ));
    assert!(matches!(
        add_root_node(&mut graph, qa("Question", " \n\t ", "")),
        Err(GraphError::InvalidMarkdown { reason, .. })
            if reason == "question must not be empty"
    ));

    let parent = add_root_node(&mut graph, statement("Parent", "")).unwrap();
    assert!(matches!(
        add_child_node(&mut graph, &parent, qa("Child Question", "", "")),
        Err(GraphError::InvalidMarkdown { reason, .. })
            if reason == "question must not be empty"
    ));
}

#[test]
fn linking_is_idempotent_and_rejects_cycles() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let parent = add_root_node(&mut graph, statement("Parent", "")).unwrap();
    let other_parent = add_root_node(&mut graph, statement("Other Parent", "")).unwrap();
    let child = add_child_node(&mut graph, &parent, statement("Child", "")).unwrap();

    link_existing_node(&mut graph, &other_parent, &child).unwrap();
    link_existing_node(&mut graph, &other_parent, &child).unwrap();

    assert_eq!(list_parents(&graph, &child).unwrap().len(), 2);
    let child_node = read_node(&graph, &child).unwrap();
    assert_eq!(child_node.alias_paths.len(), 1);

    let index = rebuild_index(&graph).unwrap();
    assert!(index.edges.iter().any(|edge| {
        edge.parent_id == other_parent && edge.child_id == child && edge.is_symlink
    }));
    assert!(matches!(
        link_existing_node(&mut graph, &child, &parent),
        Err(GraphError::CycleDetected)
    ));
}

#[test]
fn root_linking_and_orphan_move_rewrite_logical_paths() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let parent = add_root_node(&mut graph, statement("Parent", "")).unwrap();
    let child = add_root_node(&mut graph, statement("Child", "")).unwrap();

    link_existing_node(&mut graph, &parent, &child).unwrap();
    assert!(
        !list_roots(&graph)
            .unwrap()
            .iter()
            .any(|node| node.id == child)
    );
    assert_eq!(list_parents(&graph, &child).unwrap()[0].id, parent);

    unlink_child(&mut graph, &parent, &child, OrphanPolicy::MoveToRoots).unwrap();
    let child_node = read_node(&graph, &child).unwrap();
    assert!(child_node.canonical_path.starts_with("roots"));
    assert!(
        list_roots(&graph)
            .unwrap()
            .iter()
            .any(|node| node.id == child)
    );
}

#[test]
fn unlinking_alias_parent_preserves_canonical_node() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let canonical_parent = add_root_node(&mut graph, statement("Canonical Parent", "")).unwrap();
    let alias_parent = add_root_node(&mut graph, statement("Alias Parent", "")).unwrap();
    let child =
        add_child_node(&mut graph, &canonical_parent, statement("Child", "content")).unwrap();

    link_existing_node(&mut graph, &alias_parent, &child).unwrap();
    let before = read_node(&graph, &child).unwrap();
    assert_eq!(before.alias_paths.len(), 1);

    unlink_child(
        &mut graph,
        &alias_parent,
        &child,
        OrphanPolicy::FailIfWouldOrphan,
    )
    .unwrap();

    let after = read_node(&graph, &child).unwrap();
    assert_eq!(after.canonical_path, before.canonical_path);
    assert_eq!(after.content, before.content);
    assert!(after.alias_paths.is_empty());
    assert_eq!(
        list_children(&graph, &canonical_parent)
            .unwrap()
            .into_iter()
            .map(|node| (node.id, node.is_alias))
            .collect::<Vec<_>>(),
        vec![(child.clone(), false)]
    );
    assert!(list_children(&graph, &alias_parent).unwrap().is_empty());
    assert_eq!(
        list_parents(&graph, &child).unwrap()[0].id,
        canonical_parent
    );
}

#[test]
fn unlinking_canonical_parent_promotes_alias_and_preserves_subtree() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let old_parent = add_root_node(&mut graph, statement("Old Parent", "")).unwrap();
    let new_parent = add_root_node(&mut graph, statement("New Parent", "")).unwrap();
    let child = add_child_node(&mut graph, &old_parent, statement("Child", "content")).unwrap();
    let grandchild = add_child_node(&mut graph, &child, statement("Grandchild", "")).unwrap();

    link_existing_node(&mut graph, &new_parent, &child).unwrap();
    let alias_path = read_node(&graph, &child).unwrap().alias_paths[0].clone();

    unlink_child(&mut graph, &old_parent, &child, OrphanPolicy::MoveToRoots).unwrap();

    let after = read_node(&graph, &child).unwrap();
    assert_eq!(after.canonical_path, alias_path);
    assert!(after.alias_paths.is_empty());
    assert_eq!(list_children(&graph, &new_parent).unwrap()[0].id, child);
    assert_eq!(list_children(&graph, &child).unwrap()[0].id, grandchild);
}

#[test]
fn promotion_with_multiple_aliases_chooses_first_logical_path() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let old_parent = add_root_node(&mut graph, statement("Old Parent", "")).unwrap();
    let alpha_parent = add_root_node(&mut graph, statement("Alpha Parent", "")).unwrap();
    let beta_parent = add_root_node(&mut graph, statement("Beta Parent", "")).unwrap();
    let child = add_child_node(&mut graph, &old_parent, statement("Child", "content")).unwrap();

    link_existing_node(&mut graph, &beta_parent, &child).unwrap();
    link_existing_node(&mut graph, &alpha_parent, &child).unwrap();
    let before = read_node(&graph, &child).unwrap();
    assert_eq!(before.alias_paths.len(), 2);
    let expected_canonical = before.alias_paths[0].clone();
    let remaining_alias = before.alias_paths[1].clone();

    unlink_child(&mut graph, &old_parent, &child, OrphanPolicy::MoveToRoots).unwrap();

    let after = read_node(&graph, &child).unwrap();
    assert_eq!(after.canonical_path, expected_canonical);
    assert_eq!(after.alias_paths, vec![remaining_alias]);
    assert!(
        list_children(&graph, &alpha_parent)
            .unwrap()
            .iter()
            .any(|node| node.id == child && !node.is_alias)
    );
    assert!(
        list_children(&graph, &beta_parent)
            .unwrap()
            .iter()
            .any(|node| node.id == child && node.is_alias)
    );
}

#[test]
fn unlink_orphan_policies_fail_and_delete() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let parent = add_root_node(&mut graph, statement("Parent", "")).unwrap();
    let child = add_child_node(&mut graph, &parent, statement("Child", "")).unwrap();

    assert!(matches!(
        unlink_child(&mut graph, &parent, &child, OrphanPolicy::FailIfWouldOrphan),
        Err(GraphError::WouldOrphanNode(id)) if id == child
    ));
    assert_eq!(list_children(&graph, &parent).unwrap().len(), 1);

    unlink_child(&mut graph, &parent, &child, OrphanPolicy::DeleteIfNoParents).unwrap();
    assert!(matches!(
        read_node(&graph, &child),
        Err(GraphError::NodeNotFound(_))
    ));
}

#[test]
fn recursive_delete_preserves_shared_descendants_by_promotion() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let deleting_root = add_root_node(&mut graph, statement("Deleting Root", "")).unwrap();
    let outside_root = add_root_node(&mut graph, statement("Outside Root", "")).unwrap();
    let shared = add_child_node(&mut graph, &deleting_root, statement("Shared", "")).unwrap();

    link_existing_node(&mut graph, &outside_root, &shared).unwrap();
    let alias_path = read_node(&graph, &shared).unwrap().alias_paths[0].clone();

    delete_node(&mut graph, &deleting_root, DeleteMode::Recursive).unwrap();

    assert!(matches!(
        read_node(&graph, &deleting_root),
        Err(GraphError::NodeNotFound(_))
    ));
    let shared_node = read_node(&graph, &shared).unwrap();
    assert_eq!(shared_node.canonical_path, alias_path);
    assert_eq!(list_parents(&graph, &shared).unwrap()[0].id, outside_root);
}

#[test]
fn delete_modes_and_deterministic_traversal() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let root = add_root_node(&mut graph, statement("Root", "")).unwrap();
    let beta = add_child_node(&mut graph, &root, statement("Beta", "")).unwrap();
    let alpha = add_child_node(&mut graph, &root, statement("Alpha", "")).unwrap();
    let gamma = add_child_node(&mut graph, &alpha, statement("Gamma", "")).unwrap();
    link_existing_node(&mut graph, &beta, &gamma).unwrap();

    assert!(matches!(
        delete_node(&mut graph, &root, DeleteMode::FailIfHasChildren),
        Err(GraphError::NodeHasChildren(id)) if id == root
    ));
    assert_eq!(
        list_descendants(&graph, &root, TraversalOptions::default())
            .unwrap()
            .into_iter()
            .map(|node| node.title)
            .collect::<Vec<_>>(),
        vec!["Alpha", "Beta", "Gamma"]
    );
    assert_eq!(
        list_descendants(&graph, &root, TraversalOptions { max_depth: Some(1) })
            .unwrap()
            .into_iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![alpha.clone(), beta]
    );
    assert_eq!(
        list_ancestors(&graph, &gamma, TraversalOptions::default())
            .unwrap()
            .into_iter()
            .map(|node| node.title)
            .collect::<Vec<_>>(),
        vec!["Alpha", "Beta", "Root"]
    );
}

#[test]
fn rebuild_index_reports_sorted_nodes_edges_and_alias_edges() {
    let mut connection = Connection::open_in_memory().unwrap();
    let mut graph = init_graph(&mut connection, "main").unwrap();
    let root = add_root_node(&mut graph, statement("Root", "")).unwrap();
    let other = add_root_node(&mut graph, statement("Other", "")).unwrap();
    let child = add_child_node(&mut graph, &root, statement("Child", "")).unwrap();
    link_existing_node(&mut graph, &other, &child).unwrap();

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
fn rebuild_index_reports_sqlite_storage_problems() {
    let mut connection = Connection::open_in_memory().unwrap();
    let (graph, parent, child, qa_node, mismatch, parent_path, child_path) = {
        let mut graph = init_graph(&mut connection, "main").unwrap();
        let parent = add_root_node(&mut graph, statement("Parent", "")).unwrap();
        let child = add_child_node(&mut graph, &parent, statement("Child", "")).unwrap();
        let qa_node = add_root_node(&mut graph, qa("Question", "Why?", "")).unwrap();
        let mismatch = add_root_node(&mut graph, statement("Mismatch", "")).unwrap();
        let parent_path = read_node(&graph, &parent)
            .unwrap()
            .canonical_path
            .to_string_lossy()
            .into_owned();
        let child_path = read_node(&graph, &child)
            .unwrap()
            .canonical_path
            .to_string_lossy()
            .into_owned();
        (
            graph_id(&connection, "main"),
            parent,
            child,
            qa_node,
            mismatch,
            parent_path,
            child_path,
        )
    };
    drop_placement_indexes(&connection);
    let missing = "550e8400-e29b-41d4-a716-446655440000";
    let wrong_suffix = "7d9f2e5c-0f22-4c18-a0be-9f23e772a0bc";

    connection
        .execute(
            "INSERT INTO focal_placements \
             (graph_id, node_id, parent_id, slug, logical_path, is_canonical) \
             VALUES (?1, ?2, ?3, 'missing', ?4, 0)",
            params![
                graph,
                missing,
                parent,
                format!("{parent_path}/children/missing--{missing}")
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO focal_placements \
             (graph_id, node_id, parent_id, slug, logical_path, is_canonical) \
             VALUES (?1, ?2, ?3, 'duplicate', ?4, 0)",
            params![
                graph,
                child,
                parent,
                format!("{parent_path}/children/duplicate--{child}")
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO focal_placements \
             (graph_id, node_id, parent_id, slug, logical_path, is_canonical) \
             VALUES (?1, ?2, NULL, 'second-canonical', ?3, 1)",
            params![graph, child, format!("roots/second-canonical--{child}")],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE focal_nodes \
             SET qa_question = '' WHERE graph_id = ?1 AND id = ?2",
            params![graph, qa_node],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO focal_nodes \
             (graph_id, id, kind, title, statement_body, qa_question, qa_answer, \
              created_at_unix, updated_at_unix) \
             VALUES (?1, 'not-a-uuid', 'statement', 'Bad ID', 'body', NULL, NULL, 1, 1)",
            params![graph],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO focal_placements \
             (graph_id, node_id, parent_id, slug, logical_path, is_canonical) \
             VALUES (?1, 'not-a-uuid', NULL, 'bad', 'roots/bad--not-a-uuid', 1)",
            params![graph],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO focal_placements \
             (graph_id, node_id, parent_id, slug, logical_path, is_canonical) \
             VALUES (?1, ?2, ?3, 'mismatch', ?4, 0)",
            params![
                graph,
                mismatch,
                parent,
                format!("{parent_path}/children/mismatch--{wrong_suffix}")
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO focal_placements \
             (graph_id, node_id, parent_id, slug, logical_path, is_canonical) \
             VALUES (?1, ?2, ?3, 'parent-cycle', ?4, 0)",
            params![
                graph,
                parent,
                child,
                format!("{child_path}/children/parent-cycle--{parent}")
            ],
        )
        .unwrap();

    let graph = open_graph(&mut connection, "main").unwrap();
    let problems = rebuild_index(&graph).unwrap().problems;

    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { reason, .. }
            if reason.contains("missing node")
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { reason, .. }
            if reason == "duplicate placement for parent-child edge"
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::DuplicateCanonicalNode { id, .. } if id == &child
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { reason, .. } if reason == "question must not be empty"
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { reason, .. } if reason.contains("invalid node id")
    )));
    assert!(problems.iter().any(|problem| matches!(
        problem,
        GraphProblem::InvalidMarkdown { reason, .. }
            if reason == "logical path node id suffix does not match placement node id"
    )));
    assert!(
        problems
            .iter()
            .any(|problem| matches!(problem, GraphProblem::CycleDetected { .. }))
    );
}

#[test]
fn failed_multi_row_write_rolls_back_transaction() {
    let mut connection = Connection::open_in_memory().unwrap();
    {
        let graph = init_graph(&mut connection, "main").unwrap();
        assert!(list_roots(&graph).unwrap().is_empty());
    }
    connection
        .execute_batch(
            "
            CREATE TRIGGER fail_placement_insert
            BEFORE INSERT ON focal_placements
            BEGIN
                SELECT RAISE(ABORT, 'placement insert failed');
            END;
            ",
        )
        .unwrap();

    {
        let mut graph = open_graph(&mut connection, "main").unwrap();
        assert!(matches!(
            add_root_node(&mut graph, statement("Should Roll Back", "")),
            Err(GraphError::Storage(_))
        ));
    }
    connection
        .execute_batch("DROP TRIGGER fail_placement_insert;")
        .unwrap();
    let graph = open_graph(&mut connection, "main").unwrap();
    assert!(list_roots(&graph).unwrap().is_empty());
}

#[test]
fn open_database_opens_path_and_preserves_named_graph() {
    let file = NamedTempFile::new().unwrap();

    {
        let mut connection = open_database(file.path()).unwrap();
        let mut graph = init_graph(&mut connection, "main").unwrap();
        add_root_node(&mut graph, statement("Stored Root", "body")).unwrap();
    }

    let mut connection = open_database(file.path()).unwrap();
    let graph = open_graph(&mut connection, "main").unwrap();
    assert_eq!(
        list_roots(&graph)
            .unwrap()
            .into_iter()
            .map(|node| node.title)
            .collect::<Vec<_>>(),
        vec!["Stored Root"]
    );
}
