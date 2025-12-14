use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::keys::KeyDefinition;
use super::vaultwarden_target::VaultwardenTarget;
use super::k8s::KubernetesTarget;
use super::authentik::AuthentikTarget;
use super::user::UserDefinition;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RotationManifest {
    pub version: String,
    #[serde(default)]
    pub secrets: Vec<SecretDefinition>,
    #[serde(default)]
    pub users: Vec<UserDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecretDefinition {
    pub name: String,
    pub description: Option<String>,
    pub vaultwarden: VaultwardenTarget,
    pub kubernetes: KubernetesTarget,
    #[serde(default, rename = "additionalKubernetes")]
    pub additional_kubernetes: Option<Vec<KubernetesTarget>>,
    #[serde(default)]
    pub authentik: Option<AuthentikTarget>,
    pub keys: Vec<KeyDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<RotationPolicy>,
    #[serde(default)]
    pub hooks: Option<SecretHooks>,
    #[serde(default, rename = "accessUsers")]
    pub access_users: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecretHooks {
    pub pre: Option<Vec<HookCommand>>,
    pub post: Option<Vec<HookCommand>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HookCommand {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub shell: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RotationPolicy {
    pub schedule: Option<String>, // e.g. "30d"
}
