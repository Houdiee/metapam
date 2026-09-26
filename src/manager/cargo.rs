use anyhow::Result;

use crate::manager::{Manager, partition_pinned};
use crate::exec::Invocation;
use crate::manifest::grammar::PackageSpec;

pub struct Cargo;

impl Manager for Cargo {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn install(&self, packages: &[PackageSpec]) -> Result<Vec<Invocation>> {
        // `--version` binds to the whole invocation, so every pinned crate
        // needs its own command. Unpinned crates still go out in one batch.
        let (pinned, loose) = partition_pinned(packages);
        let mut commands = Vec::new();

        if !loose.is_empty() {
            commands.push(
                Invocation::new("cargo").arg("install").args(loose.iter().map(|s| s.name.clone())),
            );
        }
        for spec in pinned {
            let version = spec.version.as_deref().unwrap_or_default();
            commands.push(
                Invocation::new("cargo").arg("install").arg(spec.name.as_str()).arg("--version").arg(version),
            );
        }
        Ok(commands)
    }

    fn uninstall(&self, names: &[String]) -> Vec<Invocation> {
        if names.is_empty() {
            return Vec::new();
        }
        vec![Invocation::new("cargo").arg("uninstall").args(names.iter().cloned())]
    }

    fn list(&self) -> Invocation {
        Invocation::new("cargo").args(["install", "--list"])
    }

    fn parse_list(&self, stdout: &str) -> Vec<PackageSpec> {
        // Crates start at column zero as `name vX.Y.Z:`; their binaries follow,
        // indented.
        stdout
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with(char::is_whitespace))
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let name = parts.next()?;
                let version = parts
                    .next()
                    .map(|v| v.trim_end_matches(':').trim_start_matches('v'))
                    .filter(|v| !v.is_empty());
                Some(match version {
                    Some(version) => PackageSpec::pinned(name, version),
                    None => PackageSpec::new(name),
                })
            })
            .collect()
    }

    fn supports_pinning(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\
cargo-edit v0.12.2:
    cargo-add
    cargo-rm
ripgrep v14.1.0:
    rg
";

    #[test]
    fn crate_lines_are_parsed_and_binaries_ignored() {
        assert_eq!(
            Cargo.parse_list(LIST),
            vec![PackageSpec::pinned("cargo-edit", "0.12.2"), PackageSpec::pinned("ripgrep", "14.1.0")]
        );
    }

    #[test]
    fn pinned_crates_get_one_command_each() {
        // `cargo install a b --version 1.0` would apply the version to both.
        let cmds = Cargo.install(&[
            PackageSpec::new("bat"),
            PackageSpec::pinned("ripgrep", "14.1.0"),
            PackageSpec::pinned("fd-find", "10.1.0"),
        ]).expect("builds");
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].args, vec!["install", "bat"]);
        assert_eq!(cmds[1].args, vec!["install", "ripgrep", "--version", "14.1.0"]);
        assert_eq!(cmds[2].args, vec!["install", "fd-find", "--version", "10.1.0"]);
    }

    #[test]
    fn unpinned_crates_share_one_command() {
        let cmds = Cargo.install(&[PackageSpec::new("bat"), PackageSpec::new("fd-find")]).expect("builds");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].args, vec!["install", "bat", "fd-find"]);
    }

    #[test]
    fn nothing_to_install_means_no_commands() {
        assert!(Cargo.install(&[]).expect("builds").is_empty());
    }
}
