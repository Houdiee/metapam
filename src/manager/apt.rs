use anyhow::Result;

use crate::manager::Manager;
use crate::exec::Invocation;
use crate::manifest::grammar::PackageSpec;

pub struct Apt;

impl Manager for Apt {
    fn id(&self) -> &'static str {
        "apt"
    }

    fn install(&self, packages: &[PackageSpec]) -> Result<Vec<Invocation>> {
        if packages.is_empty() {
            return Ok(Vec::new());
        }
        // apt-get rather than apt: apt's own manual warns that its CLI is not
        // stable for scripting.
        //
        // -y is passed for installs because mpm has already shown the plan and
        // taken confirmation. It is deliberately *not* passed for removals
        // below, where apt's own prompt is a useful second gate.
        Ok(vec![
            Invocation::new("apt-get")
                .as_root()
                .args(["install", "-y"])
                .args(packages.iter().map(spec_arg)),
        ])
    }

    fn uninstall(&self, names: &[String]) -> Vec<Invocation> {
        if names.is_empty() {
            return Vec::new();
        }
        // `remove`, never `purge`: purge also deletes the package's configuration
        // files, which is not something a package-list sync should decide.
        vec![Invocation::new("apt-get").as_root().arg("remove").args(names.iter().cloned())]
    }

    fn list(&self) -> Invocation {
        // Manual packages only; the rest are dependencies.
        Invocation::new("apt-mark").arg("showmanual")
    }

    fn parse_list(&self, stdout: &str) -> Vec<PackageSpec> {
        // apt-mark reports names without versions.
        stdout
            .lines()
            .filter_map(|line| {
                let name = line.split_whitespace().next()?;
                Some(PackageSpec::new(name))
            })
            .collect()
    }

    /// Declaring a version is rejected: Debian and Ubuntu prune old versions
    /// from their archives, so `name=version` works today and fails once the
    /// version is dropped. That is not a manifest that keeps its promise.
    fn supports_pinning(&self) -> bool {
        false
    }
}

/// apt spells a version constraint `name=version`.
fn spec_arg(spec: &PackageSpec) -> String {
    match &spec.version {
        Some(version) => format!("{}={}", spec.name, version),
        None => spec.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showmanual_output_is_parsed() {
        let out = "git\nripgrep\nvim\n";
        assert_eq!(
            Apt.parse_list(out),
            vec![PackageSpec::new("git"), PackageSpec::new("ripgrep"), PackageSpec::new("vim")]
        );
    }

    #[test]
    fn pins_use_equals_syntax() {
        let cmds = Apt.install(&[PackageSpec::pinned("ripgrep", "14.1.0"), PackageSpec::new("vim")]).expect("builds");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].args, vec!["install", "-y", "ripgrep=14.1.0", "vim"]);
    }

    #[test]
    fn removal_never_purges() {
        let names = vec!["vim".to_string()];
        assert_eq!(Apt.uninstall(&names)[0].args[0], "remove");
    }

    #[test]
    fn removal_is_not_auto_confirmed() {
        let names = vec!["vim".to_string()];
        let cmd = &Apt.uninstall(&names)[0];
        assert!(!cmd.args.iter().any(|a| a == "-y"));
    }

    #[test]
    fn install_and_remove_need_root() {
        assert!(Apt.install(&[PackageSpec::new("vim")]).expect("builds")[0].needs_root);
        assert!(Apt.uninstall(&["vim".to_string()])[0].needs_root);
    }
}
