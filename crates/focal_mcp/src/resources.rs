use focal_core::TraversalOptions;
use rmcp::ErrorData as McpError;
use rmcp::model::{
    AnnotateAble, ErrorCode, ListResourceTemplatesResult, ListResourcesResult, RawResource,
    RawResourceTemplate, ReadResourceResult, Resource, ResourceContents, ResourceTemplate,
};
use serde::Serialize;
use serde_json::json;
use url::Url;
use uuid::Uuid;

use crate::server::FocalMcpServer;

enum FocalResource {
    GraphIndex,
    Contexts,
    Context {
        context_id: String,
    },
    ContextMarkdown {
        context_id: String,
    },
    Roots,
    Node {
        node_id: String,
    },
    Children {
        node_id: String,
    },
    Parents {
        node_id: String,
    },
    Ancestors {
        node_id: String,
        max_depth: Option<usize>,
    },
    Descendants {
        node_id: String,
        max_depth: Option<usize>,
    },
}

pub(crate) fn list_resources() -> ListResourcesResult {
    ListResourcesResult::with_all_items(vec![
        resource(
            "focal://graph/index",
            "graph index",
            "Graph validation index",
            "application/json",
        ),
        resource(
            "focal://contexts",
            "contexts",
            "Context document summaries",
            "application/json",
        ),
        resource(
            "focal://roots",
            "roots",
            "Root node summaries",
            "application/json",
        ),
    ])
}

pub(crate) fn list_resource_templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::with_all_items(vec![
        template(
            "focal://contexts/{context_id}",
            "context document",
            "Read one context document as JSON",
            "application/json",
        ),
        template(
            "focal://contexts/{context_id}/markdown",
            "context markdown",
            "Read one context document Markdown body",
            "text/markdown",
        ),
        template(
            "focal://nodes/{node_id}",
            "node",
            "Read one node as JSON",
            "application/json",
        ),
        template(
            "focal://nodes/{node_id}/children",
            "node children",
            "List direct child node summaries",
            "application/json",
        ),
        template(
            "focal://nodes/{node_id}/parents",
            "node parents",
            "List direct parent node summaries",
            "application/json",
        ),
        template(
            "focal://nodes/{node_id}/ancestors{?max_depth}",
            "node ancestors",
            "List ancestor node summaries",
            "application/json",
        ),
        template(
            "focal://nodes/{node_id}/descendants{?max_depth}",
            "node descendants",
            "List descendant node summaries",
            "application/json",
        ),
    ])
}

pub(crate) fn read_resource(
    server: &FocalMcpServer,
    uri: &str,
) -> Result<ReadResourceResult, McpError> {
    match parse_resource(uri)? {
        FocalResource::GraphIndex => {
            let index = server
                .with_backend(|backend| focal_core::rebuild_index(backend))
                .map_err(resource_core_error)?;
            json_resource(uri, &index)
        }
        FocalResource::Contexts => {
            let contexts = server
                .with_backend(|backend| focal_core::list_context_documents(backend))
                .map_err(resource_core_error)?;
            json_resource(uri, &contexts)
        }
        FocalResource::Context { context_id } => {
            let context = server
                .with_backend(|backend| focal_core::read_context_document(backend, &context_id))
                .map_err(resource_core_error)?;
            json_resource(uri, &context)
        }
        FocalResource::ContextMarkdown { context_id } => {
            let context = server
                .with_backend(|backend| focal_core::read_context_document(backend, &context_id))
                .map_err(resource_core_error)?;
            text_resource(uri, "text/markdown", context.markdown)
        }
        FocalResource::Roots => {
            let roots = server
                .with_backend(|backend| focal_core::list_roots(backend))
                .map_err(resource_core_error)?;
            json_resource(uri, &roots)
        }
        FocalResource::Node { node_id } => {
            let node = server
                .with_backend(|backend| focal_core::read_node(backend, &node_id))
                .map_err(resource_core_error)?;
            json_resource(uri, &node)
        }
        FocalResource::Children { node_id } => {
            let children = server
                .with_backend(|backend| focal_core::list_children(backend, &node_id))
                .map_err(resource_core_error)?;
            json_resource(uri, &children)
        }
        FocalResource::Parents { node_id } => {
            let parents = server
                .with_backend(|backend| focal_core::list_parents(backend, &node_id))
                .map_err(resource_core_error)?;
            json_resource(uri, &parents)
        }
        FocalResource::Ancestors { node_id, max_depth } => {
            let options = TraversalOptions { max_depth };
            let ancestors = server
                .with_backend(|backend| focal_core::list_ancestors(backend, &node_id, options))
                .map_err(resource_core_error)?;
            json_resource(uri, &ancestors)
        }
        FocalResource::Descendants { node_id, max_depth } => {
            let options = TraversalOptions { max_depth };
            let descendants = server
                .with_backend(|backend| focal_core::list_descendants(backend, &node_id, options))
                .map_err(resource_core_error)?;
            json_resource(uri, &descendants)
        }
    }
}

