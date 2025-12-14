use anyhow::Result;
use crate::vaultwarden::VaultwardenClient;
use crate::models::SecretDefinition;
use crate::logic::resolution::resolve_collection_ids;

pub async fn handle_folder(
    client: &VaultwardenClient,
    item: &mut serde_json::Value,
    secret: &SecretDefinition,
    org_key: &[u8],
) -> Result<()> {
    if let Some(folder_name) = &secret.vaultwarden.folder {
            println!("    Resolving folder '{}'...", folder_name);
            if let Some(fid) = client.resolve_folder_id(folder_name, org_key, true).await? {
                item["folderId"] = serde_json::Value::String(fid);
            } else {
                println!("    Warning: Failed to resolve folder ID for '{}'", folder_name);
            }
    }
    Ok(())
}

pub async fn handle_collections(
    client: &VaultwardenClient,
    item: &mut serde_json::Value,
    secret: &SecretDefinition,
    org_id: &str,
    org_key: &[u8],
) -> Result<Option<Vec<String>>> {
    if let Some(ids) = &secret.vaultwarden.collection_ids {
        if !ids.is_empty() {
            item["collectionIds"] = serde_json::Value::from(ids.clone());
            if let Some(obj) = item.as_object_mut() {
                obj.remove("collections");
            }
            return Ok(Some(ids.clone()));
        }
    }

    if let Some(cols) = &secret.vaultwarden.collections {
            println!("    Resolving collection IDs for {:?}", cols);
            let all_collections = client.list_collections(org_id).await?;
            let resolved_ids = resolve_collection_ids(cols, &all_collections, org_key);
            if resolved_ids.len() != cols.len() {
                println!("    Warning: Could not resolve all collections!");
            }
            
            item["collectionIds"] = serde_json::Value::from(resolved_ids.clone());
            // Strip read-only
             if let Some(obj) = item.as_object_mut() {
                obj.remove("collections");
            }
            Ok(Some(resolved_ids))
    } else {
         // Strip read-only even if we don't update collections explicitly?
         if let Some(obj) = item.as_object_mut() {
             obj.remove("collections");
         }
        Ok(None)
    }
}
