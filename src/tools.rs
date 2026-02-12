use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::client::dto::{Tool, ToolFunctionDef};

const DEFAULT_READ_LIMIT: usize = 2000;
const MAX_READ_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;
const MAX_SEARCH_MATCHES: usize = 50;
const SEARCH_CONTEXT_LINES: usize = 1;

#[derive(Debug, Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SearchFilesArgs {
    pub path: String,
    pub regex: String,
    pub file_pattern: Option<String>,
}

pub fn tool_definitions() -> Vec<Tool> {
    vec![read_file_tool(), search_files_tool()]
}

fn read_file_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: ToolFunctionDef {
            name: "read_file".to_string(),
            description: "Read a file and return its contents with line numbers. Use offset/limit to read slices. Avoid reading large files unless necessary.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read, relative to the workspace"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "1-based line offset to start reading from (default 1)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to return (default 2000)"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    }
}

fn search_files_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: ToolFunctionDef {
            name: "search_files".to_string(),
            description: "Search for a regex across files in a directory. Returns matching lines with context and line numbers. Use a file_pattern to narrow the search.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory to search recursively, relative to the workspace"
                    },
                    "regex": {
                        "type": "string",
                        "description": "Rust-compatible regex pattern to match"
                    },
                    "file_pattern": {
                        "type": ["string", "null"],
                        "description": "Optional glob to limit which files are searched (e.g., *.rs)"
                    }
                },
                "required": ["path", "regex"],
                "additionalProperties": false
            }),
        },
    }
}

pub fn handle_tool_call(name: &str, arguments: &str) -> String {
    match name {
        "read_file" => match serde_json::from_str::<ReadFileArgs>(arguments) {
            Ok(args) => read_file(&args),
            Err(err) => format_tool_error("read_file", &format!("Invalid arguments: {}", err)),
        },
        "search_files" => match serde_json::from_str::<SearchFilesArgs>(arguments) {
            Ok(args) => search_files(&args),
            Err(err) => format_tool_error("search_files", &format!("Invalid arguments: {}", err)),
        },
        _ => format_tool_error(name, "Unknown tool name"),
    }
}

fn read_file(args: &ReadFileArgs) -> String {
    let path = Path::new(&args.path);
    let contents = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            return format_tool_error(
                "read_file",
                &format!("Failed to read {}: {}", path.display(), err),
            )
        }
    };

    let offset = args.offset.unwrap_or(1).max(1);
    let limit = args.limit.unwrap_or(DEFAULT_READ_LIMIT).min(MAX_READ_LIMIT);

    let lines: Vec<&str> = contents.lines().collect();
    if lines.is_empty() {
        return format_file_output(path, &[]);
    }

    let start_index = offset.saturating_sub(1);
    if start_index >= lines.len() {
        return format_file_output(path, &[]);
    }

    let end_index = (start_index + limit).min(lines.len());
    let mut numbered_lines = Vec::new();
    for (i, line) in lines[start_index..end_index].iter().enumerate() {
        let line_number = offset + i;
        numbered_lines.push(format!("{:>6}| {}", line_number, truncate_line(line)));
    }

    format_file_output(path, &numbered_lines)
}

