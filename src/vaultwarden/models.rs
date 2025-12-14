use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct KdfInfo {
    #[serde(rename = "kdf")]
    pub kdf: i32,
    #[serde(rename = "kdfIterations")]
    pub kdf_iterations: u32,
    #[serde(rename = "kdfMemory")]
    pub kdf_memory: Option<u32>,
    #[serde(rename = "kdfParallelism")]
    pub kdf_parallelism: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct SyncData {
    pub profile: Profile,
    pub folders: Vec<FolderData>,
    pub ciphers: Option<Vec<Cipher>>,
    pub domains: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct Cipher {
    pub id: String,
    pub name: String,
    // other fields omitted
}

#[derive(Deserialize, Debug)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub key: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    pub id: String,
    pub email: String,
    pub key: String,
    #[serde(rename = "privateKey")]
    pub private_key: Option<String>,
    pub organizations: Option<Vec<Organization>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Collection {
    pub id: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    pub name: String,
    #[serde(rename = "externalId")]
    pub external_id: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Folder {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FolderData {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Member {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub status: i32,
    #[serde(rename = "type")]
    pub member_type: i32,
    // collections?
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MemberCollectionAccess {
    // fields
}
