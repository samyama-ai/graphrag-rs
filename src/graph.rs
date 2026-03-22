use anyhow::Result;
use samyama_sdk::{EmbeddedClient, QueryResult, SamyamaClient};
use std::path::{Path, PathBuf};

pub struct GraphStats {
    pub node_count: u64,
    pub edge_count: u64,
    pub labels: Vec<String>,
    pub edge_types: Vec<String>,
}

pub struct GraphManager {
    client: EmbeddedClient,
    snapshot_path: PathBuf,
}

impl GraphManager {
    /// Create a new GraphManager, loading existing snapshot if available.
    pub async fn new(data_dir: &str) -> Result<Self> {
        let data_path = Path::new(data_dir);
        std::fs::create_dir_all(data_path)?;

        let snapshot_path = data_path.join("graph.sgsnap");
        let client = EmbeddedClient::new();

        if snapshot_path.exists() {
            tracing::info!("Loading snapshot from {}", snapshot_path.display());
            client
                .import_snapshot("default", &snapshot_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to import snapshot: {e}"))?;
            tracing::info!("Snapshot loaded");
        }

        Ok(GraphManager {
            client,
            snapshot_path,
        })
    }

    /// Save graph to snapshot file.
    pub async fn save(&self) -> Result<()> {
        let stats = self
            .client
            .export_snapshot("default", &self.snapshot_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to export snapshot: {e}"))?;
        tracing::info!(
            "Snapshot saved: {} nodes, {} edges",
            stats.node_count,
            stats.edge_count
        );
        Ok(())
    }

    /// Execute a read/write Cypher query.
    pub async fn query(&self, cypher: &str) -> Result<QueryResult> {
        self.client
            .query("default", cypher)
            .await
            .map_err(|e| anyhow::anyhow!("Query error: {e}"))
    }

    /// Execute a read-only Cypher query.
    pub async fn query_readonly(&self, cypher: &str) -> Result<QueryResult> {
        self.client
            .query_readonly("default", cypher)
            .await
            .map_err(|e| anyhow::anyhow!("Query error: {e}"))
    }

    /// Get graph statistics.
    pub async fn stats(&self) -> Result<GraphStats> {
        let status = self
            .client
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("Status error: {e}"))?;

        // Get labels
        let label_result = self
            .client
            .query_readonly("default", "MATCH (n) RETURN DISTINCT labels(n) AS l")
            .await
            .ok();
        let mut labels: Vec<String> = label_result
            .iter()
            .flat_map(|r| &r.records)
            .filter_map(|row| {
                row.first().and_then(|v| match v {
                    serde_json::Value::Array(arr) => arr
                        .first()
                        .and_then(|s| s.as_str().map(String::from)),
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect();
        labels.sort();
        labels.dedup();

        // Get edge types
        let edge_result = self
            .client
            .query_readonly("default", "MATCH ()-[r]->() RETURN DISTINCT type(r) AS t")
            .await
            .ok();
        let edge_types: Vec<String> = edge_result
            .iter()
            .flat_map(|r| &r.records)
            .filter_map(|row| row.first().and_then(|v| v.as_str().map(String::from)))
            .collect();

        Ok(GraphStats {
            node_count: status.storage.nodes,
            edge_count: status.storage.edges,
            labels,
            edge_types,
        })
    }

    /// Generate a schema summary for LLM context.
    pub async fn schema_summary(&self) -> Result<String> {
        let stats = self.stats().await?;
        Ok(format!(
            "Graph has {} nodes and {} edges. Labels: [{}]. Edge types: [{}].",
            stats.node_count,
            stats.edge_count,
            stats.labels.join(", "),
            stats.edge_types.join(", ")
        ))
    }
}
