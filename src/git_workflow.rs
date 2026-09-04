// git_workflow.rs — Core Git Workflow operations for tokenectomy-git MCP
//
// Self-contained module providing branch creation, commit, PR, and status tools.
// Uses crate::redact for secret scrubbing and crate::safety for path validation.
//
// Security:
// - Branch names: sanitized to [a-zA-Z0-9_/-] only
// - Commit messages: null-byte stripped, capped at 500 chars
// - PR bodies: passed through redact_secrets() before GitHub API call
// - File paths: validated via safety::is_path_safe() before git add

use std::process::Command;
use serde::Serialize;

// ──────────────────────────── Result Types ────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct BranchResult {
    pub success: bool,
    pub branch_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitResult {
    pub success: bool,
    pub commit_hash: Option<String>,
    pub message: String,
    pub files_staged: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PullRequestResult {
    pub success: bool,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrStatusResult {
    pub success: bool,
    pub state: String,
    pub mergeable: Option<bool>,
    pub ci_status: String,
    pub message: String,
}

// ──────────────────────────── Sanitizers ────────────────────────────

/// Sanitize branch name: only allow alphanumeric, hyphens, underscores, slashes.
/// Collapses multiple hyphens and falls back to a safe default if empty.
fn sanitize_branch_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '/' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let collapsed: String = sanitized
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    let trimmed = collapsed
        .split('/')
        .filter(|s| !s.is_empty() && *s != "..")
        .collect::<Vec<_>>()
        .join("/");

    let final_name = trimmed.trim_matches(|c| c == '/' || c == '-' || c == '.');

    if final_name.is_empty() || final_name.ends_with(".lock") {
        "fix/tokenectomy-patch".to_string()
    } else {
        final_name.to_string()
    }
}

/// Sanitize commit message: strip null bytes and cap at 500 characters.
fn sanitize_commit_message(msg: &str) -> String {
    msg.chars().filter(|c| *c != '\0').take(500).collect()
}

// ──────────────────────────── Helpers ────────────────────────────

/// Checks whether the CWD is inside a git repository.
fn ensure_git_repo() -> Result<(), String> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|e| format!("git not found: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err("Not inside a git repository".to_string())
    }
}

/// Detects the GitHub owner/repo from the `origin` remote URL.
/// Supports both HTTPS and SSH remote formats.
fn detect_github_remote() -> Option<(String, String)> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if !url.contains("github.com") {
        return None;
    }

    let tail = if url.starts_with("git@") {
        url.trim_end_matches(".git").split(':').last()?.to_string()
    } else {
        url.trim_end_matches(".git")
            .split("github.com/")
            .last()?
            .to_string()
    };

    let parts: Vec<&str> = tail.split('/').collect();
    if parts.len() >= 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

/// Returns the name of the currently checked-out branch.
fn get_current_branch() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

// ──────────────────────── Tool Implementations ────────────────────────

/// Creates a new git branch and switches to it.
/// If the branch already exists, switches to it instead.
pub fn create_fix_branch(branch_name: &str) -> BranchResult {
    let safe_name = sanitize_branch_name(branch_name);

    if let Err(msg) = ensure_git_repo() {
        return BranchResult {
            success: false,
            branch_name: safe_name,
            message: msg,
        };
    }

    let has_changes = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);

    let result = Command::new("git")
        .args(["checkout", "-b", &safe_name])
        .output();

    match result {
        Ok(output) if output.status.success() => BranchResult {
            success: true,
            branch_name: safe_name.clone(),
            message: format!(
                "Branch '{}' created and checked out successfully{}",
                safe_name,
                if has_changes {
                    " (note: working directory has uncommitted changes)"
                } else {
                    ""
                }
            ),
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                match Command::new("git")
                    .args(["checkout", &safe_name])
                    .output()
                {
                    Ok(o) if o.status.success() => BranchResult {
                        success: true,
                        branch_name: safe_name.clone(),
                        message: format!(
                            "Branch '{}' already exists, switched to it",
                            safe_name
                        ),
                    },
                    _ => BranchResult {
                        success: false,
                        branch_name: safe_name,
                        message: format!(
                            "Branch already exists and could not switch: {}",
                            stderr.trim()
                        ),
                    },
                }
            } else {
                BranchResult {
                    success: false,
                    branch_name: safe_name,
                    message: format!("Failed to create branch: {}", stderr.trim()),
                }
            }
        }
        Err(e) => BranchResult {
            success: false,
            branch_name: safe_name,
            message: format!("Git command failed: {}", e),
        },
    }
}

