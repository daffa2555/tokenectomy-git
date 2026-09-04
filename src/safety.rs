// safety.rs — Path validation for tokenectomy-git
//
// Ensures file operations are restricted to the current working directory.
// Prevents path traversal attacks via symlinks or ../.. patterns.

/// Validates that a file path resolves to a location within the current working directory.
/// Rejects paths that:
/// - Are outside the CWD after canonicalization
/// - Don't exist (prevents traversal via non-existent intermediate paths)
pub fn is_path_safe(file_path: &str) -> bool {
    if let Ok(current_dir) = std::env::current_dir() {
        let current_dir = current_dir.canonicalize().unwrap_or(current_dir);
        let path = std::path::Path::new(file_path);
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            current_dir.join(path)
        };
        // Canonicalize to resolve symlinks
        if let Ok(resolved) = abs_path.canonicalize() {
            resolved.starts_with(&current_dir)
        } else {
            false // File doesn't exist = reject
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_outside_cwd_rejected() {
        assert!(!is_path_safe("/etc/passwd"));
        assert!(!is_path_safe("../../etc/passwd"));
    }

    #[test]
    fn test_nonexistent_file_rejected() {
        assert!(!is_path_safe("nonexistent_file_that_does_not_exist_xyz.rs"));
    }
}
