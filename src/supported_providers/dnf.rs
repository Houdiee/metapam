use crate::provider::Provider;

pub struct DnfProvider;

impl Provider for DnfProvider {
    fn get_name(&self) -> &str {
        "dnf"
    }

    fn install_command(&self) -> &str {
        "sudo dnf install"
    }

    fn uninstall_command(&self) -> &str {
        "sudo dnf remove"
    }

    // TODO FIX THIS
    fn list_command(&self) -> &str {
        "dnf list installed"
    }
}
