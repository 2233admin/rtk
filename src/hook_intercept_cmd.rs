use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::config::{Config, HookInterceptConfig};
use crate::filter::{self, FilterLevel, Language};
use crate::tracking;

// do not filter (config/data files).
const SKIP_EXTENSIONS: &[&str] = &[
    "json", "toml", "yaml", "yml", "xml", "lock", "env", "ini", "csv", "sql", "md",
];

#[derive(Debug)]
enum ReadAction {
    Skip,
    Intercept {
        filtered: String,
        original_content: String,
    },
}

// entrypoint
pub fn run() -> Result<()> {
    let config = Config::load().map(|c| c.hook_intercept).unwrap_or_default();

    let input = read_stdin()?;
    let parsed: serde_json::Value =
        serde_json::from_str(&input).context("Failed to parse hook JSON from stdin")?;

    let tool_name = parsed
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let tool_input = parsed
        .get("tool_input")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    match tool_name {
        "Read" if config.read_enabled => handle_read(&tool_input, &config),
        "Grep" if config.grep_enabled => handle_grep(&tool_input, &config),
        _ => {
            std::process::exit(1);
        }
    }
}

fn read_stdin() -> Result<String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut buf)
        .context("Failed to read from stdin")?;
    Ok(buf)
}

fn should_intercept_read(
    tool_input: &serde_json::Value,
    file_path: &str,
    config: &HookInterceptConfig,
) -> ReadAction {
    // Guard: partial reads (offset/limit set) are already bounded
    if tool_input.get("offset").and_then(|v| v.as_u64()).is_some()
        || tool_input.get("limit").and_then(|v| v.as_u64()).is_some()
    {
        return ReadAction::Skip;
    }

    let path = Path::new(file_path);

    // Guard: skip config/data file extensions
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if SKIP_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            return ReadAction::Skip;
        }
    }

    // Guard: skip large files (check size before reading into memory)
    match fs::metadata(path) {
        Ok(meta) if meta.len() as usize > config.read_max_file_size => {
            return ReadAction::Skip;
        }
        Ok(_) => {}
        Err(_) => return ReadAction::Skip,
    }

    // Guard: file must be readable as UTF-8
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return ReadAction::Skip,
    };

    // Guard: skip small files
    let line_count = content.lines().count();
    if line_count < config.read_min_file_lines {
        return ReadAction::Skip;
    }

    let lang = path
        .extension()
        .and_then(|e| e.to_str())
        .map(Language::from_extension)
        .unwrap_or(Language::Unknown);

    // Guard: skip unknown language with no comment patterns
    if lang == Language::Unknown {
        let patterns = lang.comment_patterns();
        if patterns.line.is_none() && patterns.block_start.is_none() {
            return ReadAction::Skip;
        }
    }

    // Apply MinimalFilter (same as rtk read)
    let filter_strategy = filter::get_filter(FilterLevel::Minimal);
    let filtered = filter_strategy.filter(&content, &lang);

    // Guard: skip if savings < 10%
    if filtered.len() as f64 >= content.len() as f64 * 0.9 {
        return ReadAction::Skip;
    }

    ReadAction::Intercept {
        filtered,
        original_content: content,
    }
}

fn build_grep_update(
    tool_input: &serde_json::Value,
    config: &HookInterceptConfig,
) -> Option<serde_json::Value> {
    if let Some(limit) = tool_input.get("head_limit").and_then(|v| v.as_u64()) {
        if limit > 0 {
            return None;
        }
    }

    let mut updated = tool_input.clone();
    updated["head_limit"] = serde_json::json!(config.grep_default_head_limit);
    Some(updated)
}

// cross-platform temp file path for filtered Read output.
fn build_temp_path(file_path: &str, ext: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    let hash = hasher.finish();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("rtk-read-{}-{:x}.{}", pid, hash, ext))
}

fn handle_read(tool_input: &serde_json::Value, config: &HookInterceptConfig) -> Result<()> {
    let file_path = match tool_input.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => std::process::exit(0), // pass through
    };

    let (filtered, original_content) = match should_intercept_read(tool_input, file_path, config) {
        ReadAction::Skip => std::process::exit(0),
        ReadAction::Intercept {
            filtered,
            original_content,
        } => (filtered, original_content),
    };

    // Write temp file with deterministic name
    let path = Path::new(file_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
    let temp_path = build_temp_path(file_path, ext);

    fs::write(&temp_path, &filtered)
        .with_context(|| format!("Failed to write temp file: {}", temp_path.display()))?;

    emit_response(
        &serde_json::json!({ "file_path": temp_path.to_string_lossy() }),
        &format!(
            "RTK MinimalFilter: {} -> {} bytes ({:.0}% savings)",
            original_content.len(),
            filtered.len(),
            (1.0 - filtered.len() as f64 / original_content.len() as f64) * 100.0
        ),
    );

    // Track savings
    let timer = tracking::TimedExecution::start();
    timer.track(
        &format!("read {}", file_path),
        "rtk hook-intercept read",
        &original_content,
        &filtered,
    );

    Ok(())
}

