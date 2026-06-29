//! Shared helpers for the golden-case test files.

use std::path::Path;

/// One golden case: a name, an input document, and the expected output.
pub struct Case {
    pub name: String,
    pub input: String,
    pub expected: String,
}

/// Load golden cases from a ` | `-separated fixture file.
///
/// Blank lines and lines beginning with `#` are ignored. Each remaining line
/// has the form `name | input | expected`.
pub fn load_cases(path: impl AsRef<Path>) -> Vec<Case> {
    let text = std::fs::read_to_string(path.as_ref())
        .unwrap_or_else(|e| panic!("read {:?}: {e}", path.as_ref()));
    let mut cases = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        assert_eq!(parts.len(), 3, "malformed fixture line: {line}");
        cases.push(Case {
            name: parts[0].to_string(),
            input: parts[1].to_string(),
            expected: parts[2].to_string(),
        });
    }
    cases
}

/// Absolute path to a fixture file under `tests/golden`.
pub fn golden_path(name: &str) -> String {
    format!("{}/tests/golden/{}", env!("CARGO_MANIFEST_DIR"), name)
}
