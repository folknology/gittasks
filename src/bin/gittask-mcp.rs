//! gittask MCP server

use clap::Parser;

/// gittask MCP server - Git-versioned task management
#[derive(Parser, Debug)]
#[command(name = "gittask-mcp")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use global tasks directory (~/.tasks) instead of project-local
    #[arg(short, long)]
    global: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    gittask::mcp::run_mcp_server(args.global).await
}
