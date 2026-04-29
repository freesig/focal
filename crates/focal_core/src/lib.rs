//! Unified idea graph API over the supported Focal storage backends.
//!
//! This crate is a dispatch facade. It owns no graph storage format and does
//! not define behavior beyond forwarding calls to the selected backend.

use std::fmt;
#[cfg(not(feature = "sqlite"))]
use std::marker::PhantomData;
use std::path::Path;

#[cfg(feature = "sqlite")]
use rusqlite::Connection;

pub use focal_types::{
    DeleteMode, GraphEdge, GraphError, GraphIndex, GraphProblem, NewNode, Node, NodeContent,
    NodeId, NodeKind, NodePatch, NodeSummary, OrphanPolicy, TraversalOptions,
};

pub enum Backend<'conn> {
    Fs(focal_fs::IdeaGraph),
    #[cfg(feature = "sqlite")]
    Sqlite(focal_sqlite::IdeaGraph<'conn>),
    #[cfg(not(feature = "sqlite"))]
    Sqlite(EmptySqlite<'conn>),
}

#[cfg(not(feature = "sqlite"))]
#[derive(Debug)]
pub struct EmptySqlite<'conn> {
    pub graph_name: String,
    pub _lifetime: PhantomData<&'conn mut ()>,
}

#[cfg(not(feature = "sqlite"))]
impl<'conn> EmptySqlite<'conn> {
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }
}

#[derive(Debug)]
pub enum Error {
    Fs(focal_fs::Error),
    #[cfg(feature = "sqlite")]
    Sqlite(focal_sqlite::Error),
    #[cfg(not(feature = "sqlite"))]
    Sqlite(DisabledSqliteError),
}

#[cfg(not(feature = "sqlite"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisabledSqliteError {
    pub graph_name: String,
}

impl From<focal_fs::Error> for Error {
    fn from(error: focal_fs::Error) -> Self {
        Self::Fs(error)
    }
}

#[cfg(feature = "sqlite")]
impl From<focal_sqlite::Error> for Error {
    fn from(error: focal_sqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fs(error) => write!(formatter, "filesystem backend error: {error}"),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(error) => write!(formatter, "sqlite backend error: {error}"),
            #[cfg(not(feature = "sqlite"))]
            Self::Sqlite(error) => write!(formatter, "sqlite backend error: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fs(error) => Some(error),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(error) => Some(error),
            #[cfg(not(feature = "sqlite"))]
            Self::Sqlite(error) => Some(error),
        }
    }
}

#[cfg(not(feature = "sqlite"))]
impl fmt::Display for DisabledSqliteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "sqlite support is disabled for graph `{}`",
            self.graph_name
        )
    }
}

#[cfg(not(feature = "sqlite"))]
impl std::error::Error for DisabledSqliteError {}

pub fn init_fs(root: impl AsRef<Path>) -> Result<Backend<'static>, Error> {
    focal_fs::init_graph(root)
        .map(Backend::Fs)
        .map_err(fs_error)
}

pub fn open_fs(root: impl AsRef<Path>) -> Result<Backend<'static>, Error> {
    focal_fs::open_graph(root)
        .map(Backend::Fs)
        .map_err(fs_error)
}

#[cfg(feature = "sqlite")]
pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, Error> {
    focal_sqlite::open_database(path).map_err(sqlite_error)
}

#[cfg(feature = "sqlite")]
pub fn init_sqlite<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<Backend<'conn>, Error> {
    focal_sqlite::init_graph(connection, graph_name)
        .map(Backend::Sqlite)
        .map_err(sqlite_error)
}

#[cfg(feature = "sqlite")]
pub fn open_sqlite<'conn>(
    connection: &'conn mut Connection,
    graph_name: &str,
) -> Result<Backend<'conn>, Error> {
    focal_sqlite::open_graph(connection, graph_name)
        .map(Backend::Sqlite)
        .map_err(sqlite_error)
}

#[cfg(not(feature = "sqlite"))]
pub fn disabled_sqlite(graph_name: impl Into<String>) -> Backend<'static> {
    Backend::Sqlite(EmptySqlite {
        graph_name: graph_name.into(),
        _lifetime: PhantomData,
    })
}

