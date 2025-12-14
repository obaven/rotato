use serde::{Deserialize, Serialize};
use super::authentik::AuthentikTarget;
use super::vaultwarden_target::VaultwardenTarget;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDefinition {
    pub username: String,
    pub email: String,
    #[serde(default)]
    pub groups: Vec<String>,
    pub authentik: Option<AuthentikTarget>,
    pub vaultwarden: VaultwardenTarget,
}
