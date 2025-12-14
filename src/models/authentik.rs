use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthentikTarget {
    pub path: String, // Path to write the blueprint file
    pub metadata: AuthentikMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthentikMetadata {
    pub name: String,
    pub model: String,
    pub identifiers: HashMap<String, String>,
    #[serde(default = "default_secret_field")]
    pub secret_field: String,
}

fn default_secret_field() -> String {
    "client_secret".to_string()
}