fn search_files(args: &SearchFilesArgs) -> String {
    let root = Path::new(&args.path);
    if !root.exists() {
        return format_tool_error(
            "search_files",
            &format!("Search path does not exist: {}", root.display()),
        );
    }
    if !root.is_dir() {
        return format_tool_error(
            "search_files",
            &format!("Search path is not a directory: {}", root.display()),
        );
    }

    let regex = match Regex::new(&args.regex) {
        Ok(re) => re,
        Err(err) => return format_tool_error("search_files", &format!("Invalid regex: {}", err)),
    };

    let globset = match build_globset(args.file_pattern.as_deref()) {
        Ok(value) => value,
        Err(err) => return format_tool_error("search_files", &err),
    };

    let mut results = Vec::new();
    let mut total_matches = 0;

    let walker = WalkDir::new(root).follow_links(false).into_iter();
    for entry in walker.filter_entry(|e| !is_ignored_dir(e.path())) {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        if let Some(ref set) = globset {
            if !set.is_match(entry.path()) {
                continue;
            }
        }

        let content = match fs::read_to_string(entry.path()) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let lines: Vec<&str> = content.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !regex.is_match(line) {
                continue;
            }

            total_matches += 1;
            if total_matches > MAX_SEARCH_MATCHES {
                break;
            }

            let line_number = index + 1;
            let before = index.saturating_sub(SEARCH_CONTEXT_LINES);
            let after = (index + SEARCH_CONTEXT_LINES + 1).min(lines.len());
            let context = lines[before..after]
                .iter()
                .enumerate()
                .map(|(offset, line)| {
                    let current_line = before + offset + 1;
                    let marker = if current_line == line_number {
                        '>'
                    } else {
                        ' '
                    };
                    format!("{} {:>6}| {}", marker, current_line, truncate_line(line))
                })
                .collect::<Vec<String>>();

            results.push(SearchMatch {
                path: entry.path().to_path_buf(),
                line_number,
                context,
            });
        }

        if total_matches >= MAX_SEARCH_MATCHES {
            break;
        }
    }

    format_search_results(
        root,
        &args.regex,
        args.file_pattern.as_deref(),
        &results,
        total_matches,
    )
}

fn format_file_output(path: &Path, lines: &[String]) -> String {
    let mut output = format!("FILE: {}\n", path.display());
    if lines.is_empty() {
        output.push_str("(no lines in range)\n");
        return output;
    }
    for line in lines {
        output.push_str(line);
        output.push('\n');
    }
    output
}

fn format_search_results(
    root: &Path,
    regex: &str,
    file_pattern: Option<&str>,
    results: &[SearchMatch],
    total_matches: usize,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("SEARCH ROOT: {}\n", root.display()));
    output.push_str(&format!("REGEX: {}\n", regex));
    if let Some(pattern) = file_pattern {
        output.push_str(&format!("FILE_PATTERN: {}\n", pattern));
    }

    if results.is_empty() {
        output.push_str("No matches found.\n");
        return output;
    }

    for match_result in results {
        output.push_str(&format!(
            "\n{}:{}\n",
            match_result.path.display(),
            match_result.line_number
        ));
        for line in &match_result.context {
            output.push_str(line);
            output.push('\n');
        }
    }

    if total_matches >= MAX_SEARCH_MATCHES {
        output.push_str("\nMatches truncated at limit.\n");
    }

    output
}

fn truncate_line(line: &str) -> String {
    if line.len() <= MAX_LINE_LENGTH {
        return line.to_string();
    }
    let mut truncated = line.chars().take(MAX_LINE_LENGTH).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn format_tool_error(tool: &str, message: &str) -> String {
    format!("ERROR ({tool}): {message}\n")
}

fn build_globset(pattern: Option<&str>) -> Result<Option<GlobSet>, String> {
    let Some(pattern) = pattern else {
        return Ok(None);
    };

    if pattern.trim().is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    let glob = Glob::new(pattern).map_err(|e| format!("Invalid glob pattern: {}", e))?;
    builder.add(glob);
    let set = builder
        .build()
        .map_err(|e| format!("Failed to build glob matcher: {}", e))?;
    Ok(Some(set))
}

fn is_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    name == ".git" || name == "target"
}

struct SearchMatch {
    path: PathBuf,
    line_number: usize,
    context: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn read_file_respects_offset_and_limit() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("sample.txt");
        let mut file = fs::File::create(&file_path).expect("create file");
        writeln!(file, "first").unwrap();
        writeln!(file, "second").unwrap();
        writeln!(file, "third").unwrap();

        let output = read_file(&ReadFileArgs {
            path: file_path.to_string_lossy().to_string(),
            offset: Some(2),
            limit: Some(1),
        });

        assert!(output.contains("2| second"));
        assert!(!output.contains("1| first"));
    }

    #[test]
    fn search_files_finds_matches() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("lib.rs");
        let mut file = fs::File::create(&file_path).expect("create file");
        writeln!(file, "fn target() {{}}").unwrap();

        let output = search_files(&SearchFilesArgs {
            path: dir.path().to_string_lossy().to_string(),
            regex: "target".to_string(),
            file_pattern: Some("*.rs".to_string()),
        });

        assert!(output.contains("lib.rs"));
        assert!(output.contains("target"));
    }
}
