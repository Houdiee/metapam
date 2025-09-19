use std::{collections::HashSet, process};

use anyhow::{Result, anyhow};

use crate::{config, package_diff::PackageDiff};

pub trait Provider {
    fn get_name(&self) -> &str;
    fn install_command(&self) -> &str;
    fn uninstall_command(&self) -> &str;
    fn list_command(&self) -> &str;
    fn activate(&self) -> Result<()> {
        config::create_new_config(self.get_name())?;
        config::add_packages_to_config(self.get_name(), &self.list_packages()?)?;
        Ok(())
    }

    fn list_packages(&self) -> Result<HashSet<String>> {
        let stdout = self.output_command(self.list_command(), &HashSet::new())?;

        Ok(stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|s| s.split_whitespace().next())
            .map(|s| s.to_string())
            .collect())
    }

    fn declare_packages(&self, packages: &HashSet<String>) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }
        self.spawn_command(self.install_command(), &packages)?;
        config::add_packages_to_config(self.get_name(), &packages)?;
        Ok(())
    }

    fn remove_packages(&self, packages: &HashSet<String>) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }
        self.spawn_command(self.uninstall_command(), &packages)?;
        config::remove_packages_from_config(self.get_name(), &packages)?;
        Ok(())
    }

    fn diff(&self) -> Result<PackageDiff> {
        let installed_pkgs = self.list_packages()?;
        let declared_pkgs = config::list_packages_from_config(self.get_name())?;

        let installed_not_declared = installed_pkgs.difference(&declared_pkgs).cloned().collect();
        let declared_not_installed = declared_pkgs.difference(&installed_pkgs).cloned().collect();

        Ok(PackageDiff {
            installed_not_declared,
            declared_not_installed,
        })
    }

    fn tidy(&self) -> Result<()> {
        let diff = self.diff()?;

        let declared_not_installed = diff.declared_not_installed;
        let installed_not_declared = diff.installed_not_declared;

        match (
            declared_not_installed.is_empty(),
            installed_not_declared.is_empty(),
        ) {
            (true, true) => Ok(()),

            (true, false) => {
                self.remove_packages(&installed_not_declared)?;
                Ok(())
            }

            (false, true) => {
                self.declare_packages(&declared_not_installed)?;
                Ok(())
            }

            (false, false) => {
                self.declare_packages(&declared_not_installed)?;
                self.remove_packages(&installed_not_declared)?;
                Ok(())
            }
        }
    }

    fn spawn_command(&self, command: &str, packages: &HashSet<String>) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let command_name = &parts[0];
        let command_args = &parts[1..];

        let mut child = process::Command::new(command_name)
            .args(command_args)
            .args(packages)
            .spawn()?;

        let status = child.wait()?;
        if !status.success() {
            return Err(anyhow!(
                "Command `{}` failed with exit code: {}",
                command_name,
                status.code().unwrap_or(-1)
            ));
        }

        Ok(())
    }

    fn output_command(&self, command: &str, packages: &HashSet<String>) -> Result<String> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let command_name = &parts[0];
        let command_args = &parts[1..];

        let output = process::Command::new(command_name)
            .args(command_args)
            .args(packages)
            .output()?;

        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout)
    }
}
