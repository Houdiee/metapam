use anyhow::{Context, Result, anyhow, bail};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use crate::exec::Invocation;
use crate::manager::{self, Manager};
use crate::manifest::file::ManifestFile;
use crate::manifest::grammar::PackageSpec;
use crate::manifest::{Layer, Layout, hostname};
use crate::reconcile::{self, Reconciliation};

/// Everything mpm needs to know about this machine.
pub struct Ctx {
    pub layout: Layout,
    pub host: String,

}

impl Ctx {
    pub fn discover() -> Result<Self> {
        let layout = Layout::discover()?;

        let host = hostname()?;
        warn_unmatched_host(&layout, &host);
        Ok(Self { layout, host })
    }
}


/// Warn when a `hosts/` tree exists but nothing in it is for this machine.
///
/// A mistyped directory is otherwise silent: its packages simply never count as
/// declared, and every one of them becomes a removal candidate.
fn warn_unmatched_host(layout: &Layout, host: &str) {
    let Some(known) = layout.known_hosts() else { return };
    if known.is_empty() || known.iter().any(|name| name == host) {
        return;
    }
    eprintln!(
        "{} no host layer for `{host}`; `hosts/` has {}",
        reconcile::problem("warning:"),
        known.join(", ")
    );
}

#[derive(Debug, Clone, Default)]
pub struct ApplyOpts {
    /// Show the plan and stop.
    pub dry_run: bool,
    /// Skip the confirmation prompt.
    pub yes: bool,
}

// ---------------------------------------------------------------- manager set

fn require(id: &str) -> Result<Box<dyn Manager>> {
    manager::get(id).ok_or_else(|| anyhow!("unknown package manager `{id}` (see `mpm managers`)"))
}

fn require_present(id: &str) -> Result<Box<dyn Manager>> {
    let found = require(id)?;
    if !manager::present(found.as_ref()) {
        bail!("`{id}` is not installed on this machine");
    }
    Ok(found)
}

fn present_by_id(id: &str) -> bool {
    manager::get(id).is_some_and(|found| manager::present(found.as_ref()))
}

/// Managers to act on: the one named, else every managed one present here.
fn selected(ctx: &Ctx, requested: Option<&str>) -> Result<Vec<String>> {
    match requested {
        Some(id) => {
            require_present(id)?;
            Ok(vec![id.to_string()])
        }
        None => Ok(manager::ALL
            .iter()
            .copied()
            .filter(|id| present_by_id(id) && ctx.layout.is_managed(id, &ctx.host))
            .map(str::to_string)
            .collect()),
    }
}

// -------------------------------------------------------------------- reading

fn installed_map(manager: &dyn Manager) -> Result<BTreeMap<String, PackageSpec>> {
    let stdout = manager.list().capture()?;
    Ok(manager.parse_list(&stdout).into_iter().map(|spec| (spec.name.clone(), spec)).collect())
}

/// Reconcile one manager against its manifest.
///
/// Takes the manager as a parameter rather than looking it up, so tests can
/// drive this with a stand-in instead of a real package manager.
fn reconcile_manager(
    layout: &Layout,
    host: &str,

    manager: &dyn Manager,
) -> Result<Reconciliation> {
    let id = manager.id();
    let resolved = layout.resolve(id, host)?;

    // A version this manager cannot honour is an error, not a warning.
    reconcile::validate(manager, &resolved.declared)?;

    let installed = installed_map(manager)
        .with_context(|| format!("could not list installed packages for `{id}`"))?;

    Ok(reconcile::compute(
        id,
        &resolved.declared,
        &installed,
        manager.supports_pinning(),
    ))
}

/// Reconcile every manager concurrently, returning results in `ids` order so
/// the output is identical from one run to the next.
fn reconcile_all(ctx: &Ctx, ids: &[String]) -> Vec<(String, Result<Reconciliation>)> {
    let mut handles = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.clone();
        let layout = ctx.layout.clone();
        let host = ctx.host.clone();
        handles.push(thread::spawn(move || {
            let manager = require(&id)?;
            reconcile_manager(&layout, &host, manager.as_ref())
        }));
    }

    ids.iter()
        .cloned()
        .zip(handles)
        .map(|(id, handle)| {
            let result =
                handle.join().unwrap_or_else(|_| Err(anyhow!("the worker for `{id}` panicked")));
            (id, result)
        })
        .collect()
}

