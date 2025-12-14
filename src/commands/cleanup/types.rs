use clap::Args;
use serde::Deserialize;

#[derive(Args)]
pub struct CleanupArgs {
    #[arg(long, default_value = "data/cleanup.yaml")]
    pub config: String,
    #[arg(long)]
    pub all: bool,
}

#[derive(Deserialize)]
pub struct Config {
    pub secrets: Vec<SecretConfig>,
}

#[derive(Deserialize)]
pub struct SecretConfig {
    pub name: String,
    pub env: Option<String>,
    pub vaultwarden: Option<VaultwardenConfig>,
}

#[derive(Deserialize)]
pub struct VaultwardenConfig {
    #[serde(rename = "cipherId")]
    pub cipher_id: Option<String>,
    pub name: Option<String>,
    pub folder: Option<String>,
}
