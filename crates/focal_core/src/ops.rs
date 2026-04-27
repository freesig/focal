use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GraphError;
use crate::fs_utils::{
    NODE_FILE, children_path, create_relative_dir_symlink, ensure_real_dir_inside,
    generate_node_id, is_path_inside_any, node_file_path, now_unix, path_sort_key, real_dir_exists,
    roots_path, safe_remove_dir_all, safe_remove_file, safe_rename, unique_node_path,
    validate_node_id, validate_title, write_file_atomically,
};
use crate::markdown::render_node_markdown;
use crate::model::{
    DeleteMode, GraphIndex, IdeaGraph, NewNode, Node, NodeContent, NodeId, NodeKind, NodePatch,
    NodeSummary, OrphanPolicy, TraversalOptions,
};
use crate::scan::{
    ScanResult, ScannedNode, broken_symlink_under_parent, canonical_node, canonical_path,
    child_edges, find_edge, graph_index, node_summary, parent_edges, scan_graph, sort_summaries,
    summary_for,
};

pub fn init_graph(root: impl AsRef<Path>) -> Result<IdeaGraph, GraphError> {
    let root = crate::fs_utils::absolute_path(root.as_ref())?;
    fs::create_dir_all(roots_path(&root))?;
    let root = root.canonicalize()?;
    Ok(IdeaGraph { root })
}

pub fn open_graph(root: impl AsRef<Path>) -> Result<IdeaGraph, GraphError> {
    let root = crate::fs_utils::canonicalize_existing(root.as_ref())?;
    if !real_dir_exists(&roots_path(&root))? {
        return Err(GraphError::InvalidGraphRoot(format!(
            "{} does not contain roots/",
            root.display()
        )));
    }
    Ok(IdeaGraph { root })
}

pub fn add_root_node(graph: &IdeaGraph, node: NewNode) -> Result<NodeId, GraphError> {
    let roots = roots_path(&graph.root);
    create_node_in_container(graph, &roots, node)
}

pub fn add_child_node(
    graph: &IdeaGraph,
    parent_id: &str,
    node: NewNode,
) -> Result<NodeId, GraphError> {
    validate_node_id(parent_id)?;
    let scan = scan_graph(graph)?;
    let parent = require_parent_node(&scan, parent_id)?;
    let parent_path =
        canonical_path(parent).ok_or_else(|| GraphError::ParentNotFound(parent_id.to_string()))?;
    let container = children_path(parent_path);
    if !real_dir_exists(&container)? {
        return Err(GraphError::MissingChildrenDirectory(
            parent_path.to_path_buf(),
        ));
    }
    create_node_in_container(graph, &container, node)
}

pub fn read_node(graph: &IdeaGraph, node_id: &str) -> Result<Node, GraphError> {
    validate_node_id(node_id)?;
    let scan = scan_graph(graph)?;
    let node = match canonical_node(&scan, node_id) {
        Ok(node) => node,
        Err(GraphError::NodeNotFound(_)) => {
            if let Some(path) = broken_symlink_for_id(&scan, node_id) {
                return Err(GraphError::BrokenSymlink(path));
            }
            return Err(GraphError::NodeNotFound(node_id.to_string()));
        }
        Err(error) => return Err(error),
    };
    node_from_scanned(node)
}

pub fn update_node(graph: &IdeaGraph, node_id: &str, patch: NodePatch) -> Result<Node, GraphError> {
    validate_node_id(node_id)?;
    let scan = scan_graph(graph)?;
    let node = canonical_node(&scan, node_id)?;
    let canonical_path =
        canonical_path(node).ok_or_else(|| GraphError::NodeNotFound(node_id.to_string()))?;

    let title = match patch.title {
        Some(title) => {
            validate_title(&title)?;
            title
        }
        None => node.title.clone(),
    };
    let content = match patch.content {
        Some(content) => {
            validate_content_matches(&node.kind, &content, &graph.root)?;
            validate_content_values(&content, &graph.root)?;
            content
        }
        None => node.content.clone(),
    };
    let updated_at_unix = now_unix().max(node.updated_at_unix.saturating_add(1));
    write_node_markdown(
        canonical_path,
        node_id,
        &node.kind,
        &title,
        node.created_at_unix,
        updated_at_unix,
        &content,
    )?;

    read_node(graph, node_id)
}