fn resource(
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    mime: &'static str,
) -> Resource {
    RawResource::new(uri, name)
        .with_description(description)
        .with_mime_type(mime)
        .no_annotation()
}

fn template(
    uri_template: &'static str,
    name: &'static str,
    description: &'static str,
    mime: &'static str,
) -> ResourceTemplate {
    RawResourceTemplate::new(uri_template, name)
        .with_description(description)
        .with_mime_type(mime)
        .no_annotation()
}

fn parse_resource(uri: &str) -> Result<FocalResource, McpError> {
    let parsed = Url::parse(uri).map_err(|source| {
        McpError::invalid_params(
            "invalid focal resource URI",
            Some(json!({ "uri": uri, "message": source.to_string() })),
        )
    })?;

    if parsed.scheme() != "focal" {
        return Err(invalid_uri(uri, "resource URI scheme must be focal"));
    }
    if parsed.username() != "" || parsed.password().is_some() || parsed.port().is_some() {
        return Err(invalid_uri(
            uri,
            "focal resource URI must not contain user info or a port",
        ));
    }

    let Some(collection) = parsed.host_str() else {
        return Err(invalid_uri(
            uri,
            "focal resource URI must include a collection",
        ));
    };
    let segments = path_segments(&parsed, uri)?;

    match collection {
        "graph" if segments.as_slice() == ["index"] => {
            reject_query(&parsed, uri)?;
            Ok(FocalResource::GraphIndex)
        }
        "contexts" if segments.is_empty() => {
            reject_query(&parsed, uri)?;
            Ok(FocalResource::Contexts)
        }
        "contexts" if segments.len() == 1 => {
            reject_query(&parsed, uri)?;
            let context_id = validate_context_id(&segments[0], uri)?;
            Ok(FocalResource::Context { context_id })
        }
        "contexts" if segments.len() == 2 && segments[1] == "markdown" => {
            reject_query(&parsed, uri)?;
            let context_id = validate_context_id(&segments[0], uri)?;
            Ok(FocalResource::ContextMarkdown { context_id })
        }
        "roots" if segments.is_empty() => {
            reject_query(&parsed, uri)?;
            Ok(FocalResource::Roots)
        }
        "nodes" if segments.len() == 1 => {
            reject_query(&parsed, uri)?;
            let node_id = validate_node_id(&segments[0], uri)?;
            Ok(FocalResource::Node { node_id })
        }
        "nodes" if segments.len() == 2 && segments[1] == "children" => {
            reject_query(&parsed, uri)?;
            let node_id = validate_node_id(&segments[0], uri)?;
            Ok(FocalResource::Children { node_id })
        }
        "nodes" if segments.len() == 2 && segments[1] == "parents" => {
            reject_query(&parsed, uri)?;
            let node_id = validate_node_id(&segments[0], uri)?;
            Ok(FocalResource::Parents { node_id })
        }
        "nodes" if segments.len() == 2 && segments[1] == "ancestors" => {
            let node_id = validate_node_id(&segments[0], uri)?;
            Ok(FocalResource::Ancestors {
                node_id,
                max_depth: parse_max_depth(&parsed, uri)?,
            })
        }
        "nodes" if segments.len() == 2 && segments[1] == "descendants" => {
            let node_id = validate_node_id(&segments[0], uri)?;
            Ok(FocalResource::Descendants {
                node_id,
                max_depth: parse_max_depth(&parsed, uri)?,
            })
        }
        _ => Err(McpError::resource_not_found(
            "unknown Focal resource",
            Some(json!({ "uri": uri })),
        )),
    }
}