pub fn add_root_node(backend: &mut Backend<'_>, node: NewNode) -> Result<NodeId, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::add_root_node(graph, node).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => focal_sqlite::add_root_node(graph, node).map_err(sqlite_error),
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn add_child_node(
    backend: &mut Backend<'_>,
    parent_id: &str,
    node: NewNode,
) -> Result<NodeId, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::add_child_node(graph, parent_id, node).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => {
            focal_sqlite::add_child_node(graph, parent_id, node).map_err(sqlite_error)
        }
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn read_node(backend: &Backend<'_>, node_id: &str) -> Result<Node, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::read_node(graph, node_id).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => focal_sqlite::read_node(graph, node_id).map_err(sqlite_error),
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn update_node(
    backend: &mut Backend<'_>,
    node_id: &str,
    patch: NodePatch,
) -> Result<Node, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::update_node(graph, node_id, patch).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => {
            focal_sqlite::update_node(graph, node_id, patch).map_err(sqlite_error)
        }
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn delete_node(
    backend: &mut Backend<'_>,
    node_id: &str,
    mode: DeleteMode,
) -> Result<(), Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::delete_node(graph, node_id, mode).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => {
            focal_sqlite::delete_node(graph, node_id, mode).map_err(sqlite_error)
        }
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn link_existing_node(
    backend: &mut Backend<'_>,
    parent_id: &str,
    child_id: &str,
) -> Result<(), Error> {
    match backend {
        Backend::Fs(graph) => {
            focal_fs::link_existing_node(graph, parent_id, child_id).map_err(fs_error)
        }
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => {
            focal_sqlite::link_existing_node(graph, parent_id, child_id).map_err(sqlite_error)
        }
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn unlink_child(
    backend: &mut Backend<'_>,
    parent_id: &str,
    child_id: &str,
    orphan_policy: OrphanPolicy,
) -> Result<(), Error> {
    match backend {
        Backend::Fs(graph) => {
            focal_fs::unlink_child(graph, parent_id, child_id, orphan_policy).map_err(fs_error)
        }
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => {
            focal_sqlite::unlink_child(graph, parent_id, child_id, orphan_policy)
                .map_err(sqlite_error)
        }
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn list_roots(backend: &Backend<'_>) -> Result<Vec<NodeSummary>, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::list_roots(graph).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => focal_sqlite::list_roots(graph).map_err(sqlite_error),
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn list_children(backend: &Backend<'_>, node_id: &str) -> Result<Vec<NodeSummary>, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::list_children(graph, node_id).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => focal_sqlite::list_children(graph, node_id).map_err(sqlite_error),
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn list_parents(backend: &Backend<'_>, node_id: &str) -> Result<Vec<NodeSummary>, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::list_parents(graph, node_id).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => focal_sqlite::list_parents(graph, node_id).map_err(sqlite_error),
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn list_ancestors(
    backend: &Backend<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::list_ancestors(graph, node_id, options).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => {
            focal_sqlite::list_ancestors(graph, node_id, options).map_err(sqlite_error)
        }
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn list_descendants(
    backend: &Backend<'_>,
    node_id: &str,
    options: TraversalOptions,
) -> Result<Vec<NodeSummary>, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::list_descendants(graph, node_id, options).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => {
            focal_sqlite::list_descendants(graph, node_id, options).map_err(sqlite_error)
        }
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

pub fn rebuild_index(backend: &Backend<'_>) -> Result<GraphIndex, Error> {
    match backend {
        Backend::Fs(graph) => focal_fs::rebuild_index(graph).map_err(fs_error),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(graph) => focal_sqlite::rebuild_index(graph).map_err(sqlite_error),
        #[cfg(not(feature = "sqlite"))]
        Backend::Sqlite(sqlite) => Err(disabled_sqlite_error(sqlite)),
    }
}

fn fs_error(error: focal_fs::GraphError) -> Error {
    Error::Fs(focal_fs::Error::from(error))
}

#[cfg(feature = "sqlite")]
fn sqlite_error(error: focal_sqlite::GraphError) -> Error {
    Error::Sqlite(focal_sqlite::Error::from(error))
}

#[cfg(not(feature = "sqlite"))]
fn disabled_sqlite_error(sqlite: &EmptySqlite<'_>) -> Error {
    Error::Sqlite(DisabledSqliteError {
        graph_name: sqlite.graph_name.clone(),
    })
}
