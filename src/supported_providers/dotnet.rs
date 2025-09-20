use crate::provider::Provider;
use anyhow::Result;
use std::collections::HashSet;

pub struct DotnetProvider;

impl Provider for DotnetProvider {
    fn get_name(&self) -> &str {
        "dotnet"
    }

    fn install_command(&self) -> &str {
        "dotnet tool install -g"
    }

    fn uninstall_command(&self) -> &str {
        "dotnet tool uninstall -g"
    }

    fn list_command(&self) -> &str {
        "dotnet tool list -g"
    }

    fn list_packages(&self) -> Result<HashSet<String>> {
        let stdout = self.output_command(self.list_command(), &HashSet::new())?;

        Ok(stdout
            .lines()
            .skip(2)
            .filter_map(|line| {
                let cleaned_line = line.trim();

                let parts: Vec<&str> = cleaned_line.split_whitespace().collect();
                if let Some(package_id) = parts.get(0) {
                    Some(package_id.to_string())
                } else {
                    None
                }
            })
            .collect())
    }
}
