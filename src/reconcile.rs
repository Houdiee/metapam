use anyhow::{Result, bail};
use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

use crate::manager::Manager;
    use crate::manifest::grammar::PackageSpec;

/// A declared pin that does not match the installed version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repin {
    pub spec: PackageSpec,
    pub installed: String,
}

/// The set of  that would bring one manager in line with its manifest.
///
/// `status`, `apply --dry-run` and `apply` all render and execute *this* value,
/// so a preview cannot drift away from the action it previews.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reconciliation {
    pub manager: String,
    /// Declared but not installed.
    pub install: Vec<PackageSpec>,
    /// Installed at the wrong version.
    pub repin: Vec<Repin>,
    /// Installed but not declared.
    pub remove: Vec<String>,
}

impl Reconciliation {
    /// Whether applying this would change anything.
    pub fn has_work(&self) -> bool {
        !self.install.is_empty() || !self.repin.is_empty() || !self.remove.is_empty()
    }

    /// Packages to install, including repins.
    pub fn to_install(&self) -> Vec<PackageSpec> {
        let mut specs = self.install.clone();
        specs.extend(self.repin.iter().map(|change| change.spec.clone()));
        specs
    }
}

/// Reject a manifest whose pins this manager cannot carry out.
///
/// A pin mpm cannot honour is an error, not a warning: installing an arbitrary
/// version when an exact one was asked for is the kind of quiet substitution
/// this tool exists to avoid.
pub fn validate(manager: &dyn Manager, declared: &BTreeMap<String, PackageSpec>) -> Result<()> {
    let mut problems = Vec::new();

    for (name, spec) in declared {
        let Some(version) = &spec.version else { continue };

        if manager.name_selects_version(name) {
            // `node@20 20.11.0` on Homebrew: the name already chose a version.
            problems.push(format!(
                "`{name} {version}`: the name `{name}` already selects a version -- drop `{version}`"
            ));
        } else if !manager.supports_pinning() {
            problems.push(format!(
                "`{name} {version}`: {} cannot install a specific version -- declare `{name}` alone",
                manager.id()
            ));
        }
    }

    if !problems.is_empty() {
        bail!("{}", problems.join("\n  "));
    }

    // Here rather than at install time, so a version nothing can supply is
    // reported by `status` too, not only when mpm goes to act on it.
    let pinned: Vec<&PackageSpec> = declared.values().filter(|spec| spec.version.is_some()).collect();
    manager.check_pins(&pinned)
}

