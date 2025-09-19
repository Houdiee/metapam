use crate::provider::Provider;

pub struct BrewProvider;

impl Provider for BrewProvider {
    fn get_name(&self) -> &str {
        "brew"
    }

    fn install_command(&self) -> &str {
        "brew install"
    }

    fn uninstall_command(&self) -> &str {
        "brew uninstall"
    }

    fn list_command(&self) -> &str {
        "brew list --formula"
    }
}
