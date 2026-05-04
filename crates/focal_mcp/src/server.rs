use focal_core::Backend;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Implementation, InitializeResult,
    ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, ProtocolVersion,
    ReadResourceRequestParams, ReadResourceResult, ServerCapabilities, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt};

use crate::Error;
use crate::config::{BackendConfig, OpenMode, ServerConfig, TransportConfig};
use crate::resources;
use crate::tools;

#[derive(Debug, Clone)]
enum RuntimeBackend {
    Fs {
        root: std::path::PathBuf,
    },
    #[cfg(feature = "sqlite")]
    Sqlite {
        database_path: std::path::PathBuf,
        graph_name: String,
    },
}

#[derive(Debug, Clone)]
pub struct FocalMcpServer {
    backend: RuntimeBackend,
}

impl FocalMcpServer {
    pub fn from_config(config: ServerConfig) -> Result<Self, Error> {
        match config.transport {
            TransportConfig::Stdio => {}
        }

        match config.backend {
            BackendConfig::Fs { root, mode } => {
                match mode {
                    OpenMode::Init => {
                        let _backend = focal_core::init_fs(&root)?;
                    }
                    OpenMode::Open => {
                        let _backend = focal_core::open_fs(&root)?;
                    }
                }
                Ok(Self {
                    backend: RuntimeBackend::Fs { root },
                })
            }
            #[cfg(feature = "sqlite")]
            BackendConfig::Sqlite {
                database_path,
                graph_name,
                mode,
            } => {
                let mut connection = focal_core::open_database(&database_path)?;
                match mode {
                    OpenMode::Init => {
                        let _backend = focal_core::init_sqlite(&mut connection, &graph_name)?;
                    }
                    OpenMode::Open => {
                        let _backend = focal_core::open_sqlite(&mut connection, &graph_name)?;
                    }
                }
                Ok(Self {
                    backend: RuntimeBackend::Sqlite {
                        database_path,
                        graph_name,
                    },
                })
            }
        }
    }

    pub fn tools() -> Vec<Tool> {
        tools::tool_definitions()
    }

    pub fn call_tool_by_name(
        &self,
        name: &str,
        arguments: Option<rmcp::model::JsonObject>,
    ) -> Result<CallToolResult, McpError> {
        tools::call_tool(self, name, arguments)
    }

    pub fn list_resources_result() -> ListResourcesResult {
        resources::list_resources()
    }

    pub fn list_resource_templates_result() -> ListResourceTemplatesResult {
        resources::list_resource_templates()
    }

    pub fn read_resource_uri(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        resources::read_resource(self, uri)
    }

    pub(crate) fn with_backend<T, F>(&self, operation: F) -> Result<T, Error>
    where
        F: for<'backend> FnOnce(&mut Backend<'backend>) -> Result<T, focal_core::Error>,
    {
        match &self.backend {
            RuntimeBackend::Fs { root } => {
                let mut backend = focal_core::open_fs(root)?;
                operation(&mut backend).map_err(Error::Core)
            }
            #[cfg(feature = "sqlite")]
            RuntimeBackend::Sqlite {
                database_path,
                graph_name,
            } => {
                let mut connection = focal_core::open_database(database_path)?;
                let mut backend = focal_core::open_sqlite(&mut connection, graph_name)?;
                operation(&mut backend).map_err(Error::Core)
            }
        }
    }
}

impl ServerHandler for FocalMcpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_server_info(Implementation::new("focal-mcp", env!("CARGO_PKG_VERSION")).with_title("Focal MCP"))
        .with_instructions("Use focal_ tools and focal:// resources to inspect and modify the configured Focal idea graph.")
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(Ok(ListToolsResult::with_all_items(Self::tools())))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        Self::tools().into_iter().find(|tool| tool.name == name)
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(self.call_tool_by_name(request.name.as_ref(), request.arguments))
    }

    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(Ok(Self::list_resources_result()))
    }

    fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, McpError>>
    + rmcp::service::MaybeSendFuture
    + '_ {
        std::future::ready(Ok(Self::list_resource_templates_result()))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(self.read_resource_uri(&request.uri))
    }
}

pub fn run_stdio(config: ServerConfig) -> Result<(), Error> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(Error::InvalidConfig(
            "run_stdio cannot be called from inside an active Tokio runtime".to_string(),
        ));
    }

    let server = FocalMcpServer::from_config(config)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|source| {
            Error::InvalidConfig(format!(
                "failed to create Tokio runtime for stdio: {source}"
            ))
        })?;

    runtime.block_on(async move {
        let running = server.serve(rmcp::transport::stdio()).await?;
        let _reason = running.waiting().await?;
        Ok(())
    })
}
