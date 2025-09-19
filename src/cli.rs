use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available and active providers
    List {
        #[command(subcommand)]
        list_command: ListCommands,
    },
    /// Interact with a specific provider
    Provider(ProviderArgs),
}

#[derive(Subcommand)]
pub enum ListCommands {
    /// List all available providers
    Available,
    /// List all active providers
    Active,
}

#[derive(Args)]
pub struct ProviderArgs {
    /// The provider to use (e.g., 'npm', 'pip')
    pub provider: String,

    #[command(subcommand)]
    pub provider_command: ProviderCommands,
}

#[derive(Subcommand)]
pub enum ProviderCommands {
    /// Activate a provider
    Activate,
    /// Declare a package(s)
    Declare(PackagesArgs),
    /// Uninstall packages(s)
    Remove(PackagesArgs),
    /// Lists all packages installed by a provider
    List,
    /// Tidy's up system to match the declared config
    Tidy,
    /// Shows the difference between installed and declared packages
    Diff,
}

#[derive(Args)]
pub struct PackagesArgs {
    /// The package(s) to operate on
    pub packages: Vec<String>,
}
