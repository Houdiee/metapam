use crate::{
    provider::Provider,
    supported_providers::{
        apt::AptProvider, arch::ArchProvider, cargo::CargoProvider, dotnet::DotnetProvider, fisher::FisherProvider, node::NodeProvider,
    },
};

pub mod apt;
pub mod arch;
pub mod brew;
pub mod cargo;
pub mod dotnet;
pub mod fisher;
pub mod node;

pub const SUPPORTED_PROVIDERS: &[&str] = &["pacman", "paru", "yay", "npm", "pnpm", "apt", "cargo", "dotnet", "fisher"];

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

        "dotnet" => Some(Box::new(DotnetProvider {})),

        "fisher" => Some(Box::new(FisherProvider {})),

        _ => None,
    }
}
