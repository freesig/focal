use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GraphError;
use crate::fs_utils::{
    children_path, has_node_dir_suffix, node_file_path, node_id_from_dir_name,
    resolve_symlink_path, roots_path, validate_node_id, validate_title,
};
use crate::markdown::{ParsedMarkdown, parse_node_markdown};
use crate::model::{
    GraphEdge, GraphIndex, GraphProblem, IdeaGraph, NodeContent, NodeKind, NodeSummary,
};

#[derive(Debug, Clone)]
pub(crate) struct ScannedNode {
    pub id: String,
    pub kind: NodeKind,
    pub title: String,
    pub content: NodeContent,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
    pub canonical_paths: Vec<PathBuf>,
    pub alias_paths: Vec<PathBuf>,
}

pub(crate) fn canonical_path(node: &ScannedNode) -> Option<&Path> {
    node.canonical_paths.first().map(PathBuf::as_path)
}

pub(crate) fn node_summary(node: &ScannedNode, is_alias: bool) -> Option<NodeSummary> {
    Some(NodeSummary {
        id: node.id.clone(),
        kind: node.kind.clone(),
        title: node.title.clone(),
        canonical_path: canonical_path(node)?.to_path_buf(),
        is_alias,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct RootEntry {
    pub id: String,
    pub is_symlink: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct BrokenSymlinkEntry {
    pub parent_id: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ScanResult {
    pub nodes: BTreeMap<String, ScannedNode>,
    pub edges: Vec<GraphEdge>,
    pub root_entries: Vec<RootEntry>,
    pub broken_symlinks: Vec<BrokenSymlinkEntry>,
    pub problems: Vec<GraphProblem>,
}

pub(crate) fn canonical_node<'a>(
    scan: &'a ScanResult,
    id: &str,
) -> Result<&'a ScannedNode, GraphError> {
    validate_node_id(id)?;
    let node = scan
        .nodes
        .get(id)
        .ok_or_else(|| GraphError::NodeNotFound(id.to_string()))?;
    if node.canonical_paths.len() > 1 {
        return Err(GraphError::DuplicateCanonicalNode {
            id: id.to_string(),
            paths: node.canonical_paths.clone(),
        });
    }
    if canonical_path(node).is_none() {
        let path = match node.alias_paths.first() {
            Some(path) => path.clone(),
            None => PathBuf::from(id),
        };
        return Err(GraphError::BrokenSymlink(path));
    }
    Ok(node)
}

pub(crate) fn child_edges<'a>(scan: &'a ScanResult, parent_id: &str) -> Vec<&'a GraphEdge> {
    scan.edges
        .iter()
        .filter(|edge| edge.parent_id == parent_id)
        .collect()
}

pub(crate) fn parent_edges<'a>(scan: &'a ScanResult, child_id: &str) -> Vec<&'a GraphEdge> {
    scan.edges
        .iter()
        .filter(|edge| edge.child_id == child_id)
        .collect()
}

pub(crate) fn find_edge<'a>(
    scan: &'a ScanResult,
    parent_id: &str,
    child_id: &str,
) -> Option<&'a GraphEdge> {
    scan.edges
        .iter()
        .find(|edge| edge.parent_id == parent_id && edge.child_id == child_id)
}

pub(crate) fn broken_symlink_under_parent(scan: &ScanResult, parent_id: &str) -> Option<PathBuf> {
    scan.broken_symlinks
        .iter()
        .find(|entry| entry.parent_id.as_deref() == Some(parent_id))
        .map(|entry| entry.path.clone())
}

pub(crate) fn graph_index(scan: &ScanResult) -> GraphIndex {
    let mut nodes = scan
        .nodes
        .values()
        .filter_map(|node| node_summary(node, false))
        .collect::<Vec<_>>();
    sort_summaries(&mut nodes);

    let mut edges = scan.edges.clone();
    edges.sort_by(|left, right| {
        left.parent_id
            .cmp(&right.parent_id)
            .then(left.child_id.cmp(&right.child_id))
            .then(left.path.cmp(&right.path))
    });

    GraphIndex {
        nodes,
        edges,
        problems: scan.problems.clone(),
    }
}

