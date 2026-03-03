use std::process::Command;

/// Captured stdout/stderr from a git subprocess invocation.
pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
}

pub fn git(args: &[&str]) -> Result<GitOutput, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(GitOutput { stdout, stderr })
    } else {
        Err(format!("git {} failed: {}", args.join(" "), stderr))
    }
}

/// Run git command, return stdout trimmed
pub fn git_stdout(args: &[&str]) -> Result<String, String> {
    Ok(git(args)?.stdout)
}

/// Run git command, allow non-zero exit (returns Ok with output either way)
pub fn git_allow_fail(args: &[&str]) -> Result<GitOutput, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    Ok(GitOutput { stdout, stderr })
}
