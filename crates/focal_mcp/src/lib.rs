//! MCP adapter for the Focal idea graph API.
//!
//! This crate owns no graph storage format. It exposes a configured
//! `focal-core` backend through Model Context Protocol tools and resources.

mod config;
mod error;
mod resources;
mod server;
mod tools;

pub use config::{
    BackendConfig, DEFAULT_CONFIG_RELATIVE_PATH, OpenMode, ServerConfig, TransportConfig,
    default_config_path, load_config, save_config,
};
pub use error::Error;
pub use server::{FocalMcpServer, run_stdio};
