pub mod tools;
pub mod types;

use anyhow::Result;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::graph::GraphManager;
use types::{JsonRpcRequest, JsonRpcResponse};

/// Run the MCP stdio server loop.
pub async fn serve(gm: GraphManager) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    tracing::info!("MCP server started (stdio mode)");

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            // EOF
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let resp = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {e}"),
                );
                write_response(&mut stdout, &resp).await?;
                continue;
            }
        };

        let response = handle_request(&gm, &request).await;

        // Notifications (no id) don't get responses
        if request.id.is_none() {
            continue;
        }

        if let Some(resp) = response {
            write_response(&mut stdout, &resp).await?;
        }
    }

    Ok(())
}

async fn handle_request(gm: &GraphManager, req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => {
            Some(JsonRpcResponse::success(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "graphrag-rs",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            ))
        }

        "notifications/initialized" => None,

        "tools/list" => {
            Some(JsonRpcResponse::success(id, tools::tool_definitions()))
        }

        "tools/call" => {
            let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));

            match tools::call_tool(gm, tool_name, &arguments).await {
                Ok(result) => {
                    Some(JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                            }]
                        }),
                    ))
                }
                Err(e) => {
                    Some(JsonRpcResponse::success(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {e}")
                            }],
                            "isError": true
                        }),
                    ))
                }
            }
        }

        _ => {
            Some(JsonRpcResponse::error(
                id,
                -32601,
                format!("Method not found: {}", req.method),
            ))
        }
    }
}

async fn write_response(
    stdout: &mut tokio::io::Stdout,
    resp: &JsonRpcResponse,
) -> Result<()> {
    let json = serde_json::to_string(resp)?;
    stdout.write_all(json.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;
    Ok(())
}
