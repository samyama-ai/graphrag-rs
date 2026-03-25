use anyhow::Result;
use serde_json::{json, Value};

use crate::graph::GraphManager;

/// List of all MCP tool definitions.
pub fn tool_definitions() -> Value {
    json!({
        "tools": [
            {
                "name": "search_entities",
                "description": "Search for entities in the knowledge graph by name or description",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query to match against entity names and descriptions"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results (default: 10)",
                            "default": 10
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "find_relationships",
                "description": "Find all relationships connected to a specific entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity": {
                            "type": "string",
                            "description": "Exact name of the entity to find relationships for"
                        }
                    },
                    "required": ["entity"]
                }
            },
            {
                "name": "traverse_neighbors",
                "description": "Explore the neighborhood of an entity up to a given depth",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity": {
                            "type": "string",
                            "description": "Starting entity name"
                        },
                        "depth": {
                            "type": "integer",
                            "description": "Maximum traversal depth (1-4, default: 2)",
                            "default": 2
                        }
                    },
                    "required": ["entity"]
                }
            },
            {
                "name": "graph_stats",
                "description": "Get statistics about the knowledge graph (node count, edge count, labels, types)",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

/// Dispatch a tool call by name.
pub async fn call_tool(gm: &GraphManager, name: &str, args: &Value) -> Result<Value> {
    match name {
        "search_entities" => search_entities(gm, args).await,
        "find_relationships" => find_relationships(gm, args).await,
        "traverse_neighbors" => traverse_neighbors(gm, args).await,
        "graph_stats" => graph_stats(gm).await,
        _ => anyhow::bail!("Unknown tool: {name}"),
    }
}

async fn search_entities(gm: &GraphManager, args: &Value) -> Result<Value> {
    let query = args["query"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
    let limit = args["limit"].as_u64().unwrap_or(10);

    let escaped = query.replace('\'', "\\'");
    let cypher = format!(
        "MATCH (n:Entity) \
         WHERE n.name CONTAINS '{escaped}' OR n.description CONTAINS '{escaped}' \
         RETURN n.name, n.entity_type, n.description \
         LIMIT {limit}"
    );

    let result = gm.query_readonly(&cypher).await?;
    Ok(format_query_result(&result))
}

async fn find_relationships(gm: &GraphManager, args: &Value) -> Result<Value> {
    let entity = args["entity"]
        .as_str()
        .or_else(|| args["entity_name"].as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'entity' parameter"))?;
    let escaped = entity.replace('\'', "\\'");

    let cypher = format!(
        "MATCH (a:Entity {{name: '{escaped}'}})-[r]-(b:Entity) \
         RETURN a.name, type(r), b.name, r.description"
    );

    let result = gm.query_readonly(&cypher).await?;
    Ok(format_query_result(&result))
}

async fn traverse_neighbors(gm: &GraphManager, args: &Value) -> Result<Value> {
    let entity = args["entity"]
        .as_str()
        .or_else(|| args["entity_name"].as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'entity' parameter"))?;
    let depth = args["depth"].as_u64().unwrap_or(2).min(4);
    let escaped = entity.replace('\'', "\\'");

    let cypher = format!(
        "MATCH (a:Entity {{name: '{escaped}'}})-[r*1..{depth}]-(b:Entity) \
         RETURN DISTINCT b.name, b.entity_type, b.description \
         LIMIT 50"
    );

    let result = gm.query_readonly(&cypher).await?;
    Ok(format_query_result(&result))
}

async fn graph_stats(gm: &GraphManager) -> Result<Value> {
    let stats = gm.stats().await?;
    Ok(json!({
        "node_count": stats.node_count,
        "edge_count": stats.edge_count,
        "labels": stats.labels,
        "edge_types": stats.edge_types,
    }))
}

fn format_query_result(result: &samyama_sdk::QueryResult) -> Value {
    json!({
        "columns": result.columns,
        "rows": result.records,
    })
}