fn nothing_managed(ctx: &Ctx) -> String {
    format!(
        "no package managers are managed yet.\nRun `mpm adopt` to put what is installed under management in {}",
        ctx.layout.root().display()
    )
}

// --------------------------------------------------------------------- status

/// Print drift. Returns true when the machine does not match its manifests.
pub fn status(ctx: &Ctx, requested: Option<&str>) -> Result<bool> {
    let ids = selected(ctx, requested)?;
    if ids.is_empty() {
        println!("{}", nothing_managed(ctx));
        return Ok(false);
    }

    let mut results = Vec::new();
    let mut failed = false;

    for (id, result) in reconcile_all(ctx, &ids) {
        match result {
            Ok(changes) => {
                let text = reconcile::render(&changes);
                if !text.is_empty() {
                    println!("{text}");
                }
                results.push(changes);
            }
            Err(error) => {
                eprintln!("{}: {error:#}", reconcile::bold(&id));
                failed = true;
            }
        }
    }

    // Never claim the machine is clean while a manager could not be read: an
    // unreadable manager is unknown state, not matching state.
    if failed {
        let unreadable = ids.len() - results.len();
        let rest = if results.is_empty() {
            String::new()
        } else {
            format!("; the rest: {}", reconcile::summarize(&results))
        };
        println!("{}", reconcile::problem(&format!("{unreadable} manager(s) could not be read{rest}")));
    } else {
        println!("{}", reconcile::summarize(&results));
    }

    Ok(results.iter().any(Reconciliation::has_work) || failed)
}

// ---------------------------------------------------------------------- apply

pub fn apply(ctx: &Ctx, requested: Option<&str>, opts: &ApplyOpts) -> Result<()> {
    let ids = selected(ctx, requested)?;
    if ids.is_empty() {
        println!("{}", nothing_managed(ctx));
        return Ok(());
    }

    // Acting on a half-known system is how the wrong things get uninstalled.
    let mut results = Vec::new();
    for (id, result) in reconcile_all(ctx, &ids) {
        results.push(result.with_context(|| format!("refusing to apply: `{id}` could not be inspected"))?);
    }

    for changes in &results {
        let text = reconcile::render(changes);
        if !text.is_empty() {
            println!("{text}");
        }
    }

    println!("{}", reconcile::summarize(&results));
    if !results.iter().any(Reconciliation::has_work) {
        return Ok(());
    }

    // A version mpm cannot obtain must stop the run before the prompt, not half
    // way through it.
    let mut queued = Vec::new();
    for changes in &results {
        if !changes.has_work() {
            continue;
        }
        let commands = commands_for(changes)
            .with_context(|| format!("refusing to apply: `{}`", changes.manager))?;
        queued.push((changes.manager.clone(), commands));
    }

    if opts.dry_run {
        println!("{}", reconcile::dim("dry run: nothing was changed"));
        return Ok(());
    }

    if !opts.yes && !reconcile::confirm("Apply this plan?")? {
        println!("{}", reconcile::dim("aborted"));
        return Ok(());
    }

    run_in_parallel(queued)
}

/// Drive every manager at once, reporting each as it finishes.
///
/// Managers share nothing, so there is no reason for `npm` to wait on `pacman`.
/// Workers capture their output instead of writing to the terminal, because
/// concurrent package managers would otherwise interleave into noise.
fn run_in_parallel(queued: Vec<(String, Vec<Invocation>)>) -> Result<()> {
    preauthorize(&queued)?;

    let (completions, finished) = mpsc::channel();
    for (id, commands) in queued {
        let completions = completions.clone();
        thread::spawn(move || {
            let mut transcript = String::new();
            let mut result = Ok(());

            for command in &commands {
                transcript.push_str(&format!("  $ {}\n", command.display()));
                let (output, outcome) = command.run_captured();
                transcript.push_str(&indent(&output));
                if let Err(error) = outcome {
                    result = Err(error);
                    break;
                }
            }
            let _ = completions.send(Completion { manager: id, transcript, result });
        });
    }
    drop(completions);

    let mut failures = Vec::new();
    for completion in finished {
        match completion.result {
            Ok(()) => {
                println!("{} {}", reconcile::bold(&completion.manager), reconcile::dim("done"));
                print!("{}", reconcile::dim(&completion.transcript));
            }
            Err(error) => {
                println!("{} {}", reconcile::bold(&completion.manager), reconcile::problem("failed"));
                print!("{}", completion.transcript);
                eprintln!("{}: {error:#}", reconcile::bold(&completion.manager));
                failures.push(completion.manager);
            }
        }
    }

    if !failures.is_empty() {
        failures.sort();
        bail!("failed to apply: {}", failures.join(", "));
    }
    Ok(())
}