pub fn delete_node(graph: &IdeaGraph, node_id: &str, mode: DeleteMode) -> Result<(), GraphError> {
    validate_node_id(node_id)?;
    let scan = scan_graph(graph)?;
    canonical_node(&scan, node_id)?;

    if mode == DeleteMode::FailIfHasChildren && !child_edges(&scan, node_id).is_empty() {
        return Err(GraphError::NodeHasChildren(node_id.to_string()));
    }

    let delete_set = match mode {
        DeleteMode::FailIfHasChildren => BTreeSet::from([node_id.to_string()]),
        DeleteMode::Recursive => compute_delete_set(&scan, node_id),
    };
    perform_delete_set(graph, &scan, &delete_set)
}

pub fn link_existing_node(
    graph: &IdeaGraph,
    parent_id: &str,
    child_id: &str,
) -> Result<(), GraphError> {
    validate_node_id(parent_id)?;
    validate_node_id(child_id)?;
    if parent_id == child_id {
        return Err(GraphError::CycleDetected);
    }

    let scan = scan_graph(graph)?;
    let parent = require_parent_node(&scan, parent_id)?;
    let child = require_child_node(&scan, child_id)?;

    if find_edge(&scan, parent_id, child_id).is_some() {
        return Ok(());
    }
    if reaches(&scan, child_id, parent_id) {
        return Err(GraphError::CycleDetected);
    }

    let parent_path =
        canonical_path(parent).ok_or_else(|| GraphError::ParentNotFound(parent_id.to_string()))?;
    let child_path =
        canonical_path(child).ok_or_else(|| GraphError::ChildNotFound(child_id.to_string()))?;
    let container = children_path(parent_path);
    if !real_dir_exists(&container)? {
        return Err(GraphError::MissingChildrenDirectory(
            parent_path.to_path_buf(),
        ));
    }
    let link_path = unique_node_path(&container, &child.title, child_id)?;
    create_relative_dir_symlink(&graph.root, child_path, &link_path)
}

pub fn unlink_child(
    graph: &IdeaGraph,
    parent_id: &str,
    child_id: &str,
    orphan_policy: OrphanPolicy,
) -> Result<(), GraphError> {
    validate_node_id(parent_id)?;
    validate_node_id(child_id)?;
    let scan = scan_graph(graph)?;
    require_parent_node(&scan, parent_id)?;
    let child = require_child_node(&scan, child_id)?;
    let edge = find_edge(&scan, parent_id, child_id)
        .ok_or_else(|| GraphError::ChildNotFound(child_id.to_string()))?
        .clone();

    let remaining_parents = parent_edges(&scan, child_id)
        .into_iter()
        .filter(|candidate| candidate.path != edge.path)
        .collect::<Vec<_>>();
    let canonical_is_root = canonical_path(child)
        .is_some_and(|path| path.parent() == Some(roots_path(&graph.root).as_path()));
    let would_orphan = remaining_parents.is_empty() && !canonical_is_root;

    if would_orphan && orphan_policy == OrphanPolicy::FailIfWouldOrphan {
        return Err(GraphError::WouldOrphanNode(child_id.to_string()));
    }
    if would_orphan && orphan_policy == OrphanPolicy::DeleteIfNoParents {
        return delete_node(graph, child_id, DeleteMode::Recursive);
    }

    if edge.is_symlink {
        safe_remove_file(&graph.root, &edge.path)?;
        return Ok(());
    }

    if !remaining_parents.is_empty() {
        let alias_path = remaining_parents
            .iter()
            .filter(|edge| edge.is_symlink)
            .map(|edge| edge.path.clone())
            .min_by_key(|path| path_sort_key(path))
            .ok_or_else(|| GraphError::AliasConflict(edge.path.clone()))?;
        promote_to_alias(graph, child, &alias_path)?;
        return Ok(());
    }

    if orphan_policy == OrphanPolicy::MoveToRoots {
        move_node_to_roots(graph, child)?;
        return Ok(());
    }

    safe_remove_dir_all(&graph.root, &edge.path)
}

