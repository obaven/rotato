use clap::Args;
use anyhow::{Result, anyhow};
use crate::bw_cli::BwCli;

#[derive(Args)]
pub struct ListGroupsArgs {
    #[arg(long, env = "VW_BASE_URL", default_value = "https://vaultwarden.obaven.org")]
    base_url: String,
    #[arg(long, env = "VW_ORG_ID")]
    org_id: Option<String>,
}

pub async fn run(args: ListGroupsArgs) -> Result<()> {
    // Basic auth logic reusing flows
    let (email, password_opt) = crate::flows::resolve_credentials()?;
    let session_key = std::env::var("BW_SESSION").ok();

    println!("Authenticating to probe Groups API via BW CLI...");
    // We actually need the session key for BW CLI.
    // get_org_key flow might give us a hint if we have session, but let's initialize BW CLI directly.
    
    // 1. Resolve Organization ID via regular flow if not provided?
    // Actually, get_org_key does that.
    let (_client, org_id, _org_key, _user_key) = crate::flows::get_org_key(
        &args.base_url,
        &email,
        session_key.clone(),
        password_opt.clone(),
        args.org_id.as_deref(),
    ).await?;

    println!("Target Org ID: {}", org_id);

    // 2. Initialize BW CLI
    let bw = if let Some(key) = session_key {
         BwCli::new(key, Some(org_id.clone()))
    } else if let Some(pass) = password_opt {
         let mut bw = BwCli::unlock(&email, &pass)?;
         bw.set_org_id(org_id.clone());
         bw
    } else {
        return Err(anyhow!("No session key or password available for BW CLI"));
    };

    println!("Fetching groups via BW CLI...");
    match bw.list_org_groups() {
        Ok(groups) => {
            println!("Successfully fetched {} groups.", groups.len());
            println!("{}", serde_json::to_string_pretty(&groups)?);
        },
        Err(e) => {
            println!("Failed to list groups via CLI: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
