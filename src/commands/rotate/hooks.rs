use anyhow::{anyhow, Result};
use std::process::Command;

pub fn execute_hooks(hooks: &[crate::models::HookCommand], cwd: &str, dry_run: bool, secret_data: &std::collections::HashMap<String, String>) -> Result<()> {
    for hook in hooks {
        println!("    [HOOK] Running: {}", hook.command);
        if dry_run {
            println!("      [Dry Run] Would execute: {} {:?}", hook.command, hook.args);
            // In dry run, we might want to print the env vars we WOULD set
            for (k, _) in secret_data {
                println!("      [Dry Run] Env: ROTATOR_KEY_{}=<REDACTED>", k.to_uppercase());
            }
            continue;
        }

        let mut cmd = if hook.shell.unwrap_or(false) {
            let mut c = Command::new("sh");
            let mut command_string = hook.command.clone();
            if let Some(args) = &hook.args {
                for arg in args {
                    command_string.push_str(" ");
                    command_string.push_str(arg);
                }
            }
            c.arg("-c").arg(command_string);
            c
        } else {
            let mut c = Command::new(&hook.command);
            if let Some(args) = &hook.args {
                c.args(args);
            }
            c
        };

        cmd.current_dir(cwd);
        
        // Inject secret values as environment variables
        for (key, value) in secret_data {
            let env_key = format!("ROTATOR_KEY_{}", key.to_uppercase());
            cmd.env(env_key, value);
        }
        
        if let Some(env) = &hook.env {
            cmd.envs(env);
        }

        let output = cmd.output().map_err(|e| anyhow!("Failed to spawn hook '{}': {}", hook.command, e))?;
        
        if !output.status.success() {
             let stderr = String::from_utf8_lossy(&output.stderr);
             let stdout = String::from_utf8_lossy(&output.stdout);
             return Err(anyhow!("Hook '{}' failed.\nSTDOUT: {}\nSTDERR: {}", hook.command, stdout, stderr));
        } else {
             if !output.stdout.is_empty() {
                 println!("      Output: {}", String::from_utf8_lossy(&output.stdout).trim());
             }
        }
    }
    Ok(())
}
