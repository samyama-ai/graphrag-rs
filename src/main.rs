use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod config;
mod graph;
mod ingest;
mod llm;
mod mcp;

use config::Config;
use graph::GraphManager;

#[derive(Parser)]
#[command(name = "graphrag-rs", version, about = "GraphRAG: LLM-powered knowledge graph with MCP server")]
struct Cli {
    /// LLM provider (openai)
    #[arg(long, default_value = "openai")]
    provider: String,

    /// LLM model name
    #[arg(long, default_value = "gpt-4o-mini")]
    model: String,

    /// API key (or set OPENAI_API_KEY env var)
    #[arg(long)]
    api_key: Option<String>,

    /// Data directory for graph snapshots
    #[arg(long, default_value = "./data")]
    data_dir: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest files into the knowledge graph
    Ingest {
        /// Path to file or directory to ingest
        path: String,

        /// Chunk size in characters
        #[arg(long, default_value_t = 2000)]
        chunk_size: usize,

        /// Chunk overlap in characters
        #[arg(long, default_value_t = 200)]
        overlap: usize,
    },
    /// Start MCP server (JSON-RPC over stdio)
    Serve,
    /// Show graph status and statistics
    Status,
    /// Execute a Cypher query
    Query {
        /// Cypher query string
        cypher: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let config = Config::new(
        &cli.provider,
        &cli.model,
        cli.api_key.as_deref(),
        &cli.data_dir,
    )?;

    match cli.command {
        Commands::Ingest {
            path,
            chunk_size,
            overlap,
        } => {
            let mut gm = GraphManager::new(&config.data_dir).await?;
            let llm = config.build_llm_client()?;
            let stats = ingest::run(&mut gm, llm.as_ref(), &path, chunk_size, overlap).await?;
            eprintln!(
                "Ingestion complete: {} files, {} chunks, {} entities, {} relationships",
                stats.files, stats.chunks, stats.entities, stats.relationships
            );
            gm.save().await?;
            eprintln!("Graph saved to {}/graph.sgsnap", config.data_dir);
        }
        Commands::Serve => {
            let gm = GraphManager::new(&config.data_dir).await?;
            mcp::serve(gm).await?;
        }
        Commands::Status => {
            let gm = GraphManager::new(&config.data_dir).await?;
            let stats = gm.stats().await?;
            println!("GraphRAG Status");
            println!("───────────────");
            println!("  Nodes: {}", stats.node_count);
            println!("  Edges: {}", stats.edge_count);
            println!("  Labels: {}", stats.labels.join(", "));
            println!("  Edge types: {}", stats.edge_types.join(", "));
            println!("  Data dir: {}", config.data_dir);
        }
        Commands::Query { cypher } => {
            let gm = GraphManager::new(&config.data_dir).await?;
            let result = gm.query(&cypher).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
