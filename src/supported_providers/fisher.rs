use crate::provider::Provider;

pub struct FisherProvider;

impl Provider for FisherProvider {
    fn get_name(&self) -> &str {
        "fisher"
    }

    fn install_command(&self) -> &str {
        "fisher install"
    }

    fn uninstall_command(&self) -> &str {
        "fisher remove"
    }

    fn list_command(&self) -> &str {
        "fisher list"
    }
}
