const TOOL_POLICY: &str = "You may use the tools search_files and read_file to inspect the repository. Be judicious: start from the diff and touched file list, then request only the minimum additional context needed. Do not read the entire codebase just because more context is available.";

const TOOL_GUIDE: &str = "Tool reference (use only when needed):\n\nsearch_files\n- Purpose: Regex search across files in a directory with context lines. Use this to locate definitions, usages, TODOs, or confirm patterns.\n- Parameters:\n  - path (required): Directory to search recursively, relative to the workspace.\n  - regex (required): Rust-compatible regex pattern to match.\n  - file_pattern (optional): Glob to filter files (e.g., '*.rs').\n- Notes: Prefer narrow regexes and file patterns to avoid large outputs.\n- Example:\n  { \"path\": \"src\", \"regex\": \"fn\\s+create_user_prompt\", \"file_pattern\": \"*.rs\" }\n\nread_file\n- Purpose: Read a file and return line-numbered contents. Use this to inspect specific files or ranges once you know what you need.\n- Parameters:\n  - path (required): Path to file, relative to the workspace.\n  - offset (optional): 1-based line offset to start reading (default 1).\n  - limit (optional): Maximum number of lines to return (default 2000).\n- Notes: Use offset/limit to read only the section you need; avoid full-file reads unless the file is small.\n- Example:\n  { \"path\": \"src/main.rs\", \"offset\": 1, \"limit\": 200 }";

pub fn get_system_prompt() -> String {
    let base = include_str!("../prompt.txt");
    format!("{}\n\n{}\n\n{}", TOOL_POLICY, TOOL_GUIDE, base)
}

pub fn create_user_prompt(
    diff: &str,
    files_changed: &[String],
    additional_prompt: Option<&str>,
) -> String {
    let mut user_prompt = String::from(
        "Below is a git diff and the list of touched files. Use search_files and read_file if you need more context.\n",
    );

    if let Some(additional) = additional_prompt {
        if !additional.trim().is_empty() {
            user_prompt.push_str(additional);
            user_prompt.push('\n');
        }
    }

    user_prompt.push_str("\nDIFF BEGINS:\n");
    user_prompt.push_str(diff);
    user_prompt.push_str("\nDIFF ENDS\n\nTOUCHED FILES:\n");

    if files_changed.is_empty() {
        user_prompt.push_str("(none)\n");
    } else {
        for file in files_changed {
            user_prompt.push_str(file);
            user_prompt.push('\n');
        }
    }

    user_prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_user_prompt_includes_diff_and_files() {
        let diff = "diff --git a/a b/a\n+hi\n";
        let files = vec!["src/main.rs".to_string()];
        let prompt = create_user_prompt(diff, &files, Some("Extra context"));

        assert!(prompt.contains("DIFF BEGINS"));
        assert!(prompt.contains(diff));
        assert!(prompt.contains("TOUCHED FILES"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("Extra context"));
    }
}