pub fn list_roots(graph: &IdeaGraph) -> Result<Vec<NodeSummary>, GraphError> {
    let scan = scan_graph(graph)?;
    let mut by_id = BTreeMap::new();
    for entry in &scan.root_entries {
        if let Some(summary) = summary_for(&scan, &entry.id, entry.is_symlink) {
            by_id.entry(entry.id.clone()).or_insert(summary);
        }
    }
    let mut summaries = by_id.into_values().collect::<Vec<_>>();
    sort_summaries(&mut summaries);
    Ok(summaries)
}

pub fn list_children(graph: &IdeaGraph, node_id: &str) -> Result<Vec<NodeSummary>, GraphError> {
    validate_node_id(node_id)?;
    let scan = scan_graph(graph)?;
    canonical_node(&scan, node_id)?;
    if let Some(path) = broken_symlink_under_parent(&scan, node_id) {
        return Err(GraphError::BrokenSymlink(path));
    }
    let mut summaries = Vec::new();
    for edge in child_edges(&scan, node_id) {
        let child = canonical_node(&scan, &edge.child_id)?;
        if let Some(summary) = node_summary(child, edge.is_symlink) {
            summaries.push(summary);
        }
    }
    sort_summaries(&mut summaries);
    Ok(summaries)
}

pub fn list_parents(graph: &IdeaGraph, node_id: &str) -> Result<Vec<NodeSummary>, GraphError> {
    validate_node_id(node_id)?;
    let scan = scan_graph(graph)?;
    canonical_node(&scan, node_id)?;
    let mut summaries = Vec::new();
    for edge in parent_edges(&scan, node_id) {
        let parent = canonical_node(&scan, &edge.parent_id)?;
        if let Some(summary) = node_summary(parent, false) {
            summaries.push(summary);
        }
    }
    sort_summaries(&mut summaries);
    Ok(summaries)
}

pub fn list_ancestors(
    graph: &IdeaGraph,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError> {
    traverse(graph, node_id, options, TraversalDirection::Ancestors)
}

pub fn list_descendants(
    graph: &IdeaGraph,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, GraphError> {
    traverse(graph, node_id, options, TraversalDirection::Descendants)
}

pub fn rebuild_index(graph: &IdeaGraph) -> Result<GraphIndex, GraphError> {
    Ok(graph_index(&scan_graph(graph)?))
}

fn require_parent_node<'a>(
    scan: &'a ScanResult,
    parent_id: &str,
) -> Result<&'a ScannedNode, GraphError> {
    match canonical_node(scan, parent_id) {
        Ok(node) => Ok(node),
        Err(GraphError::NodeNotFound(_)) => Err(GraphError::ParentNotFound(parent_id.to_string())),
        Err(error) => Err(error),
    }
}

fn require_child_node<'a>(
    scan: &'a ScanResult,
    child_id: &str,
) -> Result<&'a ScannedNode, GraphError> {
    match canonical_node(scan, child_id) {
        Ok(node) => Ok(node),
        Err(GraphError::NodeNotFound(_)) => Err(GraphError::ChildNotFound(child_id.to_string())),
        Err(error) => Err(error),
    }
}

