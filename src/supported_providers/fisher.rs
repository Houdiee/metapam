use crate::provider::Provider;

pub struct FisherProvider;

impl Provider for FisherProvider {
    fn get_name(&self) -> &str {
        "fisher --command=fisher"
    }

    fn install_command(&self) -> &str {
        "fish --command=\"fisher install\""
    }

    fn uninstall_command(&self) -> &str {
        "fish --command=\"fisher remove\""
    }

    fn list_command(&self) -> &str {
        "fish --command=\"fisher list\""
    }
}
