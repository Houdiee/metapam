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
    /// Activate all providers found in system
    Activate,
    /// Installs/Removes packages for all providers based off declared config
    Tidy,
    /// View the difference between all installed and declared packages
    Diff,
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
    /// Installs/Removes packages for all providers based off declared config
    Tidy,
    /// Shows the difference between installed and declared packages
    Diff,
}

#[derive(Args)]
pub struct PackagesArgs {
    /// The package(s) to operate on
    pub packages: Vec<String>,
}
