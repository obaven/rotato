use anyhow::Result;
use std::io::Write;
use crate::vaultwarden::VaultwardenClient;

pub async fn purge_all(client: &VaultwardenClient, org_key: &[u8], user_key: &[u8]) -> Result<()> {
     println!("!!! PURGE MODE ACTIVE !!! Deleting ALL items in the vault...");
     let sync_data = client.sync().await?;
     if let Some(ciphers) = sync_data.ciphers {
         println!("Found {} items to delete.", ciphers.len());
         for cipher in ciphers {
             let name_enc = cipher.name;
             let name_enc_clean = if name_enc.starts_with("2.") { &name_enc[2..] } else { &name_enc };
             
             let name = if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_hmac(name_enc_clean, &org_key) {
                  String::from_utf8(pt).unwrap_or_else(|_| "???".to_string())
             } else if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_raw(name_enc_clean, &org_key) {
                  String::from_utf8(pt).unwrap_or_else(|_| "???".to_string())
             } else {
                 "<decryption failed>".to_string()
             };

             print!("Deleting '{}' ({}) ... ", name, cipher.id);
             std::io::stdout().flush()?;
             match client.delete_item(&cipher.id).await {
                 Ok(_) => println!("OK"),
                 Err(e) => println!("Failed: {}", e),
             }
         }
     }

     println!("Deleting ALL folders...");
     let folders = client.list_folders().await?;
     println!("Found {} folders to delete.", folders.len());
     for f in folders {
         let name_enc = f.name;
         let name_enc_clean = if name_enc.starts_with("2.") { &name_enc[2..] } else { &name_enc };
         let name = if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_hmac(name_enc_clean, &user_key) {
                 String::from_utf8(pt).unwrap_or_else(|_| "???".to_string())
         } else {
             "<decryption failed>".to_string()
         };

         print!("Deleting folder '{}' ({}) ... ", name, f.id);
         std::io::stdout().flush()?;
         match client.delete_folder(&f.id).await {
             Ok(_) => println!("OK"),
             Err(e) => println!("Failed: {}", e),
         }
     }

     Ok(())
}
