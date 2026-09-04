# Tokenectomy Git 🔀

[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-blue)](https://modelcontextprotocol.io)

**Standalone MCP server** that gives AI coding assistants the ability to autonomously create branches, commit fixes, open Pull Requests, and monitor CI status on GitHub.

Designed to pair with **[Tokenectomy](https://github.com/daffa2555/Tokenectomy)** for end-to-end autonomous bug fixing.

```
                    ┌──────────────────┐
  Error Log         │   TOKENECTOMY    │      Clean Context
  (38K tokens) ───► │   🕵️ Scrub       │ ───► (2K tokens) ───► LLM ───► Patch
                    └──────────────────┘                              │
                                                                     ▼
                    ┌──────────────────┐                      ┌──────────────┐
  GitHub PR    ◄─── │ TOKENECTOMY GIT  │ ◄─────────────────── │  AI Patch    │
  (automated)       │   🔀 Workflow    │                      │  Verified ✅  │
                    │                  │                      └──────────────┘
                    │  🌿 Branch       │
                    │  📝 Commit       │
                    │  🔀 Pull Request │
                    │  📊 CI Status    │
                    └──────────────────┘
```

---

## ✨ Features

| Tool | Description |
|---|---|
| **`create_fix_branch`** | Creates a new git branch and switches to it. Branch names are sanitized for safety. |
| **`commit_fix`** | Stages files and commits with a message. Optionally embeds a Tokenectomy Surgery Report in the commit body. |
| **`open_pull_request`** | Pushes branch to origin and opens a GitHub PR via REST API. PR body is scrubbed of secrets before sending. |
| **`get_pr_status`** | Checks PR state (open/closed/merged), mergeability, and CI check status. |

---

## 🔒 Security

- **Path Validation** — All file paths in `commit_fix` are canonicalized and verified to be within the current working directory.
- **Branch Name Sanitization** — Only alphanumeric characters, hyphens, underscores, and slashes are allowed.
- **Secret Redaction** — PR bodies and commit messages pass through a ReDoS-safe regex engine that strips GitHub tokens, AWS keys, JWTs, database URLs, and generic API keys.
- **Commit Message Cap** — Messages are stripped of null bytes and capped at 500 characters.
- **Stdin DoS Protection** — MCP reads are limited to 50MB via `.take()`.

---

## 📦 Installation

### 🦀 Build from Source

```bash
git clone https://github.com/daffa2555/tokenectomy-git.git
cd tokenectomy-git
cargo build --release
sudo cp target/release/tokenectomy-git /usr/local/bin/tkmy-git
```

### ⚙️ Install via Smithery (for Claude Desktop)

```bash
npx -y @smithery/cli install tokenectomy-git --client claude
```

---

## 🚀 Usage

### Run as MCP Server

```bash
tkmy-git --mcp
```

---

## 🔌 MCP Client Configuration

### Claude Desktop / Cursor

Add to your MCP config (`claude_desktop_config.json` or Cursor settings):

```json
{
  "mcpServers": {
    "tokenectomy-git": {
      "command": "tkmy-git",
      "args": ["--mcp"]
    }
  }
}
```

### Google Antigravity

Add to `.gemini/settings.json`:

```json
{
  "mcpServers": {
    "tokenectomy-git": {
      "command": "tkmy-git",
      "args": ["--mcp"]
    }
  }
}
```

---

## 🔑 Environment Variables

| Variable | Required | Description |
|---|---|---|
| `GITHUB_TOKEN` | For PR operations | GitHub Personal Access Token with `contents:write` + `pull_requests:write` permissions |

```bash
export GITHUB_TOKEN=your_personal_access_token
```

---

## 🔗 Integration Ecosystem (Works with OSS & Pro)

`tokenectomy-git` is designed to be modular and **100% decoupled from any specific Tokenectomy edition**. It works seamlessly whether users run the free open-source version or the enterprise Pro version:

### 🆓 Scenario A: 100% Free & Open-Source Stack (MIT)
For developers using **Tokenectomy OSS**:
1. `Tokenectomy OSS` scrubs 90%+ framework noise from logs & redacts credentials.
2. The AI assistant diagnoses the issue and produces a fix patch.
3. `Tokenectomy Git` creates an isolated branch, commits the fix, and opens a GitHub Pull Request autonomously.

```json
{
  "mcpServers": {
    "tokenectomy": {
      "command": "tkmy",
      "args": ["--mcp"]
    },
    "tokenectomy-git": {
      "command": "tkmy-git",
      "args": ["--mcp"]
    }
  }
}
```

### 👑 Scenario B: Enterprise Autonomous Stack (Pro)
For developers running **Tokenectomy Pro**:
1. `Tokenectomy Pro` performs deep True Ectomy surgery (up to 99% token reduction), and runs DB & Docker diagnostics.
2. The AI assistant generates a code patch.
3. `Tokenectomy Pro` applies the patch in-memory, heals AST syntax errors, runs the project test suite (`cargo test`, `npm test`, `pytest`), and rolls back if anything breaks.
4. Once tests are verified green, `Tokenectomy Git` creates a branch, commits the changes (with the verified *Surgery Report* embedded), and publishes the Pull Request to GitHub!

```json
{
  "mcpServers": {
    "tokenectomy-pro": {
      "command": "tkmy",
      "args": ["--mcp"]
    },
    "tokenectomy-git": {
      "command": "tkmy-git",
      "args": ["--mcp"]
    }
  }
}
```

---

## 📄 License

[MIT](LICENSE) — Copyright (c) 2025 Daffa Anan
