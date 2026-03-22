pub mod chunker;
pub mod extractor;
pub mod reader;

use anyhow::Result;

use crate::graph::GraphManager;
use crate::llm::LlmClient;
use chunker::chunk_text;
use extractor::{extract_from_chunk, normalize_entity_name, normalize_rel_type};
use reader::walk_and_read;

pub struct IngestStats {
    pub files: usize,
    pub chunks: usize,
    pub entities: usize,
    pub relationships: usize,
}

/// Run the full ingestion pipeline: walk → chunk → extract → upsert → save.
pub async fn run(
    gm: &mut GraphManager,
    llm: &dyn LlmClient,
    path: &str,
    chunk_size: usize,
    overlap: usize,
) -> Result<IngestStats> {
    let files = walk_and_read(path)?;
    eprintln!("Found {} files to process", files.len());

    let mut total_chunks = 0;
    let mut total_entities = 0;
    let mut total_relationships = 0;

    for file in &files {
        let filename = &file.path;
        eprintln!("Processing: {filename}");

        let chunks = chunk_text(&file.content, chunk_size, overlap);
        eprintln!("  {} chunks", chunks.len());

        for chunk in &chunks {
            total_chunks += 1;

            let extracted = extract_from_chunk(llm, &chunk.text, filename).await?;

            // Upsert entities
            for entity in &extracted.entities {
                let name = normalize_entity_name(&entity.name);
                if name.is_empty() {
                    continue;
                }
                let entity_type = entity.entity_type.trim();
                let description = entity.description.trim();

                // Use MERGE to avoid duplicates (Samyama requires ON CREATE SET / ON MATCH SET)
                let cypher = format!(
                    "MERGE (n:Entity {{name: '{name}'}}) \
                     ON CREATE SET n.entity_type = '{entity_type}', \
                         n.description = '{description}', \
                         n.source_file = '{filename}' \
                     ON MATCH SET n.entity_type = '{entity_type}', \
                         n.description = '{description}', \
                         n.source_file = '{filename}'",
                    name = escape_cypher(&name),
                    entity_type = escape_cypher(entity_type),
                    description = escape_cypher(description),
                    filename = escape_cypher(filename),
                );

                if let Err(e) = gm.query(&cypher).await {
                    tracing::warn!("Failed to upsert entity '{name}': {e}");
                } else {
                    total_entities += 1;
                }
            }

            // Create relationships
            for rel in &extracted.relationships {
                let source = normalize_entity_name(&rel.source);
                let target = normalize_entity_name(&rel.target);
                let rel_type = normalize_rel_type(&rel.rel_type);
                let description = rel.description.trim();

                if source.is_empty() || target.is_empty() || rel_type.is_empty() {
                    continue;
                }

                let cypher = format!(
                    "MATCH (a:Entity {{name: '{source}'}}), (b:Entity {{name: '{target}'}}) \
                     CREATE (a)-[:{rel_type} {{description: '{description}'}}]->(b)",
                    source = escape_cypher(&source),
                    target = escape_cypher(&target),
                    rel_type = rel_type,
                    description = escape_cypher(description),
                );

                if let Err(e) = gm.query(&cypher).await {
                    tracing::warn!("Failed to create relationship {source}-[{rel_type}]->{target}: {e}");
                } else {
                    total_relationships += 1;
                }
            }
        }
    }

    // Write metadata
    let metadata = serde_json::json!({
        "files_processed": files.len(),
        "chunks": total_chunks,
        "entities": total_entities,
        "relationships": total_relationships,
    });
    let data_dir = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    // Write to the data dir (where snapshots live), not the input path
    let _ = std::fs::write(
        std::path::Path::new("data").join("metadata.json"),
        serde_json::to_string_pretty(&metadata)?,
    );
    let _ = data_dir; // suppress unused warning

    Ok(IngestStats {
        files: files.len(),
        chunks: total_chunks,
        entities: total_entities,
        relationships: total_relationships,
    })
}

/// Escape single quotes in Cypher string literals.
fn escape_cypher(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_cypher() {
        assert_eq!(escape_cypher("it's a test"), "it\\'s a test");
        assert_eq!(escape_cypher("no quotes"), "no quotes");
    }
}