struct Completion {
    manager: String,
    transcript: String,
    result: Result<()>,
}

/// Ask for the elevator's password once, before any worker starts.
///
/// Workers capture their output, so an elevator prompting inside one would
/// block with nothing on screen to explain why.
fn preauthorize(queued: &[(String, Vec<Invocation>)]) -> Result<()> {
    let needs_root =
        queued.iter().flat_map(|(_, commands)| commands).any(|command| command.needs_root);
    if !needs_root {
        return Ok(());
    }
    Invocation::new("true").as_root().run().context("could not obtain root")
}

fn indent(output: &str) -> String {
    output.lines().map(|line| format!("    {line}\n")).collect()
}

/// Build every command a reconciliation implies, without running any of them.
///
/// Separated from execution so that a version mpm cannot actually obtain is
/// discovered *before* the confirmation prompt, rather than half way through a
/// run that has already changed the system.
fn commands_for(changes: &Reconciliation) -> Result<Vec<Invocation>> {
    let manager = require(&changes.manager)?;

    // Install first: a package moving between managers is never briefly absent,
    // and an interrupted run leaves more installed rather than less.
    let mut commands = manager.install(&changes.to_install())?;
    commands.extend(manager.uninstall(&changes.remove));
    Ok(commands)
}

fn run_visibly(command: &Invocation) -> Result<()> {
    println!("{} {}", reconcile::dim("$"), command.display());
    command.run()
}

// ---------------------------------------------------------------------- adopt