fn path_segments(parsed: &Url, uri: &str) -> Result<Vec<String>, McpError> {
    let Some(segments) = parsed.path_segments() else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(invalid_uri(
                uri,
                "resource URI path traversal is not allowed",
            ));
        }
        result.push(segment.to_string());
    }
    Ok(result)
}

fn reject_query(parsed: &Url, uri: &str) -> Result<(), McpError> {
    if parsed.query().is_some() {
        return Err(invalid_uri(
            uri,
            "resource URI does not accept query parameters",
        ));
    }
    Ok(())
}

fn parse_max_depth(parsed: &Url, uri: &str) -> Result<Option<usize>, McpError> {
    let mut max_depth = None;
    for (key, value) in parsed.query_pairs() {
        if key != "max_depth" {
            return Err(invalid_uri(uri, "unknown resource URI query parameter"));
        }
        if max_depth.is_some() {
            return Err(invalid_uri(uri, "duplicate max_depth query parameter"));
        }
        if value.is_empty() {
            return Err(invalid_uri(uri, "max_depth must not be empty"));
        }
        max_depth = Some(value.parse::<usize>().map_err(|source| {
            McpError::invalid_params(
                "invalid max_depth query parameter",
                Some(json!({ "uri": uri, "message": source.to_string() })),
            )
        })?);
    }
    Ok(max_depth)
}

fn validate_node_id(id: &str, uri: &str) -> Result<String, McpError> {
    validate_uuid_id(id, uri, "node_id")
}

fn validate_context_id(id: &str, uri: &str) -> Result<String, McpError> {
    validate_uuid_id(id, uri, "context_id")
}

fn validate_uuid_id(id: &str, uri: &str, field: &'static str) -> Result<String, McpError> {
    let valid_shape = id.len() == 36
        && id.char_indices().all(|(index, ch)| match index {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_digit() || ('a'..='f').contains(&ch),
        });
    if valid_shape && Uuid::parse_str(id).is_ok() {
        Ok(id.to_string())
    } else {
        Err(McpError::invalid_params(
            "invalid Focal resource identifier",
            Some(json!({ "uri": uri, "field": field, "value": id })),
        ))
    }
}

fn invalid_uri(uri: &str, message: &'static str) -> McpError {
    McpError::invalid_params(message, Some(json!({ "uri": uri })))
}

fn json_resource<T>(uri: &str, value: &T) -> Result<ReadResourceResult, McpError>
where
    T: Serialize,
{
    let text = serde_json::to_string(value).map_err(|source| {
        McpError::internal_error(
            "failed to serialize Focal resource",
            Some(json!({ "uri": uri, "message": source.to_string() })),
        )
    })?;
    text_resource(uri, "application/json", text)
}

fn text_resource(
    uri: &str,
    mime_type: &'static str,
    text: String,
) -> Result<ReadResourceResult, McpError> {
    Ok(ReadResourceResult::new(vec![
        ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(mime_type.to_string()),
            text,
            meta: None,
        },
    ]))
}

fn resource_core_error(error: crate::Error) -> McpError {
    match error {
        crate::Error::Core(error) => McpError::new(
            ErrorCode::RESOURCE_NOT_FOUND,
            "failed to read Focal resource",
            Some(json!({ "message": error.to_string() })),
        ),
        other => McpError::internal_error(
            "failed to read Focal resource",
            Some(json!({ "message": other.to_string() })),
        ),
    }
}
