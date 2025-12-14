use rotato::commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    ListCollections(commands::list_collections::ListCollectionsArgs),
    CreateItems(commands::create_items::CreateItemsArgs),
    GetKey(commands::get_key::GetKeyArgs),
    Rotate(commands::rotate::RotateArgs),
    Scaffold(commands::scaffold::ScaffoldArgs),
    Setup(commands::setup::SetupArgs),
    Cleanup(commands::cleanup::CleanupArgs),
    #[command(name = "check", about = "Run pre-flight checks")]
    Check(commands::check::CheckArgs),

    #[command(name = "find", about = "Find a password by username/email")]
    Find(commands::find::FindArgs),
    #[command(name = "list-groups", about = "List groups in organization")]
    ListGroups(commands::list_groups::ListGroupsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // dotenv removed as it is not in Cargo.toml

    let cli = Cli::parse();

    match cli.command {
        Commands::ListCollections(args) => commands::list_collections::run(args).await,
        Commands::CreateItems(args) => commands::create_items::run(args).await,
        Commands::GetKey(args) => commands::get_key::run(args).await,
        Commands::Rotate(args) => commands::rotate::run(args).await,
        Commands::Scaffold(args) => commands::scaffold::run(args).await,
        Commands::Setup(args) => commands::setup::run(args).await,
        Commands::Cleanup(args) => commands::cleanup::run(args).await,
        Commands::Check(args) => commands::check::run(args).await,
        Commands::Find(args) => commands::find::run(args).await,
        Commands::ListGroups(args) => commands::list_groups::run(args).await,
    }
}
