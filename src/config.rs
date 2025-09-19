use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    fs::{self, File},
    path::PathBuf,
};

use crate::supported_providers::get_provider;

fn get_base_directory() -> Result<PathBuf> {
    let mut config_dir = dirs::config_dir().expect("Failed to get user's config directory");
    let base_dir_name = env!("CARGO_PKG_NAME");

    config_dir.push(base_dir_name);
    fs::create_dir_all(&config_dir)?;

    Ok(config_dir)
}

pub fn create_new_config(provider_name: &str) -> Result<File> {
    let file_path = get_base_directory()?.join(provider_name);
    let new_config = File::create(&file_path)?;
    Ok(new_config)
}

pub fn config_exists(provider_name: &str) -> bool {
    let file_path = get_base_directory()
        .expect("Failed to get configuration base directory")
        .join(provider_name);

    file_path.exists()
}

pub fn open_config(provider_name: &str) -> Result<File> {
    let base_dir = get_base_directory()?;
    let file_path = base_dir.join(provider_name);

    let config = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&file_path)
        .with_context(|| format!("Provider `{provider_name}` must be activated"))?;

    Ok(config)
}

pub fn get_provider_path(provider_name: &str) -> Result<PathBuf> {
    let base_dir = get_base_directory()?;
    Ok(base_dir.join(provider_name))
}

pub fn get_active_providers() -> Result<HashSet<String>> {
    let base_dir = get_base_directory()?;
    let mut providers = HashSet::new();

    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        if let Some(file_name) = entry.file_name().to_str() {
            let file_path = entry.path();
            if file_path.is_file() && get_provider(file_name).is_some() {
                providers.insert(file_name.to_string());
            }
        }
    }
    Ok(providers)
}

pub fn add_packages_to_config(provider_name: &str, new_packages: &HashSet<String>) -> Result<()> {
    todo!()
}

pub fn remove_packages_from_config(
    provider_name: &str,
    packages_to_remove: &HashSet<String>,
) -> Result<()> {
    todo!()
}

pub fn list_packages_from_config(provider_name: &str) -> Result<HashSet<String>> {
    todo!()
}
