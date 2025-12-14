use clap::Args;
use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::fs::File;

#[derive(Args)]
pub struct SetupArgs {
    #[arg(long, env = "BW_CLIENTID")]
    client_id: Option<String>,
    #[arg(long, env = "BW_CLIENTSECRET")]
    client_secret: Option<String>,
    #[arg(long, env = "GIT_PAT")]
    git_pat: Option<String>,
    #[arg(long, default_value = "apps/cicd/argo-workflows/base/sealed-vaultwarden-rotation-token.yaml")]
    output: String,
}

pub async fn run(args: SetupArgs) -> Result<()> {
    println!("=== Vaultwarden Rotation Setup ===");

    let client_id = match args.client_id {
        Some(v) => v,
        None => prompt("Enter BW_CLIENTID: ")?,
    };

    let client_secret = match args.client_secret {
        Some(v) => v,
        None => rpassword::prompt_password("Enter BW_CLIENTSECRET: ")?,
    };

    let git_pat = match args.git_pat {
        Some(v) => v,
        None => rpassword::prompt_password("Enter GIT_PAT: ")?,
    };

    if client_id.is_empty() || client_secret.is_empty() || git_pat.is_empty() {
        return Err(anyhow!("All credentials are required."));
    }

    let secret_yaml = format!(
        r#"apiVersion: v1
kind: Secret
metadata:
  name: vaultwarden-rotation-token
  namespace: argocd
type: Opaque
stringData:
  BW_CLIENTID: "{}"
  BW_CLIENTSECRET: "{}"
  GIT_PAT: "{}"
"#,
        client_id, client_secret, git_pat
    );

    println!("Sealing token -> {}", args.output);

    // 1. Fetch cert
    let cert_output = Command::new("kubeseal")
        .arg("--fetch-cert")
        .output()
        .map_err(|e| anyhow!("Failed to run kubeseal --fetch-cert: {}", e))?;

    if !cert_output.status.success() {
        return Err(anyhow!("kubeseal --fetch-cert failed: {}", String::from_utf8_lossy(&cert_output.stderr)));
    }

    let cert_path = "/tmp/vw-rotation-cert.pem";
    let mut cert_file = File::create(cert_path)?;
    cert_file.write_all(&cert_output.stdout)?;

    // 2. Seal
    let mut child = Command::new("kubeseal")
        .arg("--cert")
        .arg(cert_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(secret_yaml.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!("kubeseal failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let mut out_file = File::create(&args.output)?;
    out_file.write_all(&output.stdout)?;

    println!("Generated {}", args.output);
    
    // Cleanup
    std::fs::remove_file(cert_path).ok();

    Ok(())
}

fn prompt(msg: &str) -> Result<String> {
    print!("{}", msg);
    io::stdout().flush()?;
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer.trim().to_string())
}