/// Stages specific files, then commits with a message.
/// If `surgery_report` is provided it is appended (redacted) to the commit body.
pub fn commit_fix(
    files: &[String],
    message: &str,
    surgery_report: Option<&str>,
) -> CommitResult {
    let safe_message = sanitize_commit_message(message);

    let full_message = if let Some(report) = surgery_report {
        let safe_report = crate::redact::redact_secrets(report);
        format!(
            "{}\n\n--- Tokenectomy Surgery Report ---\n{}",
            safe_message, safe_report
        )
    } else {
        safe_message.clone()
    };

    let mut staged: Vec<String> = Vec::new();
    for file in files {
        if !crate::safety::is_path_safe(file) {
            return CommitResult {
                success: false,
                commit_hash: None,
                message: format!(
                    "Security Error: file '{}' is outside the current working directory",
                    file
                ),
                files_staged: staged,
            };
        }

        match Command::new("git").args(["add", file]).output() {
            Ok(output) if output.status.success() => {
                staged.push(file.clone());
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return CommitResult {
                    success: false,
                    commit_hash: None,
                    message: format!("Failed to stage '{}': {}", file, stderr.trim()),
                    files_staged: staged,
                };
            }
            Err(e) => {
                return CommitResult {
                    success: false,
                    commit_hash: None,
                    message: format!("Git add failed: {}", e),
                    files_staged: staged,
                };
            }
        }
    }

    if staged.is_empty() {
        return CommitResult {
            success: false,
            commit_hash: None,
            message: "No files were staged for commit".to_string(),
            files_staged: staged,
        };
    }

    match Command::new("git")
        .args(["commit", "-m", &full_message])
        .output()
    {
        Ok(output) if output.status.success() => {
            let hash = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                    } else {
                        None
                    }
                });

            CommitResult {
                success: true,
                commit_hash: hash,
                message: format!("Committed {} file(s) successfully", staged.len()),
                files_staged: staged,
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            CommitResult {
                success: false,
                commit_hash: None,
                message: format!("Commit failed: {}", stderr.trim()),
                files_staged: staged,
            }
        }
        Err(e) => CommitResult {
            success: false,
            commit_hash: None,
            message: format!("Git commit failed: {}", e),
            files_staged: staged,
        },
    }
}

/// Pushes the current branch to `origin` and opens a Pull Request via the GitHub REST API.
///
/// Requires env var `GITHUB_TOKEN`.
/// The PR body is scrubbed through `redact::redact_secrets` before being sent to GitHub.
pub async fn open_pull_request(
    title: &str,
    body: &str,
    base_branch: Option<&str>,
    head_branch: Option<&str>,
) -> PullRequestResult {
    let github_token = match std::env::var("GITHUB_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            return PullRequestResult {
                success: false,
                pr_number: None,
                pr_url: None,
                message: "GITHUB_TOKEN environment variable is not set.\n\
                          Set it to create PRs:\n  \
                          export GITHUB_TOKEN=your_personal_access_token"
                    .to_string(),
            }
        }
    };

    let (owner, repo) = match detect_github_remote() {
        Some(r) => r,
        None => {
            return PullRequestResult {
                success: false,
                pr_number: None,
                pr_url: None,
                message: "Could not detect GitHub remote. Ensure 'origin' points to a github.com repository."
                    .to_string(),
            }
        }
    };

    let head = head_branch
        .map(|s| s.to_string())
        .or_else(get_current_branch)
        .unwrap_or_else(|| "fix/tokenectomy-patch".to_string());

    let base = base_branch.unwrap_or("main");

    // Push branch to origin
    let push = tokio::process::Command::new("git")
        .args(["push", "-u", "origin", &head])
        .output()
        .await;

    match push {
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return PullRequestResult {
                success: false,
                pr_number: None,
                pr_url: None,
                message: format!("Failed to push branch '{}' to origin: {}", head, stderr.trim()),
            };
        }
        Err(e) => {
            return PullRequestResult {
                success: false,
                pr_number: None,
                pr_url: None,
                message: format!("Git push command failed: {}", e),
            };
        }
        _ => {}
    }

    // Open PR via GitHub API
    let safe_body = crate::redact::redact_secrets(body);

    let pr_payload = serde_json::json!({
        "title": title,
        "body": safe_body,
        "head": head,
        "base": base
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return PullRequestResult {
                success: false,
                pr_number: None,
                pr_url: None,
                message: format!("Failed to create HTTP client: {}", e),
            }
        }
    };

    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls",
        owner, repo
    );

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", github_token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tokenectomy-git")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&pr_payload)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: serde_json::Value = r.json().await.unwrap_or_default();
            let pr_number = json.get("number").and_then(|n| n.as_u64());
            let pr_url = json
                .get("html_url")
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());

            PullRequestResult {
                success: true,
                pr_number,
                pr_url: pr_url.clone(),
                message: format!(
                    "✅ Pull Request created successfully: {}",
                    pr_url.as_deref().unwrap_or("(url unavailable)")
                ),
            }
        }
        Ok(r) => {
            let status = r.status();
            let err_body = r.text().await.unwrap_or_default();
            PullRequestResult {
                success: false,
                pr_number: None,
                pr_url: None,
                message: format!("GitHub API error ({}): {}", status, err_body),
            }
        }
        Err(e) => PullRequestResult {
            success: false,
            pr_number: None,
            pr_url: None,
            message: format!("GitHub API request failed: {}", e),
        },
    }
}

