use crate::{
    cli::{Cli, Commands, ListCommands, ProviderCommands},
    supported_providers::get_provider,
};
use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashSet;

pub mod cli;
pub mod config;
pub mod package_diff;
pub mod provider;
pub mod supported_providers;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { list_command } => match list_command {
            ListCommands::Available => {}
            ListCommands::Active => {
                let active = config::get_active_providers()?;
                for provider in active {
                    println!("{}", provider);
                }
            }
        },

        Commands::Activate => {
            supported_providers::activate_all_supported()
                .with_context(|| format!("Failed to activate all supported providers"))?;
        }

        Commands::Provider(args) => {
            let provider_name = args.provider;

            let provider = get_provider(&provider_name)
                .with_context(|| format!("Provider `{provider_name}` not found/supported"))?;

            match args.provider_command {
                ProviderCommands::Activate => {
                    provider.activate().with_context(|| {
                        format!("Failed to activate provider `{provider_name}`")
                    })?;

                    let provider_path = config::get_provider_path(&provider_name)?;
                    println!(
                        "Activated provider `{provider_name}` at {}",
                        provider_path.display()
                    );
                }

                ProviderCommands::Declare(pkg_args) => {
                    let pkgs = HashSet::from_iter(pkg_args.packages);
                    provider.declare_packages(&pkgs)?;
                }

                ProviderCommands::Remove(pkg_args) => {
                    let pkgs = HashSet::from_iter(pkg_args.packages);
                    provider.remove_packages(&pkgs).with_context(|| {
                        format!("Provider `{provider_name}` not found/supported")
                    })?;
                }

                ProviderCommands::List => {
                    let pkgs = provider.list_packages().with_context(|| {
                        format!("Failed to tidy/cleanup packages for provider `{provider_name}`")
                    })?;
                    for pkg in pkgs {
                        println!("{}", pkg);
                    }
                }

                ProviderCommands::Tidy => {
                    provider.tidy().with_context(|| {
                        format!("Failed to tidy/cleanup packages for provider `{provider_name}`")
                    })?;
                }

                ProviderCommands::Diff => {
                    let diff = provider.diff().with_context(|| {
                        format!("Failed to show diff for provider `{provider_name}`")
                    })?;
                    println!("{}", diff);
                }
            };
        }
    }

    Ok(())
}
