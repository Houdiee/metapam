use anyhow::Result;
use std::collections::HashSet;

use crate::provider::Provider;

#[allow(dead_code)]
pub struct NodeProvider {
    pub manager: NodeManager,
}

#[allow(dead_code)]
pub enum NodeManager {
    Npm,
    Pnpm,
    Bun,
    Yarn,
}

impl NodeProvider {
    fn pnpm_list_packages(&self, stdout: String) -> HashSet<String> {
        stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .skip_while(|line| !line.contains("dependencies:"))
            .skip(1)
            .filter_map(|s| s.split_whitespace().next())
            .map(|s| s.to_string())
            .collect()
    }

    fn npm_list_packages(&self, stdout: String) -> HashSet<String> {
        stdout
            .lines()
            .skip(1)
            .filter(|line| line.contains("├──") || line.contains("└──"))
            .filter_map(|s| s.split('@').next())
            .map(|s| s.to_string())
            .collect()
    }

    fn yarn_list_packages(&self, stdout: String) -> Result<HashSet<String>> {
        todo!()
    }

    fn bun_list_packages(&self, stdout: String) -> Result<HashSet<String>> {
        todo!()
    }
}

#[allow(dead_code)]
impl Provider for NodeProvider {
    fn get_name(&self) -> &str {
        match self.manager {
            NodeManager::Npm => "npm",
            NodeManager::Pnpm => "pnpm",
            NodeManager::Bun => "bun",
            NodeManager::Yarn => "yarn",
        }
    }

    fn install_command(&self) -> &str {
        match self.manager {
            NodeManager::Npm => "npm install -g",
            NodeManager::Pnpm => "pnpm install -g",
            NodeManager::Bun => "bun install -g",
            NodeManager::Yarn => "yarn global add",
        }
    }

    fn uninstall_command(&self) -> &str {
        match self.manager {
            NodeManager::Npm => "npm uninstall -g",
            NodeManager::Pnpm => "pnpm uninstall -g",
            NodeManager::Bun => "bun uninstall -g",
            NodeManager::Yarn => "yarn global remove",
        }
    }

    fn update_command(&self) -> &str {
        match self.manager {
            NodeManager::Npm => "npm update -g",
            NodeManager::Pnpm => "pnpm update -g",
            NodeManager::Bun => "bun update -g",
            NodeManager::Yarn => "yarn global upgrade",
        }
    }

    fn list_command(&self) -> &str {
        match self.manager {
            NodeManager::Npm => "npm list -g --depth=0",
            NodeManager::Pnpm => "pnpm list -g --depth=0",
            NodeManager::Bun => "bun list -g --depth=0",
            NodeManager::Yarn => "yarn global list",
        }
    }
}
