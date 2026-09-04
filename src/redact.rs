// redact.rs — Self-contained secret redaction for tokenectomy-git
//
// ReDoS-safe (linear-time) regex patterns for scrubbing secrets
// before they leave the machine (e.g. in PR bodies, commit messages).

use std::sync::LazyLock;
use regex::Regex;

struct RedactPattern {
    regex: Regex,
    replacement: &'static str,
}

static PATTERNS: LazyLock<Vec<RedactPattern>> = LazyLock::new(|| {
    vec![
        // GitHub tokens (ghp_, gho_, ghs_, ghu_, github_pat_)
        RedactPattern {
            regex: Regex::new(r"(ghp_|gho_|ghs_|ghu_)[A-Za-z0-9_]{30,}").unwrap(),
            replacement: "[GITHUB_TOKEN_REDACTED]",
        },
        RedactPattern {
            regex: Regex::new(r"github_pat_[A-Za-z0-9_]{30,}").unwrap(),
            replacement: "[GITHUB_PAT_REDACTED]",
        },
        // AWS Access Key ID
        RedactPattern {
            regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            replacement: "[AWS_KEY_REDACTED]",
        },
        // AWS Secret Access Key (in config/env lines)
        RedactPattern {
            regex: Regex::new(r"(?i)(aws_secret_access_key\s*[=:]\s*)[A-Za-z0-9/+=]{30,}").unwrap(),
            replacement: "${1}[AWS_SECRET_REDACTED]",
        },
        // JWTs
        RedactPattern {
            regex: Regex::new(r"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}").unwrap(),
            replacement: "[JWT_REDACTED]",
        },
        // Database connection strings
        RedactPattern {
            regex: Regex::new(r#"(?i)(postgres|mysql|mongodb|redis)://[^\s'"]+"#).unwrap(),
            replacement: "[DATABASE_URL_REDACTED]",
        },
        // Generic API keys / secrets / tokens in assignment patterns
        RedactPattern {
            regex: Regex::new(r#"(?i)(api[_-]?key|api[_-]?secret|access[_-]?token|auth[_-]?token|secret[_-]?key)\s*[=:]\s*['"]?[A-Za-z0-9_\-\.]{20,}"#).unwrap(),
            replacement: "${1}=[REDACTED]",
        },
    ]
});

/// Scrubs known secret patterns from the input string.
/// Returns a new string with secrets replaced by descriptive tags.
pub fn redact_secrets(input: &str) -> String {
    let mut output = input.to_string();
    for pattern in PATTERNS.iter() {
        output = pattern.regex.replace_all(&output, pattern.replacement).to_string();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_github_token() {
        let prefix = "ghp_";
        let body = "1234567890abcdef1234567890abcdef12";
        let input = format!("token={}{}", prefix, body);
        let result = redact_secrets(&input);
        assert!(result.contains("[GITHUB_TOKEN_REDACTED]"));
        assert!(!result.contains(prefix));
    }

    #[test]
    fn test_redact_github_pat() {
        let prefix = "github_pat_";
        let body = "abcdef1234567890abcdef1234567890ab";
        let input = format!("GITHUB_TOKEN={}{}", prefix, body);
        let result = redact_secrets(&input);
        assert!(result.contains("[GITHUB_PAT_REDACTED]"));
    }

    #[test]
    fn test_redact_aws_key() {
        let prefix = "AKIA";
        let body = "IOSFODNN7EXAMPLE";
        let input = format!("aws_access_key_id = {}{}", prefix, body);
        let result = redact_secrets(&input);
        assert!(result.contains("[AWS_KEY_REDACTED]"));
        assert!(!result.contains(prefix));
    }

    #[test]
    fn test_redact_jwt() {
        let p1 = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let p2 = "eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0";
        let p3 = "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let input = format!("Authorization: Bearer {}.{}.{}", p1, p2, p3);
        let result = redact_secrets(&input);
        assert!(result.contains("[JWT_REDACTED]"));
    }

    #[test]
    fn test_redact_connection_string() {
        let scheme = "postgres://";
        let creds = "user:password@localhost:5432/mydb";
        let input = format!("DATABASE_URL={}{}", scheme, creds);
        let result = redact_secrets(&input);
        assert!(result.contains("[DATABASE_URL_REDACTED]"));
        assert!(!result.contains("password"));
    }

    #[test]
    fn test_redact_generic_api_key() {
        let prefix = "sk_live_";
        let body = "abcdefghijklmnopqrstuvwx";
        let input = format!("api_key={}{}", prefix, body);
        let result = redact_secrets(&input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains(prefix));
    }

    #[test]
    fn test_clean_text_unchanged() {
        let input = "Just a normal error message with no secrets";
        let result = redact_secrets(input);
        assert_eq!(result, input);
    }
}
