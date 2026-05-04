use std::sync::Arc;

use focal_core::{
    ContextDocumentPatch, DeleteMode, NewContextDocument, NewNode, NodeContent, NodeKind,
    NodePatch, OrphanPolicy, TraversalOptions,
};
use rmcp::ErrorData as McpError;
use rmcp::model::{
    CallToolResult, Content, ErrorCode, JsonObject, RawResource, Tool, ToolAnnotations,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::server::FocalMcpServer;

const TOOL_NAMES: [&str; 18] = [
    "focal_add_context_document",
    "focal_read_context_document",
    "focal_update_context_document",
    "focal_delete_context_document",
    "focal_list_context_documents",
    "focal_add_root_node",
    "focal_add_child_node",
    "focal_read_node",
    "focal_update_node",
    "focal_delete_node",
    "focal_link_existing_node",
    "focal_unlink_child",
    "focal_list_roots",
    "focal_list_children",
    "focal_list_parents",
    "focal_list_ancestors",
    "focal_list_descendants",
    "focal_rebuild_index",
];

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EmptyArgs {}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ContextIdArgs {
    context_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AddContextDocumentArgs {
    title: String,
    markdown: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UpdateContextDocumentArgs {
    context_id: String,
    title: Option<String>,
    markdown: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NodeIdArgs {
    node_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AddRootNodeArgs {
    kind: NodeKind,
    title: String,
    content: NodeContent,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AddChildNodeArgs {
    parent_id: String,
    kind: NodeKind,
    title: String,
    content: NodeContent,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UpdateNodeArgs {
    node_id: String,
    title: Option<String>,
    content: Option<NodeContent>,
    reviewed: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteNodeArgs {
    node_id: String,
    mode: DeleteMode,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ParentChildArgs {
    parent_id: String,
    child_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UnlinkChildArgs {
    parent_id: String,
    child_id: String,
    orphan_policy: OrphanPolicy,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TraversalArgs {
    node_id: String,
    max_depth: Option<usize>,
}

impl From<AddContextDocumentArgs> for NewContextDocument {
    fn from(args: AddContextDocumentArgs) -> Self {
        Self {
            title: args.title,
            markdown: args.markdown,
        }
    }
}

impl From<AddRootNodeArgs> for NewNode {
    fn from(args: AddRootNodeArgs) -> Self {
        Self {
            kind: args.kind,
            title: args.title,
            content: args.content,
        }
    }
}

impl From<AddChildNodeArgs> for NewNode {
    fn from(args: AddChildNodeArgs) -> Self {
        Self {
            kind: args.kind,
            title: args.title,
            content: args.content,
        }
    }
}

pub(crate) fn tool_definitions() -> Vec<Tool> {
    vec![
        tool::<AddContextDocumentArgs>(
            "focal_add_context_document",
            "Add a graph-level context document to the configured Focal graph.",
            additive(),
        ),
        tool::<ContextIdArgs>(
            "focal_read_context_document",
            "Read one context document from the configured Focal graph.",
            read_only(),
        ),
        tool::<UpdateContextDocumentArgs>(
            "focal_update_context_document",
            "Update a context document title or Markdown body.",
            destructive(),
        ),
        tool::<ContextIdArgs>(
            "focal_delete_context_document",
            "Delete one context document from the configured Focal graph.",
            destructive(),
        ),
        tool::<EmptyArgs>(
            "focal_list_context_documents",
            "List context documents in the configured Focal graph.",
            read_only(),
        ),
        tool::<AddRootNodeArgs>(
            "focal_add_root_node",
            "Add a root idea node to the configured Focal graph.",
            additive(),
        ),
        tool::<AddChildNodeArgs>(
            "focal_add_child_node",
            "Add a child idea node under an existing parent node.",
            additive(),
        ),
        tool::<NodeIdArgs>(
            "focal_read_node",
            "Read one idea node from the configured Focal graph.",
            read_only(),
        ),
        tool::<UpdateNodeArgs>(
            "focal_update_node",
            "Update a node title, content, or reviewed state.",
            destructive(),
        ),
        tool::<DeleteNodeArgs>(
            "focal_delete_node",
            "Delete a node from the configured Focal graph.",
            destructive(),
        ),
        tool::<ParentChildArgs>(
            "focal_link_existing_node",
            "Link an existing node under an additional parent.",
            ToolAnnotations::new()
                .read_only(false)
                .destructive(false)
                .idempotent(true)
                .open_world(false),
        ),
        tool::<UnlinkChildArgs>(
            "focal_unlink_child",
            "Unlink a child node from one parent using an explicit orphan policy.",
            destructive(),
        ),
        tool::<EmptyArgs>(
            "focal_list_roots",
            "List root nodes in the configured Focal graph.",
            read_only(),
        ),
        tool::<NodeIdArgs>(
            "focal_list_children",
            "List direct child nodes for one node.",
            read_only(),
        ),
        tool::<NodeIdArgs>(
            "focal_list_parents",
            "List direct parent nodes for one node.",
            read_only(),
        ),
        tool::<TraversalArgs>(
            "focal_list_ancestors",
            "List ancestor nodes breadth-first for one node.",
            read_only(),
        ),
        tool::<TraversalArgs>(
            "focal_list_descendants",
            "List descendant nodes breadth-first for one node.",
            read_only(),
        ),
        tool::<EmptyArgs>(
            "focal_rebuild_index",
            "Rebuild and return the graph validation index.",
            ToolAnnotations::new()
                .read_only(false)
                .destructive(false)
                .open_world(false),
        ),
    ]
}

pub(crate) fn call_tool(
    server: &FocalMcpServer,
    name: &str,
    arguments: Option<JsonObject>,
) -> Result<CallToolResult, McpError> {
    match name {
        "focal_add_context_document" => {
            let args: AddContextDocumentArgs = parse_args(name, arguments)?;
            let context = NewContextDocument::from(args);
            let result =
                server.with_backend(|backend| focal_core::add_context_document(backend, context));
            id_result(result, "context document added", context_resource)
        }
        "focal_read_context_document" => {
            let args: ContextIdArgs = parse_args(name, arguments)?;
            let result = server.with_backend(|backend| {
                focal_core::read_context_document(backend, &args.context_id)
            });
            value_result(result, "context document read", |document| {
                Some(context_resource(&document.id))
            })
        }
        "focal_update_context_document" => {
            let args: UpdateContextDocumentArgs = parse_args(name, arguments)?;
            let patch = ContextDocumentPatch {
                title: args.title,
                markdown: args.markdown,
            };
            let result = server.with_backend(|backend| {
                focal_core::update_context_document(backend, &args.context_id, patch)
            });
            value_result(result, "context document updated", |document| {
                Some(context_resource(&document.id))
            })
        }
        "focal_delete_context_document" => {
            let args: ContextIdArgs = parse_args(name, arguments)?;
            let result = server.with_backend(|backend| {
                focal_core::delete_context_document(backend, &args.context_id)
            });
            unit_result(result, "context document deleted")
        }
        "focal_list_context_documents" => {
            let _args: EmptyArgs = parse_args(name, arguments)?;
            let result = server.with_backend(|backend| focal_core::list_context_documents(backend));
            value_result(result, "context documents listed", |_| None)
        }
        "focal_add_root_node" => {
            let args: AddRootNodeArgs = parse_args(name, arguments)?;
            let node = NewNode::from(args);
            let result = server.with_backend(|backend| focal_core::add_root_node(backend, node));
            id_result(result, "root node added", node_resource)
        }
        "focal_add_child_node" => {
            let args: AddChildNodeArgs = parse_args(name, arguments)?;
            let parent_id = args.parent_id.clone();
            let node = NewNode::from(args);
            let result = server
                .with_backend(|backend| focal_core::add_child_node(backend, &parent_id, node));
            id_result(result, "child node added", node_resource)
        }
        "focal_read_node" => {
            let args: NodeIdArgs = parse_args(name, arguments)?;
            let result =
                server.with_backend(|backend| focal_core::read_node(backend, &args.node_id));
            value_result(result, "node read", |node| Some(node_resource(&node.id)))
        }
        "focal_update_node" => {
            let args: UpdateNodeArgs = parse_args(name, arguments)?;
            let patch = NodePatch {
                title: args.title,
                content: args.content,
                reviewed: args.reviewed,
            };
            let result = server
                .with_backend(|backend| focal_core::update_node(backend, &args.node_id, patch));
            value_result(result, "node updated", |node| Some(node_resource(&node.id)))
        }
        "focal_delete_node" => {
            let args: DeleteNodeArgs = parse_args(name, arguments)?;
            let result = server
                .with_backend(|backend| focal_core::delete_node(backend, &args.node_id, args.mode));
            unit_result(result, "node deleted")
        }
        "focal_link_existing_node" => {
            let args: ParentChildArgs = parse_args(name, arguments)?;
            let result = server.with_backend(|backend| {
                focal_core::link_existing_node(backend, &args.parent_id, &args.child_id)
            });
            unit_result(result, "node linked")
        }
        "focal_unlink_child" => {
            let args: UnlinkChildArgs = parse_args(name, arguments)?;
            let result = server.with_backend(|backend| {
                focal_core::unlink_child(
                    backend,
                    &args.parent_id,
                    &args.child_id,
                    args.orphan_policy,
                )
            });
            unit_result(result, "node unlinked")
        }
        "focal_list_roots" => {
            let _args: EmptyArgs = parse_args(name, arguments)?;
            let result = server.with_backend(|backend| focal_core::list_roots(backend));
            value_result(result, "root nodes listed", |_| None)
        }
        "focal_list_children" => {
            let args: NodeIdArgs = parse_args(name, arguments)?;
            let result =
                server.with_backend(|backend| focal_core::list_children(backend, &args.node_id));
            value_result(result, "child nodes listed", |_| None)
        }
        "focal_list_parents" => {
            let args: NodeIdArgs = parse_args(name, arguments)?;
            let result =
                server.with_backend(|backend| focal_core::list_parents(backend, &args.node_id));
            value_result(result, "parent nodes listed", |_| None)
        }
        "focal_list_ancestors" => {
            let args: TraversalArgs = parse_args(name, arguments)?;
            let options = TraversalOptions {
                max_depth: args.max_depth,
            };
            let result = server.with_backend(|backend| {
                focal_core::list_ancestors(backend, &args.node_id, options)
            });
            value_result(result, "ancestor nodes listed", |_| None)
        }
        "focal_list_descendants" => {
            let args: TraversalArgs = parse_args(name, arguments)?;
            let options = TraversalOptions {
                max_depth: args.max_depth,
            };
            let result = server.with_backend(|backend| {
                focal_core::list_descendants(backend, &args.node_id, options)
            });
            value_result(result, "descendant nodes listed", |_| None)
        }
        "focal_rebuild_index" => {
            let _args: EmptyArgs = parse_args(name, arguments)?;
            let result = server.with_backend(|backend| focal_core::rebuild_index(backend));
            value_result(result, "graph index rebuilt", |_| None)
        }
        _ => Err(McpError::new(
            ErrorCode::METHOD_NOT_FOUND,
            format!("unknown Focal tool `{name}`"),
            Some(json!({ "tool": name, "known_tools": TOOL_NAMES })),
        )),
    }
}

fn tool<T>(name: &'static str, description: &'static str, annotations: ToolAnnotations) -> Tool
where
    T: JsonSchema,
{
    Tool::new(name, description, schema_for::<T>()).with_annotations(annotations)
}

fn schema_for<T>() -> Arc<JsonObject>
where
    T: JsonSchema,
{
    match serde_json::to_value(schemars::schema_for!(T)) {
        Ok(Value::Object(object)) => Arc::new(object),
        Ok(_) | Err(_) => Arc::new(JsonObject::new()),
    }
}

fn read_only() -> ToolAnnotations {
    ToolAnnotations::new().read_only(true).open_world(false)
}

fn additive() -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(false)
        .destructive(false)
        .open_world(false)
}

fn destructive() -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(false)
        .destructive(true)
        .open_world(false)
}

fn parse_args<T>(tool_name: &str, arguments: Option<JsonObject>) -> Result<T, McpError>
where
    T: for<'de> Deserialize<'de>,
{
    let value = Value::Object(arguments.unwrap_or_default());
    serde_json::from_value(value).map_err(|source| {
        McpError::invalid_params(
            format!("invalid arguments for `{tool_name}`"),
            Some(json!({
                "tool": tool_name,
                "message": source.to_string(),
            })),
        )
    })
}

fn id_result(
    result: Result<String, crate::Error>,
    text: &'static str,
    resource: fn(&str) -> RawResource,
) -> Result<CallToolResult, McpError> {
    match result {
        Ok(id) => success(json!({ "id": id }), text, Some(resource(&id))),
        Err(crate::Error::Core(error)) => Ok(core_error_result(&error)),
        Err(error) => Err(internal_error(error)),
    }
}

fn unit_result(
    result: Result<(), crate::Error>,
    text: &'static str,
) -> Result<CallToolResult, McpError> {
    match result {
        Ok(()) => success(json!({ "ok": true }), text, None),
        Err(crate::Error::Core(error)) => Ok(core_error_result(&error)),
        Err(error) => Err(internal_error(error)),
    }
}

fn value_result<T>(
    result: Result<T, crate::Error>,
    text: &'static str,
    link: impl FnOnce(&T) -> Option<RawResource>,
) -> Result<CallToolResult, McpError>
where
    T: Serialize,
{
    match result {
        Ok(value) => {
            let resource = link(&value);
            let structured = serde_json::to_value(value).map_err(serialization_error)?;
            success(structured, text, resource)
        }
        Err(crate::Error::Core(error)) => Ok(core_error_result(&error)),
        Err(error) => Err(internal_error(error)),
    }
}

fn success(
    structured: Value,
    text: &'static str,
    resource: Option<RawResource>,
) -> Result<CallToolResult, McpError> {
    let mut result = CallToolResult::structured(structured);
    result.content = vec![Content::text(text)];
    if let Some(resource) = resource {
        result.content.push(Content::resource_link(resource));
    }
    Ok(result)
}

fn core_error_result(error: &focal_core::Error) -> CallToolResult {
    let body = core_error_body(error);
    let message = body.message.clone();
    let mut result =
        CallToolResult::structured_error(serde_json::to_value(&body).unwrap_or_else(|_| {
            json!({
                "backend": body.backend,
                "error_kind": "serialization_failed",
                "message": message,
            })
        }));
    result.content = vec![Content::text(message)];
    result
}

#[derive(Debug, Serialize)]
struct CoreErrorBody {
    backend: &'static str,
    error_kind: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    paths: Vec<String>,
}

fn core_error_body(error: &focal_core::Error) -> CoreErrorBody {
    match error {
        focal_core::Error::Fs(error) => graph_error_body("fs", error.as_graph_error()),
        #[cfg(feature = "sqlite")]
        focal_core::Error::Sqlite(error) => graph_error_body("sqlite", error.as_graph_error()),
        #[cfg(not(feature = "sqlite"))]
        focal_core::Error::Sqlite(error) => CoreErrorBody {
            backend: "sqlite_disabled",
            error_kind: "sqlite_disabled",
            message: error.to_string(),
            node_id: None,
            context_id: None,
            path: None,
            paths: Vec::new(),
        },
    }
}

fn graph_error_body(backend: &'static str, error: &focal_core::GraphError) -> CoreErrorBody {
    let mut body = CoreErrorBody {
        backend,
        error_kind: graph_error_kind(error),
        message: error.to_string(),
        node_id: None,
        context_id: None,
        path: None,
        paths: Vec::new(),
    };

    match error {
        focal_core::GraphError::ContextNotFound(id)
        | focal_core::GraphError::InvalidContextId(id)
        | focal_core::GraphError::DuplicateContextId(id) => {
            body.context_id = Some(id.clone());
        }
        focal_core::GraphError::NodeNotFound(id)
        | focal_core::GraphError::ParentNotFound(id)
        | focal_core::GraphError::ChildNotFound(id)
        | focal_core::GraphError::DuplicateNodeId(id)
        | focal_core::GraphError::InvalidNodeId(id)
        | focal_core::GraphError::NodeHasChildren(id)
        | focal_core::GraphError::WouldOrphanNode(id) => {
            body.node_id = Some(id.clone());
        }
        focal_core::GraphError::InvalidContextMarkdown { path, .. }
        | focal_core::GraphError::InvalidMarkdown { path, .. }
        | focal_core::GraphError::MissingNodeMarkdown(path)
        | focal_core::GraphError::MissingChildrenDirectory(path)
        | focal_core::GraphError::BrokenSymlink(path)
        | focal_core::GraphError::PermissionDenied(path)
        | focal_core::GraphError::AliasConflict(path) => {
            body.path = Some(path.to_string_lossy().into_owned());
        }
        focal_core::GraphError::DuplicateCanonicalNode { id, paths } => {
            body.node_id = Some(id.clone());
            body.paths = paths
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
        }
        focal_core::GraphError::DuplicateContextDocument { id, paths } => {
            body.context_id = Some(id.clone());
            body.paths = paths
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
        }
        focal_core::GraphError::Io(_)
        | focal_core::GraphError::Storage(_)
        | focal_core::GraphError::InvalidGraphRoot(_)
        | focal_core::GraphError::InvalidTitle
        | focal_core::GraphError::SymlinkUnsupported(_)
        | focal_core::GraphError::CycleDetected => {}
    }

    body
}

fn graph_error_kind(error: &focal_core::GraphError) -> &'static str {
    match error {
        focal_core::GraphError::Io(_) => "io",
        focal_core::GraphError::Storage(_) => "storage",
        focal_core::GraphError::InvalidGraphRoot(_) => "invalid_graph_root",
        focal_core::GraphError::ContextNotFound(_) => "context_not_found",
        focal_core::GraphError::NodeNotFound(_) => "node_not_found",
        focal_core::GraphError::ParentNotFound(_) => "parent_not_found",
        focal_core::GraphError::ChildNotFound(_) => "child_not_found",
        focal_core::GraphError::DuplicateNodeId(_) => "duplicate_node_id",
        focal_core::GraphError::DuplicateContextId(_) => "duplicate_context_id",
        focal_core::GraphError::InvalidNodeId(_) => "invalid_node_id",
        focal_core::GraphError::InvalidContextId(_) => "invalid_context_id",
        focal_core::GraphError::InvalidTitle => "invalid_title",
        focal_core::GraphError::InvalidContextMarkdown { .. } => "invalid_context_markdown",
        focal_core::GraphError::InvalidMarkdown { .. } => "invalid_markdown",
        focal_core::GraphError::MissingNodeMarkdown(_) => "missing_node_markdown",
        focal_core::GraphError::MissingChildrenDirectory(_) => "missing_children_directory",
        focal_core::GraphError::BrokenSymlink(_) => "broken_symlink",
        focal_core::GraphError::SymlinkUnsupported(_) => "symlink_unsupported",
        focal_core::GraphError::CycleDetected => "cycle_detected",
        focal_core::GraphError::NodeHasChildren(_) => "node_has_children",
        focal_core::GraphError::WouldOrphanNode(_) => "would_orphan_node",
        focal_core::GraphError::PermissionDenied(_) => "permission_denied",
        focal_core::GraphError::AliasConflict(_) => "alias_conflict",
        focal_core::GraphError::DuplicateCanonicalNode { .. } => "duplicate_canonical_node",
        focal_core::GraphError::DuplicateContextDocument { .. } => "duplicate_context_document",
    }
}

fn serialization_error(source: serde_json::Error) -> McpError {
    McpError::internal_error(
        "failed to serialize Focal MCP result",
        Some(json!({ "message": source.to_string() })),
    )
}

fn internal_error(error: crate::Error) -> McpError {
    McpError::internal_error(
        "failed to execute Focal MCP tool",
        Some(json!({ "message": error.to_string() })),
    )
}

fn context_resource(id: &str) -> RawResource {
    RawResource::new(format!("focal://contexts/{id}"), "context document")
        .with_mime_type("application/json")
}

fn node_resource(id: &str) -> RawResource {
    RawResource::new(format!("focal://nodes/{id}"), "node").with_mime_type("application/json")
}
