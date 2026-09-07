//! # Tokenectomy Git 🔀
//!
//! **Autonomous Git Workflow & GitHub PR Creation MCP Server for AI Coding Agents.**
//!
//! Standalone Model Context Protocol (MCP) server providing automated Git operations
//! over JSON-RPC 2.0 stdio:
//! - `create_fix_branch`: Safe branch creation with sanitization.
//! - `commit_fix`: Safe git commit with Tokenectomy Surgery Report attachment.
//! - `open_pull_request`: Automated GitHub PR creation with secret redaction.
//! - `get_pr_status`: Real-time CI check monitoring and mergeability verification.

pub mod git_workflow;
pub mod redact;
pub mod safety;
pub mod server;

pub use server::{run_mcp_server, Cli};
