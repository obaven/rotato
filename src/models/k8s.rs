use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KubernetesTarget {
    pub namespace: String,
    #[serde(rename = "secretName")]
    pub secret_name: String,
    pub path: String, // Path to SealedSecret file
    #[serde(default)]
    pub labels: Option<std::collections::HashMap<String, String>>,
}