/// Put what is installed under management, merging with what the manifest says.
pub fn adopt(ctx: &Ctx, requested: Option<&str>, layer: &Layer) -> Result<()> {
    // Every manager present, not only managed ones: adopting is how a manager
    // becomes managed.
    let ids: Vec<String> = match requested {
        Some(id) => {
            require_present(id)?;
            vec![id.to_string()]
        }
        None => manager::ALL.iter().copied().filter(|id| present_by_id(id)).map(str::to_string).collect(),
    };

    if ids.is_empty() {
        println!("none of the supported package managers were found on this machine");
        return Ok(());
    }

    for id in ids {
        let found = require(&id)?;
        let path = ctx.layout.path_for(layer, &id);
        let resolved = ctx.layout.resolve(&id, &ctx.host)?;
        let installed = installed_map(found.as_ref())
            .with_context(|| format!("could not list installed packages for `{id}`"))?;

        let mut manifest = ManifestFile::load(&path)?;
        let existed = manifest.exists();
        let mut added = Vec::new();

        for name in installed.keys() {
            // Declared by any layer already: adding it here would duplicate it.
            if resolved.declared.contains_key(name) {
                continue;
            }
            // Without a version on purpose: pinning everything on adoption would
            // freeze the whole system at today's versions.
            if manifest.declare(&PackageSpec::new(name)) {
                added.push(name.clone());
            }
        }

        if !added.is_empty() || !existed {
            manifest.save()?;
        }

        match added.len() {
            0 if existed => println!("{} already up to date", reconcile::bold(&id)),
            0 => println!("{} now managed at {}", reconcile::bold(&id), path.display()),
            count => {
                println!("{} +{count} into {}", reconcile::bold(&id), path.display());
                for name in &added {
                    println!("  {name}");
                }
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------- manifest edits

/// Declare packages. Installing them is `mpm apply`'s job.
///
/// With `pin`, each package is recorded at the version installed right now.
/// Pinning is always a deliberate act: nothing else in mpm ever writes a version
/// into a manifest, so ordinary upgrades are none of its business.
pub fn add(ctx: &Ctx, id: &str, packages: &[String], layer: &Layer, pin: bool) -> Result<()> {
    let manager = require(id)?;
    let mut specs = parse_specs(packages)?;

    if pin {
        specs = pin_to_installed(manager.as_ref(), &specs)?;
    }

    let path = ctx.layout.path_for(layer, id);
    let added = edit_manifest(&path, &specs, Edit::Declare)?;

    if added.is_empty() {
        println!("nothing to add; already declared");
        return Ok(());
    }
    println!("declared in {}", path.display());
    for spec in &added {
        println!("  + {spec}");
    }
    println!("{}", reconcile::dim("run `mpm apply` to install"));
    Ok(())
}

/// Rewrite each package to the version it is installed at.
fn pin_to_installed(manager: &dyn Manager, specs: &[PackageSpec]) -> Result<Vec<PackageSpec>> {
    let id = manager.id();

    if !manager.supports_pinning() {
        bail!("`{id}` cannot install a specific version, so there is nothing to pin");
    }
    if !manager::present(manager) {
        bail!("`{id}` is not installed on this machine, so no version can be read");
    }
    if let Some(spec) = specs.iter().find(|spec| spec.version.is_some()) {
        bail!("`{spec}` already names a version -- drop `--pin`, or drop the version");
    }

    let installed = installed_map(manager)
        .with_context(|| format!("could not list installed packages for `{id}`"))?;

    specs
        .iter()
        .map(|spec| {
            let Some(found) = installed.get(&spec.name) else {
                bail!("`{}` is not installed, so there is no version to pin", spec.name);
            };
            let Some(version) = &found.version else {
                bail!("`{id}` does not report a version for `{}`", spec.name);
            };
            Ok(PackageSpec::pinned(&spec.name, version))
        })
        .collect()
}


/// Lock already-declared packages to the version installed right now.
///
/// The same operation as `add --pin`, reachable on its own so that pinning is
/// something you do *to* a package you already have, not only something you
/// decide at the moment you declare it.
pub fn pin(ctx: &Ctx, id: &str, packages: &[String], layer: &Layer) -> Result<()> {
    add(ctx, id, packages, layer, true)
}

/// Drop the version from packages, letting them track whatever is current.
pub fn unpin(ctx: &Ctx, id: &str, packages: &[String], layer: &Layer) -> Result<()> {
    require(id)?;
    let specs = parse_specs(packages)?;
    let path = ctx.layout.path_for(layer, id);
    let mut manifest = ManifestFile::load(&path)?;
    let mut changed = Vec::new();

    for spec in &specs {
        // Never introduces a package: unpinning an undeclared one would declare it.
        if !manifest.declares(&spec.name) {
            println!(
                "{} `{}` is not declared in {}",
                reconcile::bold("note:"),
                spec.name,
                path.display()
            );
            continue;
        }
        if manifest.declare(&PackageSpec::new(&spec.name)) {
            changed.push(spec.name.clone());
        }
    }

    if changed.is_empty() {
        println!("nothing to unpin");
        return Ok(());
    }

    manifest.save()?;
    println!("unpinned in {}", path.display());
    for name in &changed {
        println!("  {name}");
    }
    Ok(())
}

/// Undeclare packages and uninstall them now.
pub fn remove(ctx: &Ctx, id: &str, packages: &[String], layer: &Layer) -> Result<()> {
    let found = require_present(id)?;

    let specs = parse_specs(packages)?;
    let path = ctx.layout.path_for(layer, id);
    let names: Vec<String> =
        edit_manifest(&path, &specs, Edit::Withdraw)?.iter().map(|spec| spec.name.clone()).collect();

    // Uninstall everything named, whether or not this layer declared it.
    let requested: Vec<String> = specs.iter().map(|spec| spec.name.clone()).collect();
    for command in found.uninstall(&requested) {
        run_visibly(&command)?;
    }

    if names.is_empty() {
        println!("{}", reconcile::dim("was not declared in this layer; nothing changed in your manifest"));
    } else {
        println!("undeclared in {}", path.display());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edit {
    Declare,
    Withdraw,
}

/// Parse command-line package arguments into manifest entries.
fn parse_specs(packages: &[String]) -> Result<Vec<PackageSpec>> {
    if packages.is_empty() {
        bail!("name at least one package");
    }
    packages
        .iter()
        .map(|text| PackageSpec::parse(text))
        .collect::<Result<Vec<PackageSpec>>>()
        .context("a package is written `<name>` or, quoted, `<name> <version>`")
}

/// Apply an edit to one layer, returning the entries that actually changed.
fn edit_manifest(path: &Path, specs: &[PackageSpec], edit: Edit) -> Result<Vec<PackageSpec>> {
    let mut manifest = ManifestFile::load(path)?;
    let mut changed = Vec::new();

    for spec in specs {
        let touched = match edit {
            Edit::Declare => manifest.declare(spec),
            Edit::Withdraw => manifest.undeclare(&spec.name),
        };
        if touched {
            changed.push(spec.clone());
        }
    }

    if !changed.is_empty() {
        manifest.save()?;
    }
    Ok(changed)
}

// ------------------------------------------------------------------ inspection

pub fn managers(ctx: &Ctx) -> Result<()> {
    for id in manager::ALL {
        let present = if present_by_id(id) { "found" } else { "-" };
        let managed =
            if ctx.layout.is_managed(id, &ctx.host) { "managed" } else { "-" };
        println!("{id:<8} {present:<7} {managed}");
    }
    println!();
    println!("{}", reconcile::dim(&format!("host: {}", ctx.host)));
    println!("{}", reconcile::dim(&format!("manifests: {}", ctx.layout.root().display())));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// A manager whose "installed" state is whatever `printf` is told to emit.
    ///
    /// This exercises the real read path -- spawn a process, capture stdout,
    /// parse it -- without needing a package manager on the machine.
    struct Fake {
        installed: &'static str,
        pinning: bool,
    }

    impl Manager for Fake {
        fn id(&self) -> &'static str {
            "cargo" // borrow a real id so Layout paths line up
        }
        fn install(&self, packages: &[PackageSpec]) -> Result<Vec<Invocation>> {
            Ok(vec![Invocation::new("true").args(packages.iter().map(|spec| spec.name.clone()))])
        }
        fn uninstall(&self, names: &[String]) -> Vec<Invocation> {
            vec![Invocation::new("true").args(names.iter().cloned())]
        }
        fn list(&self) -> Invocation {
            Invocation::new("printf").arg("%s").arg(self.installed)
        }
        fn parse_list(&self, stdout: &str) -> Vec<PackageSpec> {
            crate::manager::parse_two_column(stdout)
        }
        fn supports_pinning(&self) -> bool {
            self.pinning
        }
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!("mpm-cmd-{tag}-{unique}"));
            fs::create_dir_all(&path).expect("temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn seed(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, body).expect("write");
    }

    fn run(layout: &Layout, fake: &Fake) -> Result<Reconciliation> {
        reconcile_manager(layout, "testbox", fake)
    }

    #[test]
    fn drift_is_computed_from_the_real_read_path() {
        let dir = TempDir::new("drift");
        let layout = Layout::at(&dir.0);
        seed(&layout.common("cargo"), "ripgrep\nvim\n");

        let fake = Fake { installed: "vim 9.1\nnano 8.0\n", pinning: false };
        let changes = run(&layout, &fake).expect("reconciles");

        assert_eq!(changes.install, vec![PackageSpec::new("ripgrep")]);
        assert_eq!(changes.remove, vec!["nano"]);
    }

    #[test]
    fn a_declared_package_is_never_a_removal_candidate() {
        let dir = TempDir::new("declared");
        let layout = Layout::at(&dir.0);
        seed(&layout.common("cargo"), "nano\n");

        let fake = Fake { installed: "nano 8.0\nbat 0.24\n", pinning: false };
        let changes = run(&layout, &fake).expect("reconciles");

        assert_eq!(changes.remove, vec!["bat"]);
    }

    #[test]
    fn the_host_layer_adds_to_the_shared_one() {
        let dir = TempDir::new("layers");
        let layout = Layout::at(&dir.0);
        seed(&layout.common("cargo"), "vim\n");
        seed(&layout.host("testbox", "cargo"), "tlp\n");

        let fake = Fake { installed: "vim 9.1\ntlp 1.6\n", pinning: false };
        let changes = run(&layout, &fake).expect("reconciles");

        assert!(!changes.has_work(), "both layers count as declared");
    }

    #[test]
    fn an_unhonourable_version_stops_the_manager_before_any_work() {
        let dir = TempDir::new("badpin");
        let layout = Layout::at(&dir.0);
        seed(&layout.common("cargo"), "ripgrep 14.1.0\n");

        let fake = Fake { installed: "", pinning: false };
        let error = run(&layout, &fake).expect_err("must refuse");
        assert!(error.to_string().contains("cannot install a specific version"));
    }

    #[test]
    fn a_version_mismatch_becomes_a_repin() {
        let dir = TempDir::new("repin");
        let layout = Layout::at(&dir.0);
        seed(&layout.common("cargo"), "ripgrep 14.1.0\n");

        let fake = Fake { installed: "ripgrep 14.0.0\n", pinning: true };
        let changes = run(&layout, &fake).expect("reconciles");

        assert_eq!(changes.repin.len(), 1);
        assert_eq!(changes.repin[0].installed, "14.0.0");
    }

    #[test]
    fn declaring_appends_and_is_idempotent() {
        let dir = TempDir::new("declare");
        let path = dir.0.join("cargo");
        seed(&path, "# tools\nvim\n");

        let specs = [PackageSpec::new("bat"), PackageSpec::new("vim")];
        let added = edit_manifest(&path, &specs, Edit::Declare).expect("edit");
        assert_eq!(added, vec![PackageSpec::new("bat")], "vim was already declared");
        assert_eq!(fs::read_to_string(&path).expect("read"), "# tools\nvim\nbat\n");

        let again = edit_manifest(&path, &[PackageSpec::new("bat")], Edit::Declare).expect("edit");
        assert!(again.is_empty());
    }

    #[test]
    fn withdrawing_removes_only_the_named_entry() {
        let dir = TempDir::new("withdrawing");
        let path = dir.0.join("cargo");
        seed(&path, "# tools\nvim\nbat\n");

        let removed =
            edit_manifest(&path, &[PackageSpec::new("bat")], Edit::Withdraw).expect("edit");
        assert_eq!(removed, vec![PackageSpec::new("bat")]);
        assert_eq!(fs::read_to_string(&path).expect("read"), "# tools\nvim\n");
    }

    #[test]
    fn a_malformed_package_argument_is_rejected() {
        let error = parse_specs(&["ripgrep 14.1.0 oops".into()]).expect_err("must reject");
        assert!(format!("{error:#}").contains("<name> <version>"));
    }

    #[test]
    fn nothing_needing_root_asks_for_nothing() {
        let queued = vec![("cargo".to_string(), vec![Invocation::new("true")])];
        preauthorize(&queued).expect("no elevation, no prompt");
    }

    #[test]
    fn transcripts_are_indented_under_their_manager() {
        assert_eq!(indent("one\ntwo\n"), "    one\n    two\n");
        assert_eq!(indent(""), "");
    }

    #[test]
    fn pinning_records_the_installed_version() {
        let fake = Fake { installed: "ripgrep 14.0.0-1\nbat 0.24.0-1\n", pinning: true };
        let pinned =
            pin_to_installed(&fake, &[PackageSpec::new("ripgrep")]).expect("pins");
        assert_eq!(pinned, vec![PackageSpec::pinned("ripgrep", "14.0.0-1")]);
    }

    #[test]
    fn pinning_what_is_not_installed_is_an_error() {
        let fake = Fake { installed: "ripgrep 14.0.0-1\n", pinning: true };
        let error = pin_to_installed(&fake, &[PackageSpec::new("nano")]).expect_err("must reject");
        assert!(error.to_string().contains("not installed"));
    }

    #[test]
    fn pinning_a_manager_that_cannot_pin_is_an_error() {
        let fake = Fake { installed: "ripgrep 14.0.0-1\n", pinning: false };
        let error =
            pin_to_installed(&fake, &[PackageSpec::new("ripgrep")]).expect_err("must reject");
        assert!(error.to_string().contains("nothing to pin"));
    }

    #[test]
    fn pinning_something_already_versioned_is_a_contradiction() {
        let fake = Fake { installed: "ripgrep 14.0.0-1\n", pinning: true };
        let error = pin_to_installed(&fake, &[PackageSpec::pinned("ripgrep", "13.0.0-1")])
            .expect_err("must reject");
        assert!(error.to_string().contains("already names a version"));
    }
}
