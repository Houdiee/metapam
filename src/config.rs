use crate::supported_providers::get_provider;
use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

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
    let config_path = get_provider_path(provider_name)?;
    let current_packages = list_packages_from_config(provider_name)?;
    let mut new_packages_to_write = new_packages
        .iter()
        .filter(|p| !current_packages.contains(*p))
        .cloned()
        .collect::<Vec<String>>();

    if new_packages_to_write.is_empty() {
        return Ok(());
    }

    new_packages_to_write.sort();

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&config_path)
        .with_context(|| format!("Failed to open config for provider `{provider_name}`"))?;

    for package in new_packages_to_write {
        writeln!(file, "{}", package)
            .with_context(|| format!("Failed to write to config for provider `{provider_name}`"))?;
    }

    Ok(())
}

pub fn remove_packages_from_config(
    provider_name: &str,
    packages_to_remove: &HashSet<String>,
) -> Result<()> {
    let config_path = get_provider_path(provider_name)?;
    let file = open_config(provider_name)?;
    let reader = BufReader::new(file);

    let mut lines_to_keep = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed_line = line.trim();

        if trimmed_line.starts_with('#') || trimmed_line.starts_with("//") {
            lines_to_keep.push(line);
            continue;
        }

        if !packages_to_remove.contains(trimmed_line) {
            lines_to_keep.push(line);
        }
    }

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&config_path)
        .with_context(|| format!("Failed to open config for provider `{provider_name}`"))?;

    for line in lines_to_keep {
        writeln!(file, "{}", line)
            .with_context(|| format!("Failed to write to config for provider `{provider_name}`"))?;
    }

    Ok(())
}

pub fn list_packages_from_config(provider_name: &str) -> Result<HashSet<String>> {
    let file = open_config(provider_name)?;
    let reader = BufReader::new(file);
    let mut packages = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed_line = line.trim();

        if trimmed_line.is_empty()
            || trimmed_line.starts_with('#')
            || trimmed_line.starts_with("//")
        {
            continue;
        }

        packages.insert(trimmed_line.to_string());
    }

    Ok(packages)
}