fn create_node_in_container(
    graph: &IdeaGraph,
    container: &Path,
    node: NewNode,
) -> Result<NodeId, GraphError> {
    validate_new_node(&node, &graph.root)?;

    let scan = scan_graph(graph)?;
    ensure_real_dir_inside(&graph.root, container)?;
    let id = loop {
        let id = generate_node_id();
        if !scan.nodes.contains_key(&id) {
            break id;
        }
    };
    let node_dir = unique_node_path(container, &node.title, &id)?;
    let created_at_unix = now_unix();

    fs::create_dir_all(&node_dir)?;
    fs::create_dir_all(children_path(&node_dir))?;
    write_node_markdown(
        &node_dir,
        &id,
        &node.kind,
        &node.title,
        created_at_unix,
        created_at_unix,
        &node.content,
    )?;

    Ok(id)
}

fn write_node_markdown(
    node_dir: &Path,
    id: &str,
    kind: &NodeKind,
    title: &str,
    created_at_unix: u64,
    updated_at_unix: u64,
    content: &NodeContent,
) -> Result<(), GraphError> {
    let markdown = render_node_markdown(id, kind, title, created_at_unix, updated_at_unix, content);
    let node_file = node_file_path(node_dir);
    write_file_atomically(node_dir, &node_file, &markdown)
}

fn node_from_scanned(node: &ScannedNode) -> Result<Node, GraphError> {
    Ok(Node {
        id: node.id.clone(),
        kind: node.kind.clone(),
        title: node.title.clone(),
        content: node.content.clone(),
        created_at_unix: node.created_at_unix,
        updated_at_unix: node.updated_at_unix,
        canonical_path: canonical_path(node)
            .ok_or_else(|| GraphError::NodeNotFound(node.id.clone()))?
            .to_path_buf(),
        alias_paths: node.alias_paths.clone(),
    })
}

fn validate_new_node(node: &NewNode, path: &Path) -> Result<(), GraphError> {
    validate_title(&node.title)?;
    validate_content_matches(&node.kind, &node.content, path)?;
    validate_content_values(&node.content, path)
}

fn validate_content_matches(
    kind: &NodeKind,
    content: &NodeContent,
    path: &Path,
) -> Result<(), GraphError> {
    match (kind, content) {
        (NodeKind::Statement, NodeContent::Statement { .. })
        | (NodeKind::QuestionAnswer, NodeContent::QuestionAnswer { .. }) => Ok(()),
        _ => Err(GraphError::InvalidMarkdown {
            path: path.to_path_buf(),
            reason: "node kind does not match node content".to_string(),
        }),
    }
}

fn validate_content_values(content: &NodeContent, path: &Path) -> Result<(), GraphError> {
    match content {
        NodeContent::Statement { .. } => Ok(()),
        NodeContent::QuestionAnswer { question, .. } if question.trim().is_empty() => {
            Err(GraphError::InvalidMarkdown {
                path: path.to_path_buf(),
                reason: "question must not be empty".to_string(),
            })
        }
        NodeContent::QuestionAnswer { .. } => Ok(()),
    }
}

fn reaches(scan: &ScanResult, start_id: &str, target_id: &str) -> bool {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([start_id.to_string()]);

    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        for edge in child_edges(scan, &id) {
            if edge.child_id == target_id {
                return true;
            }
            queue.push_back(edge.child_id.clone());
        }
    }

    false
}

fn compute_delete_set(scan: &ScanResult, target_id: &str) -> BTreeSet<String> {
    let mut delete_set = BTreeSet::from([target_id.to_string()]);
    let mut changed = true;

    while changed {
        changed = false;
        for edge in &scan.edges {
            if !delete_set.contains(&edge.parent_id) || delete_set.contains(&edge.child_id) {
                continue;
            }
            let has_root_entry = scan
                .root_entries
                .iter()
                .any(|entry| entry.id == edge.child_id);
            if has_root_entry {
                continue;
            }
            let parents = parent_edges(scan, &edge.child_id);
            if parents
                .iter()
                .all(|parent_edge| delete_set.contains(&parent_edge.parent_id))
            {
                delete_set.insert(edge.child_id.clone());
                changed = true;
            }
        }
    }

    delete_set
}

