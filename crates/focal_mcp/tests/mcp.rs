use std::fs;

use focal_core::{Node, NodeContent};
use focal_mcp::{BackendConfig, FocalMcpServer, OpenMode, ServerConfig, TransportConfig};
use rmcp::ServerHandler;
use rmcp::model::{JsonObject, ResourceContents};
use serde_json::{Value, json};

fn args(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        other => panic!("tool arguments must be an object: {other}"),
    }
}

fn structured_id(result: &rmcp::model::CallToolResult) -> String {
    result
        .structured_content
        .as_ref()
        .and_then(|value| value.get("id"))
        .and_then(Value::as_str)
        .expect("tool result should include structured id")
        .to_string()
}

fn text_resource(result: rmcp::model::ReadResourceResult) -> String {
    let Some(ResourceContents::TextResourceContents { text, .. }) =
        result.contents.into_iter().next()
    else {
        panic!("expected one text resource");
    };
    text
}

fn fs_server() -> (tempfile::TempDir, FocalMcpServer) {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let server = FocalMcpServer::from_config(ServerConfig {
        backend: BackendConfig::Fs {
            root: tempdir.path().join("ideas"),
            mode: OpenMode::Init,
        },
        transport: TransportConfig::Stdio,
    })
    .expect("init fs server");
    (tempdir, server)
}

#[test]
fn yaml_config_loads_filesystem_stdio_server() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let config_path = tempdir.path().join("focal-mcp.yaml");
    fs::write(
        &config_path,
        format!(
            "backend:\n  type: fs\n  root: {}\n  mode: init\ntransport:\n  type: stdio\n",
            tempdir.path().join("ideas").display()
        ),
    )
    .expect("write config");

    let config = focal_mcp::load_config(&config_path).expect("load config");

    assert_eq!(config.transport, TransportConfig::Stdio);
    assert!(matches!(
        config.backend,
        BackendConfig::Fs {
            mode: OpenMode::Init,
            ..
        }
    ));
}

#[test]
fn capabilities_tools_and_resource_templates_are_advertised() {
    let (_tempdir, server) = fs_server();
    let info = server.get_info();

    assert_eq!(info.protocol_version.to_string(), "2025-11-25");
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());

    let tools = FocalMcpServer::tools();
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    for required in [
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
    ] {
        assert!(names.contains(&required), "missing tool {required}");
    }

    let read_node = tools
        .iter()
        .find(|tool| tool.name == "focal_read_node")
        .expect("read node tool");
    assert_eq!(
        read_node
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.read_only_hint),
        Some(true)
    );

    let link = tools
        .iter()
        .find(|tool| tool.name == "focal_link_existing_node")
        .expect("link tool");
    assert_eq!(
        link.annotations
            .as_ref()
            .and_then(|annotations| annotations.idempotent_hint),
        Some(true)
    );

    let templates = FocalMcpServer::list_resource_templates_result();
    let uri_templates = templates
        .resource_templates
        .iter()
        .map(|template| template.uri_template.as_str())
        .collect::<Vec<_>>();
    assert!(uri_templates.contains(&"focal://nodes/{node_id}/descendants{?max_depth}"));
    assert!(uri_templates.contains(&"focal://contexts/{context_id}/markdown"));
}

#[test]
fn filesystem_tools_dispatch_to_core_and_return_structured_content() {
    let (_tempdir, server) = fs_server();

    let context_result = server
        .call_tool_by_name(
            "focal_add_context_document",
            Some(args(json!({
                "title": "Raw planning notes",
                "markdown": "Messy input"
            }))),
        )
        .expect("add context tool");
    assert_eq!(context_result.is_error, Some(false));
    let context_id = structured_id(&context_result);

    let root_result = server
        .call_tool_by_name(
            "focal_add_root_node",
            Some(args(json!({
                "kind": "statement",
                "title": "Rust keeps local tools simple",
                "content": {
                    "type": "statement",
                    "body": "Rust can manage local files without a background service."
                }
            }))),
        )
        .expect("add root tool");
    let root_id = structured_id(&root_result);

    let child_result = server
        .call_tool_by_name(
            "focal_add_child_node",
            Some(args(json!({
                "parent_id": root_id,
                "kind": "qa",
                "title": "Why use symlinks?",
                "content": {
                    "type": "qa",
                    "question": "Why use symlinks for shared children?",
                    "answer": "They preserve one canonical Markdown file.",
                    "alternative_answers": ["They keep shared children visible."]
                }
            }))),
        )
        .expect("add child tool");
    let child_id = structured_id(&child_result);

    let descendants = server
        .call_tool_by_name(
            "focal_list_descendants",
            Some(args(json!({ "node_id": root_id, "max_depth": null }))),
        )
        .expect("list descendants");
    let descendants = descendants
        .structured_content
        .as_ref()
        .and_then(Value::as_array)
        .expect("descendants array");
    assert_eq!(descendants.len(), 1);
    assert_eq!(descendants[0]["id"], child_id);

    let reviewed = server
        .call_tool_by_name(
            "focal_update_node",
            Some(args(json!({ "node_id": child_id, "reviewed": true }))),
        )
        .expect("review node");
    assert_eq!(
        reviewed.structured_content.as_ref().unwrap()["reviewed"],
        true
    );

    let context = server
        .call_tool_by_name(
            "focal_read_context_document",
            Some(args(json!({ "context_id": context_id }))),
        )
        .expect("read context");
    assert_eq!(
        context.structured_content.as_ref().unwrap()["markdown"],
        "Messy input"
    );
}

