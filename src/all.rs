use crate::{
    config::{self},
    supported_providers::{self, SUPPORTED_PROVIDERS, get_provider},
};
use anyhow::{Context, Result};
use which::which;

pub fn activate_all_supported() -> Result<()> {
    for supported in SUPPORTED_PROVIDERS {
        let provider =
            get_provider(&supported).with_context(|| format!("The provider {supported} should be supported but has no concrete implementation"))?;

        match which(supported) {
            Ok(_) => {
                if !config::config_exists(provider.get_name()) {
                    provider.activate()?;
                    println!("Activated provider `{supported}`");
                } else {
                    println!("Found existing config for `{supported}`. Skipping...");
                }
            }
            Err(_) => {
                println!("Provider `{supported}` not found in filesystem. Skipping...")
            }
        }
    }
    Ok(())
}

pub fn diff_all_active() -> Result<()> {
    for supported in SUPPORTED_PROVIDERS {
        let provider =
            get_provider(&supported).with_context(|| format!("The provider {supported} should be supported but has no concrete implementation"))?;

        if config::config_exists(provider.get_name()) {
            let diff = provider.diff()?;
            if !diff.installed_not_declared.is_empty() || !diff.declared_not_installed.is_empty() {
                println!("Diff for provider `{}`", provider.get_name());
                println!("{}", diff);
            }
        }
    }
    Ok(())
}

pub fn tidy_all_active() -> Result<()> {
    for supported in SUPPORTED_PROVIDERS {
        let provider =
            get_provider(&supported).with_context(|| format!("The provider {supported} should be supported but has no concrete implementation"))?;

        if config::config_exists(provider.get_name()) {
            provider.tidy()?;
        }
    }
    Ok(())
}