fn perform_delete_set(
    graph: &IdeaGraph,
    scan: &ScanResult,
    delete_set: &BTreeSet<String>,
) -> Result<(), GraphError> {
    let delete_paths = delete_set
        .iter()
        .filter_map(|id| {
            scan.nodes
                .get(id)
                .and_then(canonical_path)
                .map(Path::to_path_buf)
        })
        .collect::<Vec<_>>();

    for (id, alias_path) in promotion_targets_for_delete(scan, delete_set, &delete_paths) {
        let node = canonical_node(scan, &id)?;
        promote_to_alias(graph, node, &alias_path)?;
    }

    for id in delete_set {
        if let Some(node) = scan.nodes.get(id) {
            for alias_path in &node.alias_paths {
                safe_remove_file(&graph.root, alias_path)?;
            }
        }
    }

    let mut top_level_paths = delete_paths;
    top_level_paths.sort();
    let mut retained = Vec::<PathBuf>::new();
    for path in top_level_paths {
        if !retained.iter().any(|parent| path.starts_with(parent)) {
            retained.push(path);
        }
    }
    for path in retained {
        if path.exists() {
            safe_remove_dir_all(&graph.root, &path)?;
        }
    }

    Ok(())
}

fn promotion_targets_for_delete(
    scan: &ScanResult,
    delete_set: &BTreeSet<String>,
    delete_paths: &[PathBuf],
) -> Vec<(String, PathBuf)> {
    let mut candidates = BTreeMap::<String, PathBuf>::new();

    for edge in &scan.edges {
        if !delete_set.contains(&edge.parent_id) || delete_set.contains(&edge.child_id) {
            continue;
        }
        let Some(node) = scan.nodes.get(&edge.child_id) else {
            continue;
        };
        let Some(canonical_path) = canonical_path(node) else {
            continue;
        };
        if !is_path_inside_any(canonical_path, delete_paths) {
            continue;
        }
        let Some(alias_path) = node
            .alias_paths
            .iter()
            .filter(|path| !is_path_inside_any(path, delete_paths))
            .min_by_key(|path| path_sort_key(path))
            .cloned()
        else {
            continue;
        };
        candidates
            .entry(edge.child_id.clone())
            .or_insert(alias_path);
    }

    let mut targets = candidates.into_iter().collect::<Vec<_>>();
    targets.sort_by_key(|(id, _)| {
        match scan
            .nodes
            .get(id)
            .and_then(canonical_path)
            .map(|path| path.components().count())
        {
            Some(depth) => depth,
            None => usize::MAX,
        }
    });

    let mut selected = Vec::<(String, PathBuf)>::new();
    let mut selected_paths = Vec::<PathBuf>::new();
    for (id, alias_path) in targets {
        let Some(canonical_path) = scan.nodes.get(&id).and_then(canonical_path) else {
            continue;
        };
        if selected_paths
            .iter()
            .any(|selected_path| canonical_path.starts_with(selected_path))
        {
            continue;
        }
        selected_paths.push(canonical_path.to_path_buf());
        selected.push((id, alias_path));
    }

    selected
}

fn promote_to_alias(
    graph: &IdeaGraph,
    node: &ScannedNode,
    new_canonical_path: &Path,
) -> Result<(), GraphError> {
    let old_canonical_path =
        canonical_path(node).ok_or_else(|| GraphError::NodeNotFound(node.id.clone()))?;
    safe_remove_file(&graph.root, new_canonical_path)?;
    safe_rename(&graph.root, old_canonical_path, new_canonical_path)?;

    for alias_path in &node.alias_paths {
        if alias_path == new_canonical_path {
            continue;
        }
        rewrite_existing_alias(graph, alias_path, new_canonical_path)?;
    }

    Ok(())
}