#[test]
fn resources_return_core_data_and_markdown_views() {
    let (_tempdir, server) = fs_server();

    let context_id = structured_id(
        &server
            .call_tool_by_name(
                "focal_add_context_document",
                Some(args(json!({
                    "title": "Raw notes",
                    "markdown": "Markdown body"
                }))),
            )
            .expect("add context"),
    );
    let node_id = structured_id(
        &server
            .call_tool_by_name(
                "focal_add_root_node",
                Some(args(json!({
                    "kind": "statement",
                    "title": "Root",
                    "content": { "type": "statement", "body": "Body" }
                }))),
            )
            .expect("add root"),
    );

    let node_text = text_resource(
        server
            .read_resource_uri(&format!("focal://nodes/{node_id}"))
            .expect("read node resource"),
    );
    let node: Node = serde_json::from_str(&node_text).expect("node json");
    assert_eq!(node.id, node_id);
    assert_eq!(
        node.content,
        NodeContent::Statement {
            body: "Body".to_string()
        }
    );

    let markdown = text_resource(
        server
            .read_resource_uri(&format!("focal://contexts/{context_id}/markdown"))
            .expect("read markdown resource"),
    );
    assert_eq!(markdown, "Markdown body");

    let roots = text_resource(
        server
            .read_resource_uri("focal://roots")
            .expect("read roots resource"),
    );
    assert!(roots.contains(&node_id));
}

#[test]
fn tool_and_resource_errors_are_structured_and_do_not_panic() {
    let (_tempdir, server) = fs_server();

    let tool_error = server
        .call_tool_by_name(
            "focal_read_node",
            Some(args(json!({ "node_id": "not-a-uuid" }))),
        )
        .expect("tool should return MCP tool error result");
    assert_eq!(tool_error.is_error, Some(true));
    let structured = tool_error.structured_content.as_ref().expect("error body");
    assert_eq!(structured["backend"], "fs");
    assert_eq!(structured["error_kind"], "invalid_node_id");
    assert_eq!(structured["node_id"], "not-a-uuid");

    let resource_error = server
        .read_resource_uri("focal://nodes/550E8400-e29b-41d4-a716-446655440000")
        .expect_err("uppercase UUID should be rejected before core read");
    assert_eq!(resource_error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_tools_and_resources_use_configured_namespace() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let server = FocalMcpServer::from_config(ServerConfig {
        backend: BackendConfig::Sqlite {
            database_path: tempdir.path().join("focal.db"),
            graph_name: "main".to_string(),
            mode: OpenMode::Init,
        },
        transport: TransportConfig::Stdio,
    })
    .expect("init sqlite server");

    let node_id = structured_id(
        &server
            .call_tool_by_name(
                "focal_add_root_node",
                Some(args(json!({
                    "kind": "statement",
                    "title": "SQLite root",
                    "content": { "type": "statement", "body": "Persisted" }
                }))),
            )
            .expect("add sqlite root"),
    );

    let node_text = text_resource(
        server
            .read_resource_uri(&format!("focal://nodes/{node_id}"))
            .expect("read sqlite node resource"),
    );
    assert!(node_text.contains("SQLite root"));
}

#[test]
fn public_mcp_code_avoids_panic_style_recoverable_paths() {
    let sources = [
        include_str!("../src/lib.rs"),
        include_str!("../src/config.rs"),
        include_str!("../src/error.rs"),
        include_str!("../src/resources.rs"),
        include_str!("../src/server.rs"),
        include_str!("../src/tools.rs"),
    ];

    for source in sources {
        for forbidden in [
            "unwrap(",
            "expect(",
            "panic!",
            "todo!",
            "unimplemented!",
            "get_unchecked",
        ] {
            assert!(
                !source.contains(forbidden),
                "public focal-mcp source contains forbidden recoverable-state pattern: {forbidden}"
            );
        }
    }
}