pub(crate) fn scan_graph(graph: &IdeaGraph) -> Result<ScanResult, GraphError> {
    let roots = roots_path(&graph.root);
    if !roots.is_dir() {
        return Err(GraphError::InvalidGraphRoot(format!(
            "{} does not contain roots/",
            graph.root.display()
        )));
    }

    let mut scanner = Scanner {
        graph,
        result: ScanResult {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            root_entries: Vec::new(),
            broken_symlinks: Vec::new(),
            problems: Vec::new(),
        },
    };
    scan_container(&mut scanner, &roots, None)?;
    detect_cycles(&mut scanner.result);
    Ok(scanner.result)
}

pub(crate) fn sort_summaries(summaries: &mut [NodeSummary]) {
    summaries.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then(left.id.cmp(&right.id))
            .then(left.canonical_path.cmp(&right.canonical_path))
    });
}

struct Scanner<'a> {
    graph: &'a IdeaGraph,
    result: ScanResult,
}

fn scan_container(
    scanner: &mut Scanner<'_>,
    container: &Path,
    parent_id: Option<&str>,
) -> Result<(), GraphError> {
    let mut entries = match fs::read_dir(container) {
        Ok(entries) => entries.collect::<Result<Vec<_>, _>>()?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(GraphError::Io(error)),
    };
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            scan_symlink_entry(scanner, &path, parent_id)?;
        } else if file_type.is_dir() {
            scan_real_node_dir(scanner, &path, parent_id)?;
        }
    }

    Ok(())
}

fn scan_real_node_dir(
    scanner: &mut Scanner<'_>,
    node_dir: &Path,
    parent_id: Option<&str>,
) -> Result<(), GraphError> {
    let Some(parsed) = parse_real_node_dir(&mut scanner.result, node_dir)? else {
        return Ok(());
    };
    let id = parsed.id.clone();
    add_canonical(&mut scanner.result, node_dir.to_path_buf(), parsed);

    if let Some(parent_id) = parent_id {
        scanner.result.edges.push(GraphEdge {
            parent_id: parent_id.to_string(),
            child_id: id.clone(),
            path: node_dir.to_path_buf(),
            is_symlink: false,
        });
    } else {
        scanner.result.root_entries.push(RootEntry {
            id: id.clone(),
            is_symlink: false,
        });
    }

    let children = children_path(node_dir);
    if children.is_dir() {
        scan_container(scanner, &children, Some(&id))?;
    }

    Ok(())
}

fn scan_symlink_entry(
    scanner: &mut Scanner<'_>,
    link_path: &Path,
    parent_id: Option<&str>,
) -> Result<(), GraphError> {
    let target = match resolve_symlink_path(link_path).and_then(|path| {
        path.canonicalize()
            .map_err(|_| GraphError::BrokenSymlink(link_path.to_path_buf()))
    }) {
        Ok(target) => target,
        Err(_) => {
            add_broken_symlink(&mut scanner.result, link_path, parent_id);
            return Ok(());
        }
    };

    if !target.starts_with(roots_path(&scanner.graph.root)) {
        add_broken_symlink(&mut scanner.result, link_path, parent_id);
        return Ok(());
    }

    let Some(parsed) = parse_real_node_dir(&mut scanner.result, &target)? else {
        add_broken_symlink(&mut scanner.result, link_path, parent_id);
        return Ok(());
    };
    let id = parsed.id.clone();
    add_alias(&mut scanner.result, &id, link_path.to_path_buf(), parsed);

    if let Some(parent_id) = parent_id {
        scanner.result.edges.push(GraphEdge {
            parent_id: parent_id.to_string(),
            child_id: id.clone(),
            path: link_path.to_path_buf(),
            is_symlink: true,
        });
    } else {
        scanner.result.root_entries.push(RootEntry {
            id,
            is_symlink: true,
        });
    }

    Ok(())
}

