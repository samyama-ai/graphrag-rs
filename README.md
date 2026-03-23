# graphrag-rs

Turn any folder of documents into a knowledge graph — then query it from Claude, Cursor, or any MCP-compatible AI assistant.

**graphrag-rs** is a single Rust binary that:
1. Reads your docs (`.md`, `.txt`, `.csv`, `.json`)
2. Extracts entities and relationships via LLM
3. Serves the knowledge graph over [MCP](https://modelcontextprotocol.io) (Model Context Protocol)

Powered by [Samyama](https://github.com/samyama-ai/samyama-graph), a high-performance embedded graph database.

## Quick Start (3 steps)

### 1. Install

```bash
curl -sSL https://raw.githubusercontent.com/samyama-ai/graphrag-rs/main/install.sh | sh
```

Or build from source:

```bash
git clone https://github.com/samyama-ai/graphrag-rs.git
cd graphrag-rs
cargo build --release
cp target/release/graphrag-rs ~/.local/bin/
```

### 2. Ingest your documents

```bash
export OPENAI_API_KEY="sk-..."
graphrag-rs ingest ./my-docs/
```

That's it. graphrag-rs walks the folder, chunks the text, extracts entities and relationships via GPT-4o-mini, and saves the graph locally.

### 3. Connect to your AI assistant

```bash
graphrag-rs serve
```

This starts an MCP server over stdio. Add it to your assistant's config:

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "graphrag": {
      "command": "graphrag-rs",
      "args": ["serve"]
    }
  }
}
```

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "graphrag": {
      "command": "graphrag-rs",
      "args": ["--data-dir", "/path/to/your/data", "serve"]
    }
  }
}
```

> Restart Claude Desktop after editing. The graphrag tools will appear under the hammer icon.

**Claude Code** (`.mcp.json` in your project directory):

```json
{
  "graphrag": {
    "type": "stdio",
    "command": "graphrag-rs",
    "args": ["--data-dir", "/path/to/your/data", "serve"]
  }
}
```

**Cursor** (`.cursor/mcp.json` in your project):

```json
{
  "mcpServers": {
    "graphrag": {
      "command": "graphrag-rs",
      "args": ["serve"]
    }
  }
}
```

Now your AI assistant can search entities, explore relationships, and traverse the knowledge graph.

## MCP Tools

| Tool | Description |
|------|-------------|
| `search_entities` | Search entities by name or description |
| `find_relationships` | Find all relationships for an entity |
| `traverse_neighbors` | Explore the neighborhood up to depth 4 |
| `graph_stats` | Get node/edge counts, labels, and types |

## CLI Reference

```
graphrag-rs ingest <path>          # Ingest files into the knowledge graph
graphrag-rs serve                  # Start MCP server (stdio)
graphrag-rs status                 # Show graph statistics
graphrag-rs query "<cypher>"       # Run a Cypher query directly
```

### Global Options

```
--provider <name>      LLM provider (default: openai)
--model <name>         Model name (default: gpt-4o-mini)
--api-key <key>        API key (or set OPENAI_API_KEY)
--data-dir <path>      Where to store the graph (default: ./data)
```

### Ingest Options

```
--chunk-size <n>       Characters per chunk (default: 2000)
--overlap <n>          Overlap between chunks (default: 200)
```

## Example

```bash
# Ingest the project's sample fixture
graphrag-rs ingest ./fixtures/

# Check what was extracted
graphrag-rs status
# => Nodes: 8, Edges: 5, Labels: Entity, Edge types: DEPENDS_ON, USES, IMPLEMENTS

# Query directly with Cypher
graphrag-rs query "MATCH (a)-[r]->(b) RETURN a.name, type(r), b.name"
```

## How It Works

```
Documents  ──►  Chunker  ──►  LLM Extraction  ──►  Knowledge Graph  ──►  MCP Server
  .md .txt       paragraph-     GPT-4o-mini          Samyama              search, traverse,
  .csv .json     aware split    entities + rels       embedded engine      query via AI
```

1. **Walk & Read** — recursively reads `.md`, `.txt`, `.csv`, `.json` files
2. **Chunk** — splits text at paragraph boundaries with configurable overlap
3. **Extract** — sends each chunk to the LLM with a structured extraction prompt; parses JSON entities and relationships
4. **Upsert** — `MERGE`s entities and `CREATE`s relationships in the embedded graph (deduplicates by name)
5. **Persist** — saves the graph as a `.sgsnap` snapshot file
6. **Serve** — exposes the graph via MCP tools over stdio

## Requirements

- **OpenAI API key** (for entity extraction during ingest)
- **Rust 1.75+** (if building from source)

## License

Apache-2.0
