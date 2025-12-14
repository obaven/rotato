use anyhow::{anyhow, Result};
use std::process::Command;

pub fn run_git_command(args: &[&str], cwd: &str) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| anyhow!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!("Git command failed: \nSTDERR: {}\nSTDOUT: {}", 
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    Ok(())
}
