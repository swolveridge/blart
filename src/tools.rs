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
    pub mode: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub indentation: Option<IndentationOptions>,
}

#[derive(Debug, Deserialize)]
pub struct IndentationOptions {
    pub anchor_line: Option<usize>,
    pub max_levels: Option<usize>,
    pub include_siblings: Option<bool>,
    pub include_header: Option<bool>,
    pub max_lines: Option<usize>,
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
            description: "Read a file and return its contents with line numbers for diffing or discussion. IMPORTANT: This tool reads exactly one file per call. If you need multiple files, issue multiple parallel read_file calls. Supports two modes: 'slice' (default) reads lines sequentially with offset/limit; 'indentation' extracts complete semantic code blocks around an anchor line based on indentation hierarchy. Slice mode is ideal for initial file exploration, understanding overall structure, reading configuration/data files, or when you need a specific line range. Use it when you don't have a target line number. PREFER indentation mode when you have a specific line number from search results, error messages, or definition lookups - it guarantees complete, syntactically valid code blocks without mid-function truncation. IMPORTANT: Indentation mode requires anchor_line to be useful. Without it, only header content (imports) is returned. By default, returns up to 2000 lines per file. Lines longer than 2000 characters are truncated. Supports text extraction from PDF and DOCX files, but may not handle other binary files properly. Example: { path: 'src/app.ts' } Example (indentation mode): { path: 'src/app.ts', mode: 'indentation', indentation: { anchor_line: 42 } }".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read, relative to the workspace"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["slice", "indentation"],
                        "description": "Reading mode. 'slice' (default): read lines sequentially with offset/limit. 'indentation': extract a semantic code block around anchor_line."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "1-based line offset to start reading from (default 1)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to return (default 2000)"
                    },
                    "indentation": {
                        "type": "object",
                        "description": "Indentation mode options. Only used when mode='indentation'.",
                        "properties": {
                            "anchor_line": {
                                "type": "integer",
                                "description": "1-based line number to anchor the extraction."
                            },
                            "max_levels": {
                                "type": "integer",
                                "description": "Maximum indentation levels to include above the anchor (0 = unlimited)."
                            },
                            "include_siblings": {
                                "type": "boolean",
                                "description": "Include sibling blocks at the same indentation level as the anchor block."
                            },
                            "include_header": {
                                "type": "boolean",
                                "description": "Include file header content (imports/module-level comments) at top of output."
                            },
                            "max_lines": {
                                "type": "integer",
                                "description": "Hard cap on lines returned for indentation mode."
                            }
                        },
                        "required": [],
                        "additionalProperties": false
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
            description: "Request to perform a regex search across files in a specified directory, providing context-rich results. This tool searches for patterns or specific content across multiple files, displaying each match with encapsulating context.\n\nCraft your regex patterns carefully to balance specificity and flexibility. Use this tool to find code patterns, TODO comments, function definitions, or any text-based information across the project. The results include surrounding context, so analyze the surrounding code to better understand the matches. Leverage this tool in combination with other tools for more comprehensive analysis.\n\nParameters:\n- path: (required) The path of the directory to search in (relative to the current workspace directory). This directory will be recursively searched.\n- regex: (required) The regular expression pattern to search for. Uses Rust regex syntax.\n- file_pattern: (optional) Glob pattern to filter files (e.g., '*.ts' for TypeScript files). If not provided, it will search all files (*).\n\nExample: Searching for all .ts files in the current directory\n{ \"path\": \".\", \"regex\": \".*\", \"file_pattern\": \"*.ts\" }\n\nExample: Searching for function definitions in JavaScript files\n{ \"path\": \"src\", \"regex\": \"function\\s+\\w+\", \"file_pattern\": \"*.js\" }".to_string(),
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

pub fn summarize_tool_call(name: &str, arguments: &str) -> String {
    match name {
        "read_file" => match serde_json::from_str::<ReadFileArgs>(arguments) {
            Ok(args) => {
                if args.mode.as_deref() == Some("indentation") {
                    let anchor = args
                        .indentation
                        .as_ref()
                        .and_then(|opt| opt.anchor_line)
                        .unwrap_or(1);
                    format!(
                        "read_file {} (indentation anchor_line={})",
                        args.path, anchor
                    )
                } else {
                    let offset = args.offset.unwrap_or(1).max(1);
                    let limit = args.limit.unwrap_or(DEFAULT_READ_LIMIT).min(MAX_READ_LIMIT);
                    let end = offset.saturating_add(limit.saturating_sub(1));
                    format!("read_file {}:{}-{}", args.path, offset, end)
                }
            }
            Err(_) => format!("read_file (invalid args)"),
        },
        "search_files" => match serde_json::from_str::<SearchFilesArgs>(arguments) {
            Ok(args) => match args.file_pattern.as_deref() {
                Some(pattern) if !pattern.trim().is_empty() => format!(
                    "search_files {} regex={} files={}",
                    args.path, args.regex, pattern
                ),
                _ => format!("search_files {} regex={}", args.path, args.regex),
            },
            Err(_) => format!("search_files (invalid args)"),
        },
        _ => format!("{} (unknown tool)", name),
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

    if args.mode.as_deref() == Some("indentation") {
        return read_file_indentation(path, &contents, args);
    }

    read_file_slice(path, &contents, args)
}

fn read_file_slice(path: &Path, contents: &str, args: &ReadFileArgs) -> String {
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

fn read_file_indentation(path: &Path, contents: &str, args: &ReadFileArgs) -> String {
    let lines: Vec<&str> = contents.lines().collect();
    if lines.is_empty() {
        return format_file_output(path, &[]);
    }

    let indentation = args.indentation.as_ref();
    let anchor_line = indentation
        .and_then(|opt| opt.anchor_line)
        .unwrap_or(1)
        .max(1);
    let anchor_index = anchor_line.saturating_sub(1).min(lines.len() - 1);
    let include_siblings = indentation
        .and_then(|opt| opt.include_siblings)
        .unwrap_or(false);
    let include_header = indentation
        .and_then(|opt| opt.include_header)
        .unwrap_or(true);
    let max_levels = indentation.and_then(|opt| opt.max_levels).unwrap_or(0);
    let max_lines = indentation.and_then(|opt| opt.max_lines);

    let anchor_index = find_non_blank_line(&lines, anchor_index);
    let base_indent = line_indent(lines[anchor_index]);

    let mut start_index = if include_siblings {
        find_parent_boundary_up(&lines, anchor_index, base_indent)
    } else {
        find_block_start_up(&lines, anchor_index, base_indent)
    };
    let mut end_index = if include_siblings {
        find_parent_boundary_down(&lines, anchor_index, base_indent)
    } else {
        find_block_end_down(&lines, anchor_index, base_indent)
    };

    if max_levels > 0 {
        start_index = expand_start_for_levels(&lines, start_index, base_indent, max_levels);
    }

    if include_header {
        let header_end = find_header_end(&lines);
        if header_end < start_index {
            start_index = header_end;
        }
    }

    if end_index < start_index {
        end_index = start_index;
    }

    if let Some(max_lines) = max_lines {
        let max_lines = max_lines.max(1);
        let allowed_end = start_index.saturating_add(max_lines.saturating_sub(1));
        end_index = end_index.min(allowed_end);
    }

    let mut numbered_lines = Vec::new();
    for (i, line) in lines[start_index..=end_index].iter().enumerate() {
        let line_number = start_index + 1 + i;
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

fn find_non_blank_line(lines: &[&str], index: usize) -> usize {
    if !lines[index].trim().is_empty() {
        return index;
    }
    let mut up = index;
    while up > 0 {
        up -= 1;
        if !lines[up].trim().is_empty() {
            return up;
        }
    }
    let mut down = index + 1;
    while down < lines.len() {
        if !lines[down].trim().is_empty() {
            return down;
        }
        down += 1;
    }
    index
}

fn line_indent(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

fn find_block_start_up(lines: &[&str], anchor: usize, base_indent: usize) -> usize {
    let mut idx = anchor;
    while idx > 0 {
        let prev = idx - 1;
        let line = lines[prev];
        if line.trim().is_empty() {
            idx = prev;
            continue;
        }
        let indent = line_indent(line);
        if indent <= base_indent {
            return prev + 1;
        }
        idx = prev;
    }
    0
}

fn find_block_end_down(lines: &[&str], anchor: usize, base_indent: usize) -> usize {
    let mut idx = anchor + 1;
    while idx < lines.len() {
        let line = lines[idx];
        if line.trim().is_empty() {
            idx += 1;
            continue;
        }
        let indent = line_indent(line);
        if indent <= base_indent {
            return idx.saturating_sub(1);
        }
        idx += 1;
    }
    lines.len() - 1
}

fn find_parent_boundary_up(lines: &[&str], anchor: usize, base_indent: usize) -> usize {
    let mut idx = anchor;
    while idx > 0 {
        let prev = idx - 1;
        let line = lines[prev];
        if line.trim().is_empty() {
            idx = prev;
            continue;
        }
        let indent = line_indent(line);
        if indent < base_indent {
            return prev + 1;
        }
        idx = prev;
    }
    0
}

fn find_parent_boundary_down(lines: &[&str], anchor: usize, base_indent: usize) -> usize {
    let mut idx = anchor + 1;
    while idx < lines.len() {
        let line = lines[idx];
        if line.trim().is_empty() {
            idx += 1;
            continue;
        }
        let indent = line_indent(line);
        if indent < base_indent {
            return idx.saturating_sub(1);
        }
        idx += 1;
    }
    lines.len() - 1
}

fn expand_start_for_levels(
    lines: &[&str],
    mut start: usize,
    base_indent: usize,
    max_levels: usize,
) -> usize {
    let mut current_indent = base_indent;
    let mut levels = 0;
    let mut idx = start;
    while idx > 0 {
        idx -= 1;
        let line = lines[idx];
        if line.trim().is_empty() {
            continue;
        }
        let indent = line_indent(line);
        if indent < current_indent {
            levels += 1;
            current_indent = indent;
            if levels > max_levels {
                return idx + 1;
            }
            start = idx;
        }
    }
    start
}

fn find_header_end(lines: &[&str]) -> usize {
    let mut seen_non_blank = false;
    let mut end = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            end = i + 1;
            if seen_non_blank {
                break;
            }
        } else {
            seen_non_blank = true;
            end = i + 1;
        }
    }
    end
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
            mode: None,
            offset: Some(2),
            limit: Some(1),
            indentation: None,
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

    #[test]
    fn read_file_indentation_mode_extracts_block() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("sample.rs");
        let mut file = fs::File::create(&file_path).expect("create file");
        writeln!(file, "fn outer() {{").unwrap();
        writeln!(file, "    let x = 1;").unwrap();
        writeln!(file, "    println!(\"hi\");").unwrap();
        writeln!(file, "}}").unwrap();

        let output = read_file(&ReadFileArgs {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("indentation".to_string()),
            offset: None,
            limit: None,
            indentation: Some(IndentationOptions {
                anchor_line: Some(2),
                max_levels: None,
                include_siblings: None,
                include_header: Some(false),
                max_lines: None,
            }),
        });

        assert!(output.contains("2|     let x = 1;"));
        assert!(output.contains("3|     println!(\"hi\");"));
        assert!(!output.contains("1| fn outer()"));
    }
}
