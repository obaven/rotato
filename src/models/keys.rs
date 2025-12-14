use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct KeyDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>, // For random
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>, // For static
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>, // Future use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FileSource>, // For file-based
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeyType {
    #[default]
    Random,
    Static,
    File,
    K8s, // Fallback/Retrieval
    Ssh,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSource {
    pub path: String,
    #[serde(rename = "keyPath")]
    pub key_path: String,
}
