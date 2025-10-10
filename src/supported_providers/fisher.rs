use crate::provider::Provider;

pub struct FisherProvider;

impl Provider for FisherProvider {
    fn get_name(&self) -> &str {
        "fisher"
    }

    fn install_command(&self) -> &str {
        "fish -c \"fisher install\""
    }

    fn uninstall_command(&self) -> &str {
        "fish -c \"fisher remove\""
    }

    fn list_command(&self) -> &str {
        "fish -c \"fisher list\""
    }
}
