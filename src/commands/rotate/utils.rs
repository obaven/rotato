use anyhow::{anyhow, Result};
use std::process::Command;

pub fn find_monorepo_root() -> Result<std::path::PathBuf> {
    // 1. Try git root
    if let Ok(output) = Command::new("git").args(&["rev-parse", "--show-toplevel"]).output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
             let path = std::path::PathBuf::from(s.trim());
             if path.join("apps").exists() {
                 return Ok(path);
             }
             // If this is a nested repo (vaultwarden), check parents
             for ancestor in path.ancestors() {
                  if ancestor.join("apps").exists() {
                      return Ok(ancestor.to_path_buf());
                  }
             }
             // Even higher? (outside the current git root if it's submodule)
             let mut current = path;
             while let Some(parent) = current.parent() {
                 if parent.to_path_buf().join("apps").exists() {
                      return Ok(parent.to_path_buf());
                  }
                  current = parent.to_path_buf();
             }
        }
    }
    
    // Fallback: Traversal from CWD
    let mut current = std::env::current_dir()?;
    loop {
        if current.join("apps").exists() {
            return Ok(current);
        }
        if !current.pop() { break; }
    }

    Err(anyhow!("Could not find monorepo root (containing 'apps' directory)."))
}
