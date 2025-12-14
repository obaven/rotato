use clap::Args;
use anyhow::Result;

#[derive(Args)]
pub struct ListCollectionsArgs {
    #[arg(long, env = "VW_BASE_URL", default_value = "https://vaultwarden.obaven.org")]
    base_url: String,
    #[arg(long, env = "BW_CLIENTID")]
    client_id: Option<String>,
    #[arg(long, env = "BW_CLIENTSECRET")]
    client_secret: Option<String>,
    #[arg(long, env = "VW_ORG_ID")]
    org_id: Option<String>,
}

pub async fn run(args: ListCollectionsArgs) -> Result<()> {
    let (email, password_opt) = crate::flows::resolve_credentials()?;
    
    let session_key = std::env::var("BW_SESSION").ok();

    println!("Authenticating to Vaultwarden...");
    let (client, org_id, org_key, _) = crate::flows::get_org_key(
        &args.base_url,
        &email,
        session_key,
        password_opt,
        args.org_id.as_deref(),
    ).await?;

    println!("Listing collections for Org: {}", org_id);
    let collections = client.list_collections(&org_id).await?;
    println!("Collections:");
    for c in collections {
        let name = if c.name.starts_with("2.") {
             let b = crate::crypto::decrypt_aes256_cbc_hmac(&c.name[2..], &org_key).unwrap_or_default();
             String::from_utf8(b).unwrap_or(c.name.clone())
         } else {
             c.name.clone()
         };
        println!("- {} : {}", name, c.id);
    }

    Ok(())
}