/// Compare declared state against installed state.
///
/// Pure: no process is spawned and no file is read, so the safety-critical
/// decision of *what gets uninstalled* is directly testable.
pub fn compute(
    manager: &str,
    declared: &BTreeMap<String, PackageSpec>,
    installed: &BTreeMap<String, PackageSpec>,
    supports_pinning: bool,
) -> Reconciliation {
    let mut changes = Reconciliation { manager: manager.to_string(), ..Reconciliation::default() };

    for (name, wanted) in declared {
        match installed.get(name) {
            None => changes.install.push(wanted.clone()),
            Some(present) => {
                // An unknown installed version counts as satisfied, rather than
                // reinstalling on every single run.
                if !supports_pinning {
                    continue;
                }
                if let (Some(want), Some(have)) = (&wanted.version, &present.version) {
                    if want != have {
                        changes.repin.push(Repin { spec: wanted.clone(), installed: have.clone() });
                    }
                }
            }
        }
    }

    for name in installed.keys() {
        if declared.contains_key(name) {
            continue;
        }
        changes.remove.push(name.clone());
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(specs: &[PackageSpec]) -> BTreeMap<String, PackageSpec> {
        specs.iter().map(|spec| (spec.name.clone(), spec.clone())).collect()
    }


    fn changes_of(
        declared: &[PackageSpec],
        installed: &[PackageSpec],
        pinning: bool,
    ) -> Reconciliation {
        compute("test", &map(declared), &map(installed), pinning)
    }

    #[test]
    fn matching_state_is_quiet() {
        let changes = changes_of(&[PackageSpec::new("vim")], &[PackageSpec::new("vim")], false);
        assert!(!changes.has_work());
        assert!(!changes.has_work());
    }

    #[test]
    fn declared_but_missing_is_installed() {
        let changes = changes_of(&[PackageSpec::new("vim")], &[], false);
        assert_eq!(changes.install, vec![PackageSpec::new("vim")]);
        assert!(changes.remove.is_empty());
    }

    #[test]
    fn installed_but_undeclared_is_removed() {
        let changes = changes_of(&[], &[PackageSpec::new("nano")], false);
        assert_eq!(changes.remove, vec!["nano"]);
        assert!(changes.install.is_empty());
    }

    #[test]
    fn a_pin_mismatch_is_a_repin() {
        let changes = changes_of(
            &[PackageSpec::pinned("ripgrep", "14.1.0")],
            &[PackageSpec::pinned("ripgrep", "14.0.0")],
            true,
        );
        assert_eq!(
            changes.repin,
            vec![Repin { spec: PackageSpec::pinned("ripgrep", "14.1.0"), installed: "14.0.0".into() }]
        );
        assert!(changes.install.is_empty());
        assert!(changes.remove.is_empty());
    }

    #[test]
    fn a_satisfied_pin_is_no_work() {
        let changes = changes_of(
            &[PackageSpec::pinned("ripgrep", "14.1.0")],
            &[PackageSpec::pinned("ripgrep", "14.1.0")],
            true,
        );
        assert!(!changes.has_work());
    }

    #[test]
    fn an_unknown_installed_version_is_assumed_satisfied() {
        // apt reports manual packages without versions. Treating that as drift
        // would reinstall every pinned package on every run.
        let changes = changes_of(
            &[PackageSpec::pinned("ripgrep", "14.1.0")],
            &[PackageSpec::new("ripgrep")],
            true,
        );
        assert!(!changes.has_work());
    }

    #[test]
    fn repins_are_installed_alongside_new_packages() {
        let changes = changes_of(
            &[PackageSpec::new("bat"), PackageSpec::pinned("ripgrep", "14.1.0")],
            &[PackageSpec::pinned("ripgrep", "14.0.0")],
            true,
        );
        assert_eq!(
            changes.to_install(),
            vec![PackageSpec::new("bat"), PackageSpec::pinned("ripgrep", "14.1.0")]
        );
    }

    #[test]
    fn output_is_ordered_not_hash_ordered() {
        // Output must be identical run to run, or it cannot be diffed or scripted.
        let declared = [PackageSpec::new("zsh"), PackageSpec::new("bat"), PackageSpec::new("micro")];
        let first = changes_of(&declared, &[], false);
        let second = changes_of(&declared, &[], false);
        assert_eq!(first.install, second.install);
        assert_eq!(
            first.install,
            vec![PackageSpec::new("bat"), PackageSpec::new("micro"), PackageSpec::new("zsh")]
        );
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::manager;

    fn declared(specs: &[PackageSpec]) -> BTreeMap<String, PackageSpec> {
        specs.iter().map(|spec| (spec.name.clone(), spec.clone())).collect()
    }

    #[test]
    fn a_pin_on_a_manager_that_cannot_pin_is_an_error() {
        let brew = manager::get("brew").expect("brew");
        let error = validate(brew.as_ref(), &declared(&[PackageSpec::pinned("ripgrep", "14.1.0")]))
            .expect_err("must reject");
        assert!(error.to_string().contains("cannot install a specific version"));
    }

    #[test]
    fn a_pin_on_a_manager_that_can_pin_is_accepted() {
        let cargo = manager::get("cargo").expect("cargo");
        validate(cargo.as_ref(), &declared(&[PackageSpec::pinned("ripgrep", "14.1.0")])).expect("accepted");
    }

    #[test]
    fn a_versioned_formula_plus_a_pin_is_a_contradiction() {
        let brew = manager::get("brew").expect("brew");
        let error = validate(brew.as_ref(), &declared(&[PackageSpec::pinned("node@20", "20.11.0")]))
            .expect_err("must reject");
        assert!(error.to_string().contains("already selects a version"));
    }

    #[test]
    fn a_versioned_formula_alone_is_fine() {
        let brew = manager::get("brew").expect("brew");
        validate(brew.as_ref(), &declared(&[PackageSpec::new("node@20")])).expect("accepted");
    }

    #[test]
    fn unpinned_entries_are_always_fine() {
        for id in manager::ALL {
            let manager = manager::get(id).expect("registered");
            validate(manager.as_ref(), &declared(&[PackageSpec::new("vim")]))
                .unwrap_or_else(|error| panic!("`{id}` rejected an unpinned package: {error}"));
        }
    }

    #[test]
    fn every_problem_is_reported_at_once() {
        let brew = manager::get("brew").expect("brew");
        let error = validate(
            brew.as_ref(),
            &declared(&[PackageSpec::pinned("ripgrep", "14.1.0"), PackageSpec::pinned("bat", "0.24.0")]),
        )
        .expect_err("must reject");
        assert!(error.to_string().contains("ripgrep"));
        assert!(error.to_string().contains("bat"));
    }
}

// ------------------------------------------------------------- presentation


fn colored() -> bool {
    static COLORED: OnceLock<bool> = OnceLock::new();
    *COLORED.get_or_init(|| {
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        std::io::stdout().is_terminal()
    })
}

fn paint(code: &str, text: &str) -> String {
    if colored() { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_string() }
}

pub fn bold(text: &str) -> String {
    paint("1", text)
}

pub fn dim(text: &str) -> String {
    paint("2", text)
}

/// A line that must not read as reassurance.
pub fn problem(text: &str) -> String {
    paint("33", text)
}

fn green(text: &str) -> String {
    paint("32", text)
}

fn red(text: &str) -> String {
    paint("31", text)
}

fn yellow(text: &str) -> String {
    paint("33", text)
}

/// Render a changes as the list of changes it represents.
///
/// Returns an empty string when there is nothing at all to say, so callers can
/// stay silent on a clean machine.
pub fn render(changes: &Reconciliation) -> String {
    if !changes.has_work() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str(&bold(&changes.manager));
    out.push('\n');

    for spec in &changes.install {
        out.push_str(&format!("  {} {}\n", green("+"), spec));
    }
    for change in &changes.repin {
        out.push_str(&format!(
            "  {} {} {}\n",
            yellow("~"),
            change.spec,
            dim(&format!("(installed {})", change.installed))
        ));
    }
    for name in &changes.remove {
        out.push_str(&format!("  {} {}\n", red("-"), name));
    }

    out
}

/// One-line summary of what a set of sets adds up to.
pub fn summarize(sets: &[Reconciliation]) -> String {
    let install: usize = sets.iter().map(|changes| changes.install.len()).sum();
    let repin: usize = sets.iter().map(|changes| changes.repin.len()).sum();
    let remove: usize = sets.iter().map(|changes| changes.remove.len()).sum();

    let mut parts = Vec::new();
    if install > 0 {
        parts.push(green(&format!("{install} to install")));
    }
    if repin > 0 {
        parts.push(yellow(&format!("{repin} to change")));
    }
    if remove > 0 {
        parts.push(red(&format!("{remove} to remove")));
    }
    if parts.is_empty() {
        return green("everything matches your manifests").to_string();
    }
    parts.join(", ")
}

/// Ask for confirmation on stdin.
///
/// A non-interactive stdin answers *no*: a pipeline must pass `--yes`
/// explicitly rather than have consent inferred from the absence of a terminal.
pub fn confirm(question: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        println!("{question} [y/N] n {}", dim("(stdin is not a terminal; pass --yes to proceed)"));
        return Ok(false);
    }

    print!("{question} [y/N] ");
    std::io::stdout().flush()?;

    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer)? == 0 {
        println!();
        return Ok(false);
    }
    let answer = answer.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    use crate::reconcile::Repin;
    use crate::manifest::grammar::PackageSpec;

    fn changes() -> Reconciliation {
        Reconciliation {
            manager: "pacman".into(),
            install: vec![PackageSpec::new("ripgrep")],
            repin: vec![Repin { spec: PackageSpec::pinned("bat", "0.24.0"), installed: "0.23.0".into() }],
            remove: vec!["nano".into()],
        }
    }

    #[test]
    fn a_clean_plan_renders_nothing() {
        let quiet = Reconciliation { manager: "pacman".into(), ..Reconciliation::default() };
        assert!(render(&quiet).is_empty());
    }

    #[test]
    fn every_bucket_appears_in_the_rendering() {
        let text = render(&changes());
        assert!(text.contains("ripgrep"));
        assert!(text.contains("bat 0.24.0"));
        assert!(text.contains("installed 0.23.0"));
        assert!(text.contains("nano"));

    }

    #[test]
    fn the_summary_counts_each_bucket() {
        let text = summarize(&[changes()]);
        assert!(text.contains("1 to install"));
        assert!(text.contains("1 to change"));
        assert!(text.contains("1 to remove"));

    }

    #[test]
    fn a_clean_summary_says_so() {
        let quiet = Reconciliation { manager: "pacman".into(), ..Reconciliation::default() };
        assert!(summarize(&[quiet]).contains("everything matches"));
    }
}
