use clap::Args;

#[derive(Args, Clone)]
pub struct RotateArgs {
    #[arg(long, default_value = "data/config.yaml")]
    pub config: String,
    
    #[arg(long, help = "Scan for rotation.yaml files instead of using config.yaml", default_value = "true")]
    pub scan: bool,
    
    #[arg(long, help = "Run in dry-run mode (no changes committed)")]
    pub dry_run: bool,
    
    #[arg(long, help = "Enable verbose debug logging")]
    pub debug: bool,

    #[arg(long, help = "Force rotation regardless of policy")]
    pub force: bool,
    
    #[arg(long, help = "Debug: API Payloads (Large)")]
    pub debug_api: bool,
    
    #[arg(long, help = "Debug: Crypto operations (MAC/RSA) - Noisy")]
    pub debug_crypto: bool,
    
    #[arg(long, help = "Debug: Authentication steps")]
    pub debug_auth: bool,

    #[arg(long, help = "Filter rotation to specific secret name(s)")]
    pub secret: Option<String>,
}
