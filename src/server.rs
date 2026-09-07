// src/server.rs — Tokenectomy Git MCP Server
//
// Standalone JSON-RPC 2.0 over stdio MCP server providing
// Git workflow automation tools for AI coding assistants.
//
// Tools: create_fix_branch, commit_fix, open_pull_request, get_pr_status

use crate::git_workflow;

use clap::Parser;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

#[derive(Parser)]
#[command(name = "tokenectomy-git", version, about = "Git Workflow MCP Server — Autonomous branch, commit, and PR creation for AI coding assistants.")]
pub struct Cli {
    /// Run as MCP server (JSON-RPC over stdio)
    #[arg(long, default_value_t = true)]
    pub mcp: bool,
}

fn error_response(id: Option<Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

pub async fn run_mcp_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut handle = stdin.lock();

    loop {
        let mut line = String::new();
        // Limit read to 50MB to prevent DoS
        let n = handle.by_ref().take(50 * 1024 * 1024).read_line(&mut line)?;
        if n == 0 {
            break; // EOF
        }

        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                let err = error_response(None, -32700, "Parse error or line too long");
                writeln!(stdout, "{}", serde_json::to_string(&err)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let id = req.get("id").cloned();
        let method = match req.get("method").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => {
                let err = error_response(id, -32600, "Invalid Request: missing method");
                writeln!(stdout, "{}", serde_json::to_string(&err)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = match method {
            "initialize" => {
                Some(success_response(id.unwrap_or(Value::Null), json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "tokenectomy-git",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })))
            }
            "notifications/initialized" => None,
            "ping" => Some(success_response(id.unwrap_or(Value::Null), json!({}))),
            "tools/list" => {
                Some(success_response(id.unwrap_or(Value::Null), json!({
                    "tools": [
                        {
                            "name": "create_fix_branch",
                            "description": "Creates a new git branch for the fix and switches to it. If the branch already exists, switches to it. Branch names are sanitized for safety.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "branch_name": { "type": "string", "description": "Name for the fix branch (e.g. 'fix/resolve-null-pointer'). Will be sanitized." }
                                },
                                "required": ["branch_name"]
                            }
                        },
                        {
                            "name": "commit_fix",
                            "description": "Stages specified files and commits them with a message. Optionally appends a Tokenectomy Surgery Report to the commit body. All file paths are validated to be within the workspace.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "files": {
                                        "type": "array",
                                        "description": "List of file paths to stage and commit",
                                        "items": { "type": "string" }
                                    },
                                    "message": { "type": "string", "description": "Commit message (e.g. 'fix: resolve null pointer in handler')" },
                                    "surgery_report": { "type": "string", "description": "Optional Tokenectomy Surgery Report to include in the commit body" }
                                },
                                "required": ["files", "message"]
                            }
                        },
                        {
                            "name": "open_pull_request",
                            "description": "Pushes the current branch to origin and opens a GitHub Pull Request. Requires GITHUB_TOKEN env var. PR body is scrubbed of secrets before sending. Auto-detects owner/repo from git remote.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "title": { "type": "string", "description": "PR title (e.g. 'fix: resolve null pointer in /api/users handler')" },
                                    "body": { "type": "string", "description": "PR description body. Will be redacted of any secrets before submission." },
                                    "base_branch": { "type": "string", "description": "Target branch to merge into (default: 'main')" },
                                    "head_branch": { "type": "string", "description": "Source branch (default: current branch)" }
                                },
                                "required": ["title", "body"]
                            }
                        },
                        {
                            "name": "get_pr_status",
                            "description": "Checks the status of a GitHub Pull Request including state (open/closed/merged), mergeability, and CI check status. Requires GITHUB_TOKEN env var.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "pr_number": { "type": "integer", "description": "The Pull Request number to check" }
                                },
                                "required": ["pr_number"]
                            }
                        }
                    ]
                })))
            }
            "tools/call" => {
                let params = req.get("params");
                let name = params.and_then(|p| p.get("name")).and_then(|n| n.as_str());
                let args = params.and_then(|p| p.get("arguments"));

                if name.is_none() || args.is_none() {
                    Some(error_response(id, -32602, "Invalid params"))
                } else {
                    let name = name.unwrap();
                    let args = args.unwrap();

                    match name {
                        "create_fix_branch" => {
                            let branch_name = args.get("branch_name").and_then(|b| b.as_str());
                            if let Some(bname) = branch_name {
                                let result = git_workflow::create_fix_branch(bname);
                                let json_res = serde_json::to_value(&result).unwrap_or(json!({ "error": "Serialization failed" }));
                                Some(success_response(id.unwrap_or(Value::Null), json!({
                                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&json_res).unwrap_or_default() }],
                                    "branch": json_res
                                })))
                            } else {
                                Some(error_response(id, -32602, "Missing 'branch_name' argument"))
                            }
                        }
                        "commit_fix" => {
                            let files_val = args.get("files").and_then(|f| f.as_array());
                            let message = args.get("message").and_then(|m| m.as_str());
                            let surgery_report = args.get("surgery_report").and_then(|s| s.as_str());

                            if let (Some(files_arr), Some(msg)) = (files_val, message) {
                                let files: Vec<String> = files_arr
                                    .iter()
                                    .filter_map(|f| f.as_str().map(|s| s.to_string()))
                                    .collect();
                                let result = git_workflow::commit_fix(&files, msg, surgery_report);
                                let json_res = serde_json::to_value(&result).unwrap_or(json!({ "error": "Serialization failed" }));
                                Some(success_response(id.unwrap_or(Value::Null), json!({
                                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&json_res).unwrap_or_default() }],
                                    "commit": json_res
                                })))
                            } else {
                                Some(error_response(id, -32602, "Missing 'files' (array) or 'message' argument"))
                            }
                        }
                        "open_pull_request" => {
                            let title = args.get("title").and_then(|t| t.as_str());
                            let body = args.get("body").and_then(|b| b.as_str());
                            let base = args.get("base_branch").and_then(|b| b.as_str());
                            let head = args.get("head_branch").and_then(|h| h.as_str());

                            if let (Some(t), Some(b)) = (title, body) {
                                let result = git_workflow::open_pull_request(t, b, base, head).await;
                                let json_res = serde_json::to_value(&result).unwrap_or(json!({ "error": "Serialization failed" }));
                                Some(success_response(id.unwrap_or(Value::Null), json!({
                                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&json_res).unwrap_or_default() }],
                                    "pull_request": json_res
                                })))
                            } else {
                                Some(error_response(id, -32602, "Missing 'title' or 'body' argument"))
                            }
                        }
                        "get_pr_status" => {
                            let pr_number = args.get("pr_number").and_then(|n| n.as_u64());
                            if let Some(num) = pr_number {
                                let result = git_workflow::get_pr_status(num).await;
                                let json_res = serde_json::to_value(&result).unwrap_or(json!({ "error": "Serialization failed" }));
                                Some(success_response(id.unwrap_or(Value::Null), json!({
                                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&json_res).unwrap_or_default() }],
                                    "status": json_res
                                })))
                            } else {
                                Some(error_response(id, -32602, "Missing 'pr_number' argument"))
                            }
                        }
                        _ => {
                            Some(error_response(id, -32601, &format!("Tool '{}' not found", name)))
                        }
                    }
                }
            }
            _ => {
                if id.is_some() {
                    Some(error_response(id, -32601, "Method not found"))
                } else {
                    None
                }
            }
        };

        if let Some(res) = response {
            writeln!(stdout, "{}", serde_json::to_string(&res)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}
