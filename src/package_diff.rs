use std::{collections::HashSet, fmt::Display};

#[derive(Debug)]
pub struct PackageDiff {
    pub installed_not_declared: HashSet<String>,
    pub declared_not_installed: HashSet<String>,
}

impl Display for PackageDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "To be removed:")?;
        for pkg in &self.installed_not_declared {
            writeln!(f, "- {}", pkg)?;
        }

        writeln!(f, "To be installed:")?;
        for pkg in &self.declared_not_installed {
            writeln!(f, "+ {}", pkg)?;
        }

        Ok(())
    }
}
