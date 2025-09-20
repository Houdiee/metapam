use crate::{
    provider::Provider,
    supported_providers::{
        apt::AptProvider, arch::ArchProvider, cargo::CargoProvider, dotnet::DotnetProvider,
        go::GoProvider, node::NodeProvider,
    },
};

pub mod apt;
pub mod arch;
pub mod brew;
pub mod cargo;
pub mod dotnet;
pub mod go;
pub mod node;

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
