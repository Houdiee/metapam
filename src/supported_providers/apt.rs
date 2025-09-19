use crate::provider::Provider;

pub struct AptProvider;

impl Provider for AptProvider {
    fn get_name(&self) -> &str {
        "apt"
    }

    fn install_command(&self) -> &str {
        "sudo apt install"
    }

    fn uninstall_command(&self) -> &str {
        "sudo apt purge"
    }

    fn list_command(&self) -> &str {
        "apt-mark showmanual"
    }
}
