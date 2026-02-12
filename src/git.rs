use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct GitData {
    pub diff: String,
    pub files_changed: Vec<String>,
    pub head_hash: String,
    pub merge_base_hash: String,
    pub branch_name: Option<String>,
    pub repo_name: String,
    pub remote_url: Option<String>,
}

impl GitData {
    pub fn new(
        diff: String,
        files_changed: Vec<String>,
        head_hash: String,
        merge_base_hash: String,
        branch_name: Option<String>,
        repo_name: String,
        remote_url: Option<String>,
    ) -> Self {
        Self {
            diff,
            files_changed,
            head_hash,
            merge_base_hash,
            branch_name,
            repo_name,
            remote_url,
        }
    }
}

fn run_git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git {} failed: {}", args.join(" "), stderr));
    }

    String::from_utf8(output.stdout)
        .context("Failed to parse git output as UTF-8")
        .map(|s| s.trim().to_string())
}

pub fn get_git_data(default_branch: &str) -> Result<GitData> {
    let head_hash = run_git(&["rev-parse", "HEAD"])?;

    let merge_base_hash = run_git(&["merge-base", "HEAD", default_branch])?;

    let branch_name = run_git(&["branch", "--show-current"])?;
    let branch_name = if branch_name.is_empty() {
        None
    } else {
        Some(branch_name)
    };

    let diff_output = Command::new("git")
        .args([
            "diff",
            "--no-ext-diff",
            "--unified=5",
            "--no-color",
            &merge_base_hash,
        ])
        .output()
        .context("Failed to execute git diff")?;

    if !diff_output.status.success() {
        let stderr = String::from_utf8_lossy(&diff_output.stderr);
        return Err(anyhow!("git diff failed: {}", stderr));
    }

    let diff = String::from_utf8(diff_output.stdout).context("Failed to parse diff as UTF-8")?;

    let files_output = Command::new("git")
        .args(["diff", "--no-ext-diff", "--name-only", &merge_base_hash])
        .output()
        .context("Failed to execute git diff --name-only")?;

    if !files_output.status.success() {
        let stderr = String::from_utf8_lossy(&files_output.stderr);
        return Err(anyhow!("git diff --name-only failed: {}", stderr));
    }

    let files_changed = String::from_utf8(files_output.stdout)
        .context("Failed to parse changed files as UTF-8")?
        .lines()
        .map(|s| s.to_string())
        .collect();

    let repo_path = run_git(&["rev-parse", "--show-toplevel"])?;
    let repo_name = Path::new(&repo_path)
        .file_name()
        .context("Failed to extract repo name from path")?
        .to_str()
        .context("Repo name is not valid UTF-8")?
        .to_string();

    let remote_url = if let Some(ref branch) = branch_name {
        let remote_result = Command::new("git")
            .args(["config", "--get", &format!("branch.{}.remote", branch)])
            .stderr(Stdio::null())
            .output();

        if let Ok(remote_output) = remote_result {
            if remote_output.status.success() {
                if let Ok(remote_name) = String::from_utf8(remote_output.stdout) {
                    let remote_name = remote_name.trim().to_string();
                    if remote_name.is_empty() {
                        None
                    } else {
                        let url_result = Command::new("git")
                            .args(["remote", "get-url", &remote_name])
                            .stderr(Stdio::null())
                            .output();
                        if let Ok(url_output) = url_result {
                            if url_output.status.success() {
                                String::from_utf8(url_output.stdout)
                                    .ok()
                                    .map(|s| s.trim().to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(GitData::new(
        diff,
        files_changed,
        head_hash,
        merge_base_hash,
        branch_name,
        repo_name,
        remote_url,
    ))
}
