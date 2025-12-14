use clap::Args;
use anyhow::Result;
use std::io::{self, Write};
use base64::{Engine as _, engine::general_purpose};

#[derive(Args)]
pub struct GetKeyArgs {
    #[arg(long, env = "VW_BASE_URL", default_value = "https://vaultwarden.obaven.org")]
    base_url: String,
    #[arg(long, env = "BW_EMAIL")]
    email: Option<String>,
    #[arg(long, env = "BW_SESSION")]
    session_key: Option<String>,
}

pub async fn run(args: GetKeyArgs) -> Result<()> {
    println!("=== Vaultwarden Organization Key Extractor ===");
    
    let email = match args.email {
        Some(e) => e,
        None => {
            print!("Email: ");
            io::stdout().flush()?;
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            buffer.trim().to_string()
        }
    };

    // We pass the session key as base64 string directly to the flow
    let (_client, org_id, org_key, _) = crate::flows::get_org_key(
        &args.base_url,
        &email,
        args.session_key,
        None, // password_override
        None, // No filter, just get the first one or logic inside handles it
    ).await?;

    println!("SUCCESS! Organization Key for '{}': {}", org_id, general_purpose::STANDARD.encode(&org_key));
    
    Ok(())
}
