use crate::{
    provider::Provider,
    supported_providers::{arch::ArchProvider, node::NodeProvider},
};

pub mod arch;
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

        _ => None,
    }
}
