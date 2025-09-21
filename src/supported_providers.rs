use crate::{
    config,
    provider::Provider,
    supported_providers::{
        apt::AptProvider, arch::ArchProvider, cargo::CargoProvider, dotnet::DotnetProvider,
        go::GoProvider, node::NodeProvider,
    },
};
use anyhow::Result;
use which::which;

pub mod apt;
pub mod arch;
pub mod brew;
pub mod cargo;
pub mod dotnet;
pub mod go;
pub mod node;

pub fn activate_all_supported() -> Result<()> {
    for supported in SUPPORTED_PROVIDERS {
        let provider = get_provider(supported).expect(
            format!(
                "The provider {supported} should be supported but has no concrete implementation"
            )
            .as_str(),
        );

        match which(supported) {
            Ok(_) => {
                if !config::config_exists(provider.get_name()) {
                    provider.activate()?;
                    println!("Activated provider `{supported}`");
                } else {
                    println!("Found existing config for `{supported}`. Skipping...");
                }
            }
            Err(_) => {
                println!("`{supported}` not found in filesystem. Skipping...")
            }
        }
    }

    Ok(())
}

pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "pacman", "paru", "yay", "npm", "pnpm", "apt", "cargo", "go", "dotnet",
];

pub fn get_provider(name: &str) -> Option<Box<dyn Provider>> {
    match name {
        // Arch
        "pacman" => Some(Box::new(ArchProvider {
            manager: arch::ArchManager::Pacman,
        })),
        "paru" => Some(Box::new(ArchProvider {
            manager: arch::ArchManager::Paru,
        })),
        "yay" => Some(Box::new(ArchProvider {
            manager: arch::ArchManager::Yay,
        })),

        // Node
        "npm" => Some(Box::new(NodeProvider {
            manager: node::NodeManager::Npm,
        })),
        "pnpm" => Some(Box::new(NodeProvider {
            manager: node::NodeManager::Pnpm,
        })),
        "bun" => Some(Box::new(NodeProvider {
            manager: node::NodeManager::Bun,
        })),
        "yarn" => Some(Box::new(NodeProvider {
            manager: node::NodeManager::Yarn,
        })),

        "apt" => Some(Box::new(AptProvider {})),

        "cargo" => Some(Box::new(CargoProvider {})),

        "go" => Some(Box::new(GoProvider {})),

        "dotnet" => Some(Box::new(DotnetProvider {})),

        _ => None,
    }
}
