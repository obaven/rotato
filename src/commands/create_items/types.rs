use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Args)]
pub struct CreateItemsArgs {
    #[arg(long, env = "VW_BASE_URL", default_value = "https://vaultwarden.obaven.org")]
    pub base_url: String,
    #[arg(long, env = "BW_CLIENTID")]
    pub client_id: Option<String>,
    #[arg(long, env = "BW_CLIENTSECRET")]
    pub client_secret: Option<String>,
    #[arg(long, env = "VW_ORG_ID")]
    pub org_id: Option<String>,
    #[arg(long, env = "VW_ORG_KEY")]
    pub org_key: Option<String>, // Base64 encoded decrypted org key
    #[arg(long, default_value = "data/config.yaml")]
    pub config: String,
    
    #[arg(long, help = "Scan for rotation.yaml files instead of using config.yaml")]
    pub scan: bool,
    
    #[arg(long, help = "Filter to specific secret name(s)")]
    pub secret: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub secrets: Vec<SecretConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct SecretConfig {
    pub name: String,
    pub env: Option<String>,
    pub vaultwarden: Option<VaultwardenConfig>,
    pub keys: Option<Vec<KeyConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes: Option<serde_json::Value>, // Preserve as flexible JSON to avoid struct mismatch or use models definition if available
    #[serde(rename = "sourceFiles")]
    pub source_files: Option<HashMap<String, String>>,
    // Deprecated fields kept for back-compat if needed, but kubernetes object is preferred
    pub namespace: Option<String>,
    #[serde(rename = "secretName")]
    pub secret_name: Option<String>,
    #[serde(rename = "argocdApp")]
    pub argocd_app: Option<String>,
    pub notes: Option<String>,
    pub repo: Option<String>,
    pub path: Option<String>,
    #[serde(default, rename = "accessUsers")]
    pub access_users: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum KeyConfig {
    Name(String),
    Def(KeyDef),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct KeyDef {
    pub name: String,
    // Preserve other fields
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub length: Option<usize>,
    pub value: Option<String>,
    pub generator: Option<String>,
}

impl KeyConfig {
    pub fn name(&self) -> &str {
        match self {
            KeyConfig::Name(s) => s,
            KeyConfig::Def(d) => &d.name,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct VaultwardenConfig {
    #[serde(rename = "cipherId")]
    pub cipher_id: Option<String>,
    #[serde(rename = "collectionIds")]
    pub collection_ids: Option<Vec<String>>,
    pub collections: Option<Vec<String>>, // RotationManifest uses "collections"
    pub folder: Option<String>,
    pub notes: Option<String>,
}