/// Fetches the current state, mergeability, and CI status of an existing Pull Request.
pub async fn get_pr_status(pr_number: u64) -> PrStatusResult {
    let github_token = match std::env::var("GITHUB_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            return PrStatusResult {
                success: false,
                state: "unknown".to_string(),
                mergeable: None,
                ci_status: "unknown".to_string(),
                message: "GITHUB_TOKEN not set".to_string(),
            }
        }
    };

    let (owner, repo) = match detect_github_remote() {
        Some(r) => r,
        None => {
            return PrStatusResult {
                success: false,
                state: "unknown".to_string(),
                mergeable: None,
                ci_status: "unknown".to_string(),
                message: "Could not detect GitHub remote".to_string(),
            }
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return PrStatusResult {
                success: false,
                state: "unknown".to_string(),
                mergeable: None,
                ci_status: "unknown".to_string(),
                message: format!("HTTP client error: {}", e),
            }
        }
    };

    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}",
        owner, repo, pr_number
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", github_token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "tokenectomy-git")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let json: serde_json::Value = r.json().await.unwrap_or_default();
            let state = json
                .get("state")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string();
            let mergeable = json.get("mergeable").and_then(|m| m.as_bool());

            let head_sha = json
                .get("head")
                .and_then(|h| h.get("sha"))
                .and_then(|s| s.as_str())
                .unwrap_or("");

            let ci_status = if !head_sha.is_empty() {
                let ci_url = format!(
                    "https://api.github.com/repos/{}/{}/commits/{}/status",
                    owner, repo, head_sha
                );
                match client
                    .get(&ci_url)
                    .header("Authorization", format!("Bearer {}", github_token))
                    .header("Accept", "application/vnd.github+json")
                    .header("User-Agent", "tokenectomy-git")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .send()
                    .await
                {
                    Ok(cr) if cr.status().is_success() => {
                        let ci_json: serde_json::Value =
                            cr.json().await.unwrap_or_default();
                        ci_json
                            .get("state")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown")
                            .to_string()
                    }
                    _ => "unknown".to_string(),
                }
            } else {
                "unknown".to_string()
            };

            PrStatusResult {
                success: true,
                state: state.clone(),
                mergeable,
                ci_status: ci_status.clone(),
                message: format!(
                    "PR #{}: state={}, mergeable={:?}, CI={}",
                    pr_number, state, mergeable, ci_status
                ),
            }
        }
        Ok(r) => {
            let status = r.status();
            PrStatusResult {
                success: false,
                state: "unknown".to_string(),
                mergeable: None,
                ci_status: "unknown".to_string(),
                message: format!("GitHub API error: {}", status),
            }
        }
        Err(e) => PrStatusResult {
            success: false,
            state: "unknown".to_string(),
            mergeable: None,
            ci_status: "unknown".to_string(),
            message: format!("GitHub API request failed: {}", e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_branch_name() {
        assert_eq!(sanitize_branch_name("fix/my-bug"), "fix/my-bug");
        assert_eq!(sanitize_branch_name("fix: some bug!"), "fix-some-bug");
        assert_eq!(
            sanitize_branch_name("feat/hello world 123"),
            "feat/hello-world-123"
        );
        assert_eq!(sanitize_branch_name(""), "fix/tokenectomy-patch");
        assert_eq!(sanitize_branch_name("---"), "fix/tokenectomy-patch");
    }

    #[test]
    fn test_sanitize_commit_message() {
        let msg = "fix: resolve null pointer\0 issue";
        let sanitized = sanitize_commit_message(msg);
        assert!(!sanitized.contains('\0'));
        assert!(sanitized.contains("fix: resolve null pointer issue"));
    }

    #[test]
    fn test_sanitize_commit_message_truncates() {
        let long_msg = "a".repeat(1000);
        let sanitized = sanitize_commit_message(&long_msg);
        assert_eq!(sanitized.len(), 500);
    }

    #[test]
    fn test_adversary_fuzz_branch_name_edge_cases() {
        // Adversary attack vector: leading/trailing slashes produce illegal git ref names
        let evil_input = "/evil/branch//";
        let res = sanitize_branch_name(evil_input);
        assert!(!res.starts_with('/'), "Branch name should not start with slash: {}", res);
        assert!(!res.ends_with('/'), "Branch name should not end with slash: {}", res);
    }
}
