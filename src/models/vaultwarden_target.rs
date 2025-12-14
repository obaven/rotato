use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultwardenTarget {
    #[serde(rename = "cipherId")]
    pub cipher_id: Option<String>,
    pub name: Option<String>, // For lookup by name
    #[serde(rename = "collectionIds", skip_serializing_if = "Option::is_none")]
    pub collection_ids: Option<Vec<String>>,
    pub collections: Option<Vec<String>>,
    pub folder: Option<String>,
    pub notes: Option<String>,
    pub fields: Option<HashMap<String, String>>,
}
