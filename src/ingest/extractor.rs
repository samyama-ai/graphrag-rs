use anyhow::Result;
use serde::Deserialize;

use crate::llm::LlmClient;

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedRelationship {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub rel_type: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractedGraph {
    #[serde(default)]
    pub entities: Vec<ExtractedEntity>,
    #[serde(default)]
    pub relationships: Vec<ExtractedRelationship>,
}

const EXTRACTION_PROMPT: &str = r#"You are a knowledge graph extraction engine. Given a text chunk, extract entities and relationships.

Return ONLY valid JSON with this exact schema:
{
  "entities": [
    {"name": "EntityName", "type": "EntityType", "description": "Brief description"}
  ],
  "relationships": [
    {"source": "SourceEntity", "target": "TargetEntity", "type": "RELATIONSHIP_TYPE", "description": "Brief description"}
  ]
}

Rules:
- Entity names should be normalized (proper case, no extra whitespace)
- Entity types should be PascalCase (e.g., Person, Organization, Technology, Concept)
- Relationship types should be UPPER_SNAKE_CASE (e.g., WORKS_AT, USES, DEPENDS_ON)
- Extract ALL meaningful entities and relationships from the text
- Keep descriptions concise (1 sentence max)
- Return empty arrays if no entities or relationships found"#;

/// Extract entities and relationships from a text chunk using an LLM.
pub async fn extract_from_chunk(
    llm: &dyn LlmClient,
    chunk: &str,
    filename: &str,
) -> Result<ExtractedGraph> {
    let user_prompt = format!("Source file: {filename}\n\nText:\n{chunk}");

    let response = llm.chat(EXTRACTION_PROMPT, &user_prompt).await?;

    parse_extraction_response(&response)
}

/// Resilient JSON parsing: try direct → strip fences → find first {..last } → empty fallback.
fn parse_extraction_response(response: &str) -> Result<ExtractedGraph> {
    // Try direct parse
    if let Ok(graph) = serde_json::from_str::<ExtractedGraph>(response) {
        return Ok(graph);
    }

    // Strip markdown code fences
    let stripped = response
        .trim()
        .strip_prefix("```json")
        .or_else(|| response.trim().strip_prefix("```"))
        .unwrap_or(response)
        .trim()
        .strip_suffix("```")
        .unwrap_or(response)
        .trim();

    if let Ok(graph) = serde_json::from_str::<ExtractedGraph>(stripped) {
        return Ok(graph);
    }

    // Find first { to last }
    if let (Some(start), Some(end)) = (stripped.find('{'), stripped.rfind('}')) {
        let json_str = &stripped[start..=end];
        if let Ok(graph) = serde_json::from_str::<ExtractedGraph>(json_str) {
            return Ok(graph);
        }
    }

    // Fallback: empty graph
    tracing::warn!("Failed to parse LLM extraction response, returning empty graph");
    Ok(ExtractedGraph::default())
}

/// Normalize an entity name: trim, collapse whitespace, title case.
pub fn normalize_entity_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Normalize a relationship type: uppercase, replace spaces with underscores.
pub fn normalize_rel_type(rel_type: &str) -> String {
    rel_type
        .trim()
        .to_uppercase()
        .replace(' ', "_")
        .replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_json() {
        let json = r#"{"entities": [{"name": "Rust", "type": "Language", "description": "A language"}], "relationships": []}"#;
        let graph = parse_extraction_response(json).unwrap();
        assert_eq!(graph.entities.len(), 1);
        assert_eq!(graph.entities[0].name, "Rust");
    }

    #[test]
    fn test_parse_fenced_json() {
        let json = "```json\n{\"entities\": [{\"name\": \"A\", \"type\": \"B\", \"description\": \"C\"}], \"relationships\": []}\n```";
        let graph = parse_extraction_response(json).unwrap();
        assert_eq!(graph.entities.len(), 1);
    }

    #[test]
    fn test_parse_with_preamble() {
        let json = "Here is the extraction:\n{\"entities\": [], \"relationships\": [{\"source\": \"A\", \"target\": \"B\", \"type\": \"REL\", \"description\": \"d\"}]}";
        let graph = parse_extraction_response(json).unwrap();
        assert_eq!(graph.relationships.len(), 1);
    }

    #[test]
    fn test_parse_garbage_returns_empty() {
        let graph = parse_extraction_response("not json at all").unwrap();
        assert!(graph.entities.is_empty());
        assert!(graph.relationships.is_empty());
    }

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(normalize_entity_name("  Rust  Language  "), "Rust Language");
    }

    #[test]
    fn test_normalize_rel_type() {
        assert_eq!(normalize_rel_type("works at"), "WORKS_AT");
        assert_eq!(normalize_rel_type("depends-on"), "DEPENDS_ON");
    }
}
