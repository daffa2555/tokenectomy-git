// src/bin/tkmy-git.rs — Ergonomic short alias for Tokenectomy Git MCP Server
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _args = tokenectomy_git::Cli::parse();
    tokenectomy_git::run_mcp_server().await
}