fn move_node_to_roots(graph: &IdeaGraph, node: &ScannedNode) -> Result<(), GraphError> {
    let old_canonical_path =
        canonical_path(node).ok_or_else(|| GraphError::NodeNotFound(node.id.clone()))?;
    if old_canonical_path.parent() == Some(roots_path(&graph.root).as_path()) {
        return Ok(());
    }
    let target = unique_node_path(&roots_path(&graph.root), &node.title, &node.id)?;
    safe_rename(&graph.root, old_canonical_path, &target)?;

    for alias_path in &node.alias_paths {
        rewrite_existing_alias(graph, alias_path, &target)?;
    }

    Ok(())
}

fn rewrite_existing_alias(
    graph: &IdeaGraph,
    alias_path: &Path,
    target_path: &Path,
) -> Result<(), GraphError> {
    match fs::symlink_metadata(alias_path) {
        Ok(_) => {
            safe_remove_file(&graph.root, alias_path)?;
            create_relative_dir_symlink(&graph.root, target_path, alias_path)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(GraphError::Io(error)),
    }
}

#[derive(Debug, Clone, Copy)]
enum TraversalDirection {
    Ancestors,
    Descendants,
}

fn traverse(
    graph: &IdeaGraph,
    node_id: &str,
    options: TraversalOptions,
    direction: TraversalDirection,
) -> Result<Vec<NodeSummary>, GraphError> {
    validate_node_id(node_id)?;
    let scan = scan_graph(graph)?;
    canonical_node(&scan, node_id)?;

    if options.max_depth == Some(0) {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let mut visited = BTreeSet::from([node_id.to_string()]);
    let mut queue = VecDeque::new();
    for neighbor in sorted_neighbors(&scan, node_id, direction)? {
        queue.push_back((neighbor.id.clone(), neighbor, 1usize));
    }

    while let Some((id, summary, depth)) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        results.push(summary);

        if options
            .max_depth
            .is_some_and(|max_depth| depth >= max_depth)
        {
            continue;
        }
        for neighbor in sorted_neighbors(&scan, &id, direction)? {
            if !visited.contains(&neighbor.id) {
                queue.push_back((neighbor.id.clone(), neighbor, depth + 1));
            }
        }
    }

    Ok(results)
}

fn sorted_neighbors(
    scan: &ScanResult,
    node_id: &str,
    direction: TraversalDirection,
) -> Result<Vec<NodeSummary>, GraphError> {
    let mut summaries = match direction {
        TraversalDirection::Ancestors => parent_edges(scan, node_id)
            .into_iter()
            .filter_map(|edge| summary_for(scan, &edge.parent_id, false))
            .collect::<Vec<_>>(),
        TraversalDirection::Descendants => {
            if let Some(path) = broken_symlink_under_parent(scan, node_id) {
                return Err(GraphError::BrokenSymlink(path));
            }
            let mut summaries = Vec::new();
            for edge in child_edges(scan, node_id) {
                let node = canonical_node(scan, &edge.child_id)?;
                if let Some(summary) = node_summary(node, edge.is_symlink) {
                    summaries.push(summary);
                }
            }
            summaries
        }
    };
    sort_summaries(&mut summaries);
    Ok(summaries)
}

fn broken_symlink_for_id(scan: &ScanResult, node_id: &str) -> Option<PathBuf> {
    scan.problems.iter().find_map(|problem| match problem {
        crate::model::GraphProblem::BrokenSymlink { path }
            if problem_path_node_id(path).as_deref() == Some(node_id) =>
        {
            Some(path.clone())
        }
        _ => None,
    })
}

fn problem_path_node_id(path: &Path) -> Option<String> {
    if let Some(id) = crate::scan::node_id_from_entry_path(path) {
        return Some(id);
    }
    if path.file_name().and_then(|name| name.to_str()) == Some(NODE_FILE) {
        return path.parent().and_then(crate::scan::node_id_from_entry_path);
    }
    None
}
