# GraphRAG Architecture

GraphRAG is a knowledge graph builder powered by large language models (LLMs). It is written in Rust and uses the Samyama graph database as its embedded storage engine.

## Core Components

The system has three main components: the ingestion pipeline, the MCP server, and the query engine.

The ingestion pipeline reads text files, splits them into chunks, and sends each chunk to an LLM for entity and relationship extraction. The extracted knowledge is then merged into a property graph using Cypher queries.

The MCP server exposes the knowledge graph to AI assistants via the Model Context Protocol. It implements JSON-RPC 2.0 over stdio and provides tools for searching entities, finding relationships, and traversing the graph neighborhood.

## Technology Stack

GraphRAG depends on several key technologies:

- **Rust** provides memory safety and high performance for the core engine
- **Samyama** is the embedded graph database that stores all entities and relationships
- **OpenAI GPT-4o-mini** performs entity and relationship extraction from text chunks
- **MCP (Model Context Protocol)** enables AI assistants like Claude to query the knowledge graph
- **Tokio** provides the async runtime for I/O operations

## Use Cases

Knowledge graphs built with GraphRAG can be used for:

1. Research paper analysis — extracting key concepts and their relationships
2. Codebase understanding — mapping software components and dependencies
3. Organizational knowledge — capturing institutional knowledge from documents
4. Compliance mapping — linking regulations to business processes
