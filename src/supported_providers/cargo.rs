use crate::provider::Provider;
use anyhow::Result;
use std::collections::HashSet;

pub struct CargoProvider;

impl Provider for CargoProvider {
    fn get_name(&self) -> &str {
        "cargo"
    }

    fn install_command(&self) -> &str {
        "cargo install"
    }

    fn uninstall_command(&self) -> &str {
        "cargo uninstall"
    }

    fn list_command(&self) -> &str {
        "cargo install --list"
    }

    fn list_packages(&self) -> Result<HashSet<String>> {
        let stdout = self.output_command(self.list_command(), &HashSet::new())?;

        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty() && line.starts_with(|c: char| c.is_alphabetic()))
            .map(|line| line.split(" v").next().unwrap_or("").to_string())
            .collect())
    }
}
