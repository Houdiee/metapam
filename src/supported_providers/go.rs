use crate::{config, provider::Provider};
use anyhow::{Result, anyhow};
use std::{collections::HashSet, fs, path::PathBuf};

pub struct GoProvider;

impl GoProvider {
    fn get_go_bin_path(&self) -> Result<PathBuf> {
        let go_path = self
            .output_command("go env GOPATH", &HashSet::new())?
            .trim()
            .to_string();

        let mut bin_path = PathBuf::from(&go_path);
        bin_path.push("bin");

        if !bin_path.exists() {
            return Err(anyhow!("failed to get GOPATH/bin"));
        }

        Ok(bin_path)
    }
}

impl Provider for GoProvider {
    fn get_name(&self) -> &str {
        "go"
    }

    fn install_command(&self) -> &str {
        "go install"
    }

    fn uninstall_command(&self) -> &str {
        ""
    }

    fn list_command(&self) -> &str {
        ""
    }

    fn list_packages(&self) -> Result<HashSet<String>> {
        let bin_path = self.get_go_bin_path()?;
        let mut pkgs = HashSet::new();
        for entry in fs::read_dir(&bin_path)? {
            let file_name = entry?
                .file_name()
                .into_string()
                .expect("Failed to convert OsString to String");

            pkgs.insert(file_name);
        }

        Ok(pkgs)
    }

    fn remove_packages(&self, packages: &HashSet<String>) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let bin_path = self.get_go_bin_path()?;
        for entry in fs::read_dir(&bin_path)? {
            let file_name = entry?
                .file_name()
                .into_string()
                .expect("Failed to convert OsString to String");

            if packages.contains(&file_name) {
                let file_path = bin_path.join(&file_name);
                fs::remove_file(file_path)?;
            }
        }
        config::remove_packages_from_config(self.get_name(), &packages)?;
        Ok(())
    }
}