fn handle_grep(tool_input: &serde_json::Value, config: &HookInterceptConfig) -> Result<()> {
    match build_grep_update(tool_input, config) {
        None => std::process::exit(0),
        Some(updated) => {
            emit_response(
                &updated,
                &format!(
                    "RTK: injected head_limit={}",
                    config.grep_default_head_limit
                ),
            );
        }
    }

    Ok(())
}

/// Print hookSpecificOutput JSON to stdout.
fn emit_response(updated_input: &serde_json::Value, reason: &str) {
    let response = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "permissionDecisionReason": reason,
            "updatedInput": updated_input
        }
    });
    println!("{}", response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;

    fn default_config() -> HookInterceptConfig {
        HookInterceptConfig::default()
    }

    // --- should_intercept_read tests ---

    #[test]
    fn test_should_skip_partial_reads() {
        let config = default_config();
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        for i in 0..150 {
            writeln!(file, "// comment {}", i).unwrap();
            writeln!(file, "let x_{} = {};", i, i).unwrap();
        }
        let path_str = file.path().to_str().unwrap().to_string();

        // With offset set
        let input_with_offset = serde_json::json!({
            "file_path": &path_str,
            "offset": 10
        });
        assert!(matches!(
            should_intercept_read(&input_with_offset, &path_str, &config),
            ReadAction::Skip
        ));

        // With limit set
        let input_with_limit = serde_json::json!({
            "file_path": &path_str,
            "limit": 50
        });
        assert!(matches!(
            should_intercept_read(&input_with_limit, &path_str, &config),
            ReadAction::Skip
        ));
    }

    #[test]
    fn test_should_skip_config_extensions() {
        let config = default_config();
        let input = serde_json::json!({ "file_path": "/some/file.json" });

        // .json should be skipped (even if file doesn't exist, extension check comes first)
        assert!(matches!(
            should_intercept_read(&input, "/some/file.json", &config),
            ReadAction::Skip
        ));

        let input_toml = serde_json::json!({ "file_path": "/some/file.toml" });
        assert!(matches!(
            should_intercept_read(&input_toml, "/some/file.toml", &config),
            ReadAction::Skip
        ));
    }

    #[test]
    fn test_should_skip_small_files() {
        let config = default_config(); // min_file_lines = 100
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        // Write only 50 lines — below the 100-line threshold
        for i in 0..25 {
            writeln!(file, "// comment {}", i).unwrap();
            writeln!(file, "let x_{} = {};", i, i).unwrap();
        }
        let path_str = file.path().to_str().unwrap().to_string();
        let input = serde_json::json!({ "file_path": &path_str });

        assert!(matches!(
            should_intercept_read(&input, &path_str, &config),
            ReadAction::Skip
        ));
    }

    #[test]
    fn test_should_skip_large_files() {
        let config = HookInterceptConfig {
            read_max_file_size: 100, // Very small limit for testing
            ..default_config()
        };
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        // Write more than 100 bytes
        for i in 0..200 {
            writeln!(file, "// comment line number {}", i).unwrap();
        }
        let path_str = file.path().to_str().unwrap().to_string();
        let input = serde_json::json!({ "file_path": &path_str });

        assert!(matches!(
            should_intercept_read(&input, &path_str, &config),
            ReadAction::Skip
        ));
    }

    #[test]
    fn test_should_intercept_rust_file() {
        let config = default_config();
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        // 150 lines with ~50% comments — should get good savings
        for i in 0..75 {
            writeln!(file, "// This is a detailed comment about line {}", i).unwrap();
            writeln!(file, "let var_{} = {};", i, i).unwrap();
        }
        let path_str = file.path().to_str().unwrap().to_string();
        let input = serde_json::json!({ "file_path": &path_str });

        match should_intercept_read(&input, &path_str, &config) {
            ReadAction::Intercept {
                filtered,
                original_content,
            } => {
                // Filtered should be shorter
                assert!(
                    filtered.len() < original_content.len(),
                    "Filtered ({}) should be shorter than original ({})",
                    filtered.len(),
                    original_content.len()
                );
                // Comments should be stripped
                assert!(!filtered.contains("// This is a detailed comment"));
                // Code should remain
                assert!(filtered.contains("let var_0"));
            }
            ReadAction::Skip => panic!("Expected Intercept for large commented .rs file"),
        }
    }

    #[test]
    fn test_should_skip_low_savings() {
        let config = default_config();
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        // 150 lines with almost no comments — savings will be <10%
        for i in 0..150 {
            writeln!(file, "let var_{} = {};", i, i).unwrap();
        }
        let path_str = file.path().to_str().unwrap().to_string();
        let input = serde_json::json!({ "file_path": &path_str });

        assert!(matches!(
            should_intercept_read(&input, &path_str, &config),
            ReadAction::Skip
        ));
    }

    // --- build_grep_update tests ---

    #[test]
    fn test_build_grep_update_injects_limit() {
        let config = default_config();
        let input = serde_json::json!({
            "pattern": "fn main",
            "path": "src/"
        });

        let updated = build_grep_update(&input, &config).expect("Should inject head_limit");
        assert_eq!(updated["head_limit"], config.grep_default_head_limit);
        // Original fields preserved
        assert_eq!(updated["pattern"], "fn main");
        assert_eq!(updated["path"], "src/");
    }

    #[test]
    fn test_build_grep_update_preserves_explicit() {
        let config = default_config();
        let input = serde_json::json!({
            "pattern": "fn main",
            "path": "src/",
            "head_limit": 10
        });

        assert!(
            build_grep_update(&input, &config).is_none(),
            "Should return None when head_limit already set"
        );
    }

    // --- build_temp_path tests ---

    #[test]
    fn test_temp_path_cross_platform() {
        let temp_path = build_temp_path("/some/file.rs", "rs");

        // Must use the system temp dir, not hardcoded /tmp/
        assert!(
            temp_path.starts_with(std::env::temp_dir()),
            "Temp path {:?} should start with system temp dir {:?}",
            temp_path,
            std::env::temp_dir()
        );

        // Must preserve extension
        assert_eq!(temp_path.extension().and_then(|e| e.to_str()), Some("rs"));

        // Must contain rtk-read prefix
        let file_name = temp_path.file_name().unwrap().to_str().unwrap();
        assert!(
            file_name.starts_with("rtk-read-"),
            "File name {:?} should start with 'rtk-read-'",
            file_name
        );
    }

    // --- Retained good tests ---

    #[test]
    fn test_read_filter_matches_rtk_read() {
        // Create a Rust file with comments
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        writeln!(
            file,
            r#"// This is a comment
// Another comment
// Third comment
fn main() {{
    println!("Hello");
}}

// More comments
// And more
fn helper() {{
    let x = 1;
}}"#
        )
        .unwrap();

        // Apply the same filter that rtk read uses
        let content = fs::read_to_string(file.path()).unwrap();
        let lang = Language::Rust;
        let filter = filter::get_filter(FilterLevel::Minimal);
        let filtered = filter.filter(&content, &lang);

        // Comments should be stripped
        assert!(!filtered.contains("// This is a comment"));
        // Code should remain
        assert!(filtered.contains("fn main()"));
    }

    #[test]
    fn test_read_token_savings() {
        // Create a Rust file with lots of comments (should get good savings)
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        for i in 0..120 {
            writeln!(file, "// Comment explaining something about line {}", i).unwrap();
            writeln!(file, "// Another comment for line {}", i).unwrap();
            writeln!(file, "let var_{} = {};", i, i * 2).unwrap();
        }

        let content = fs::read_to_string(file.path()).unwrap();
        let lang = Language::Rust;
        let filter_strategy = filter::get_filter(FilterLevel::Minimal);
        let filtered = filter_strategy.filter(&content, &lang);

        let savings = (1.0 - filtered.len() as f64 / content.len() as f64) * 100.0;
        assert!(
            savings >= 60.0,
            "Expected >= 60% savings on commented Rust code, got {:.1}%",
            savings
        );
    }

    #[test]
    fn test_output_json_format() {
        let updated_input = serde_json::json!({ "file_path": "/tmp/test.rs" });
        let reason = "RTK test";

        let response = serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "allow",
                "permissionDecisionReason": reason,
                "updatedInput": updated_input
            }
        });

        // Validate structure
        let hook_output = response.get("hookSpecificOutput").unwrap();
        assert_eq!(
            hook_output.get("hookEventName").unwrap().as_str().unwrap(),
            "PreToolUse"
        );
        assert_eq!(
            hook_output
                .get("permissionDecision")
                .unwrap()
                .as_str()
                .unwrap(),
            "allow"
        );
        assert!(hook_output.get("updatedInput").is_some());
    }
}
