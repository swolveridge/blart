pub fn get_system_prompt() -> String {
    let base = include_str!("../prompt.txt");
    let tools = include_str!("../prompt_tools.txt");
    format!("{}\n\n{}", tools, base)
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