fn parse_real_node_dir(
    result: &mut ScanResult,
    node_dir: &Path,
) -> Result<Option<ParsedMarkdown>, GraphError> {
    let node_file = node_file_path(node_dir);
    if !node_file.is_file() {
        result.problems.push(GraphProblem::MissingNodeMarkdown {
            path: node_dir.to_path_buf(),
        });
        return Ok(None);
    }

    let source = fs::read_to_string(&node_file)?;
    let parsed = match parse_node_markdown(&node_file, &source) {
        Ok(parsed) => parsed,
        Err(reason) => {
            result.problems.push(GraphProblem::InvalidMarkdown {
                path: node_file,
                reason,
            });
            return Ok(None);
        }
    };

    if let Err(error) = validate_node_id(&parsed.id) {
        result.problems.push(GraphProblem::InvalidMarkdown {
            path: node_file,
            reason: error.to_string(),
        });
        return Ok(None);
    }

    if let Err(error) = validate_title(&parsed.title) {
        result.problems.push(GraphProblem::InvalidMarkdown {
            path: node_file,
            reason: error.to_string(),
        });
        return Ok(None);
    }

    if !has_node_dir_suffix(node_dir, &parsed.id) {
        result.problems.push(GraphProblem::InvalidMarkdown {
            path: node_file,
            reason: "node directory id suffix does not match metadata id".to_string(),
        });
        return Ok(None);
    }

    let children = children_path(node_dir);
    if !children.is_dir() {
        result
            .problems
            .push(GraphProblem::MissingChildrenDirectory {
                path: node_dir.to_path_buf(),
            });
    }

    Ok(Some(parsed))
}

fn add_broken_symlink(result: &mut ScanResult, link_path: &Path, parent_id: Option<&str>) {
    let path = link_path.to_path_buf();
    let parent_id = parent_id.map(str::to_string);
    if result
        .broken_symlinks
        .iter()
        .any(|entry| entry.parent_id == parent_id && entry.path == path)
    {
        return;
    }
    result.broken_symlinks.push(BrokenSymlinkEntry {
        parent_id,
        path: path.clone(),
    });
    result.problems.push(GraphProblem::BrokenSymlink { path });
}

fn add_canonical(result: &mut ScanResult, path: PathBuf, parsed: ParsedMarkdown) {
    let node = result
        .nodes
        .entry(parsed.id.clone())
        .or_insert_with(|| scanned_node_from_parsed(&parsed));

    if !node.canonical_paths.contains(&path) {
        node.canonical_paths.push(path);
        node.canonical_paths.sort();
    }
    if node.canonical_paths.len() > 1 {
        result.problems.push(GraphProblem::DuplicateCanonicalNode {
            id: node.id.clone(),
            paths: node.canonical_paths.clone(),
        });
    }
}

fn add_alias(result: &mut ScanResult, id: &str, path: PathBuf, parsed: ParsedMarkdown) {
    let node = result
        .nodes
        .entry(id.to_string())
        .or_insert_with(|| scanned_node_from_parsed(&parsed));

    if !node.alias_paths.contains(&path) {
        node.alias_paths.push(path);
        node.alias_paths.sort();
    }
}

fn scanned_node_from_parsed(parsed: &ParsedMarkdown) -> ScannedNode {
    ScannedNode {
        id: parsed.id.clone(),
        kind: parsed.kind.clone(),
        title: parsed.title.clone(),
        content: parsed.content.clone(),
        created_at_unix: parsed.created_at_unix,
        updated_at_unix: parsed.updated_at_unix,
        canonical_paths: Vec::new(),
        alias_paths: Vec::new(),
    }
}

fn detect_cycles(result: &mut ScanResult) {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in &result.edges {
        adjacency
            .entry(edge.parent_id.as_str())
            .or_default()
            .push(edge.child_id.as_str());
    }
    for children in adjacency.values_mut() {
        children.sort_unstable();
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut cycles = BTreeSet::new();
    for id in result.nodes.keys() {
        detect_cycle_from(id, &adjacency, &mut visiting, &mut visited, &mut cycles);
    }

    result.problems.extend(
        cycles
            .into_iter()
            .map(|node_id| GraphProblem::CycleDetected { node_id }),
    );
}

fn detect_cycle_from<'a>(
    id: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
    cycles: &mut BTreeSet<String>,
) {
    if visited.contains(id) {
        return;
    }
    if !visiting.insert(id) {
        cycles.insert(id.to_string());
        return;
    }

    if let Some(children) = adjacency.get(id) {
        for child in children {
            detect_cycle_from(child, adjacency, visiting, visited, cycles);
        }
    }

    visiting.remove(id);
    visited.insert(id);
}

pub(crate) fn summary_for(scan: &ScanResult, id: &str, is_alias: bool) -> Option<NodeSummary> {
    node_summary(scan.nodes.get(id)?, is_alias)
}

pub(crate) fn node_id_from_entry_path(path: &Path) -> Option<String> {
    node_id_from_dir_name(path)
}
