use crate::provider::Provider;

#[allow(dead_code)]
pub struct ArchProvider {
    pub manager: ArchManager,
}

#[allow(dead_code)]
pub enum ArchManager {
    Pacman,
    Paru,
    Yay,
}

#[allow(dead_code)]
impl Provider for ArchProvider {
    fn get_name(&self) -> &str {
        match self.manager {
            ArchManager::Pacman => "pacman",
            ArchManager::Paru => "paru",
            ArchManager::Yay => "yay",
        }
    }

    fn install_command(&self) -> &str {
        match self.manager {
            ArchManager::Pacman => "sudo pacman -S",
            ArchManager::Paru => "paru -S",
            ArchManager::Yay => "yay -S",
        }
    }

    fn uninstall_command(&self) -> &str {
        match self.manager {
            ArchManager::Pacman => "sudo pacman -Rns",
            ArchManager::Paru => "paru -Rns",
            ArchManager::Yay => "yay -Rns",
        }
    }

    fn list_command(&self) -> &str {
        match self.manager {
            ArchManager::Pacman => "pacman -Qqe",
            ArchManager::Paru => "paru -Qqe",
            ArchManager::Yay => "yay -Qqe",
        }
    }
}
