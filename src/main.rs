use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::process::ExitCode;

use crate::commands::{ApplyOpts, Ctx};
use crate::manifest::Layer;

mod commands;
mod exec;
mod manager;
mod manifest;
mod reconcile;

#[derive(Parser)]
#[command(
    name = "mpm",
    version,
    about = "Declare the packages your machine should have, and converge to it"
)]
struct Cli {
    /// Defaults to `status`.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show how this machine differs from its manifests (the default)
    Status(Scope),

    /// Install and remove packages so this machine matches its manifests
    Apply(ApplyArgs),

    /// Put installed packages under management, merging with what the manifest says
    Adopt(AdoptArgs),

    /// Declare packages, to be installed by `mpm apply`
    Add(AddArgs),

    /// Lock packages to the version installed right now
    Pin(PackageArgs),

    /// Let packages track whatever version is current
    Unpin(PackageArgs),

    /// Undeclare packages and uninstall them now
    Remove(PackageArgs),

    /// Show supported package managers and how this machine is configured
    Managers,
}

#[derive(Args)]
struct Scope {
    /// Act on one package manager instead of every managed one
    #[arg(short, long, value_name = "MANAGER")]
    manager: Option<String>,
}

#[derive(Args)]
struct ApplyArgs {
    #[command(flatten)]
    scope: Scope,

    /// Show the plan and exit without changing anything
    #[arg(long)]
    dry_run: bool,

    /// Do not ask for confirmation
    #[arg(short = 'y', long)]
    yes: bool,
}

#[derive(Args)]
struct AdoptArgs {
    #[command(flatten)]
    scope: Scope,

    #[command(flatten)]
    layer: LayerArgs,
}

#[derive(Args)]
struct AddArgs {
    #[command(flatten)]
    packages: PackageArgs,

    /// Record the version installed right now, instead of any version
    #[arg(long)]
    pin: bool,
}

#[derive(Args)]
struct PackageArgs {
    /// The package manager to act on
    #[arg(short, long, value_name = "MANAGER")]
    manager: String,

    #[command(flatten)]
    layer: LayerArgs,

    /// Packages. Quote to pin: `'ripgrep 14.1.0'`
    #[arg(required = true, value_name = "PACKAGE")]
    packages: Vec<String>,
}

/// Which layer an edit targets. Defaults to `common`.
#[derive(Args)]
struct LayerArgs {
    /// Target this machine's host layer instead of common
    #[arg(long)]
    host: bool,
}

impl LayerArgs {
    fn resolve(&self, host: &str) -> Layer {
        if self.host { Layer::Host(host.to_string()) } else { Layer::Common }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{} {error:#}", reconcile::bold("error:"));
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let ctx = Ctx::discover()?;

    let command = cli.command.unwrap_or(Command::Status(Scope { manager: None }));

    match command {
        Command::Status(scope) => {
            // Non-zero on drift, so mpm is usable in CI and shell prompts.
            let drifted = commands::status(&ctx, scope.manager.as_deref())?;
            return Ok(if drifted { ExitCode::FAILURE } else { ExitCode::SUCCESS });
        }

        Command::Apply(args) => commands::apply(
            &ctx,
            args.scope.manager.as_deref(),
            &ApplyOpts {
                dry_run: args.dry_run,
                yes: args.yes,
            },
        )?,

        Command::Adopt(args) => {
            let layer = args.layer.resolve(&ctx.host);
            commands::adopt(&ctx, args.scope.manager.as_deref(), &layer)?;
        }

        Command::Add(args) => {
            let layer = args.packages.layer.resolve(&ctx.host);
            commands::add(
                &ctx,
                &args.packages.manager,
                &args.packages.packages,
                &layer,
                args.pin,
            )?;
        }

        Command::Pin(args) => {
            let layer = args.layer.resolve(&ctx.host);
            commands::pin(&ctx, &args.manager, &args.packages, &layer)?;
        }

        Command::Unpin(args) => {
            let layer = args.layer.resolve(&ctx.host);
            commands::unpin(&ctx, &args.manager, &args.packages, &layer)?;
        }

        Command::Remove(args) => {
            let layer = args.layer.resolve(&ctx.host);
            commands::remove(&ctx, &args.manager, &args.packages, &layer)?;
        }

        Command::Managers => commands::managers(&ctx)?,
    }

    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    fn layer(host: bool) -> Layer {
        LayerArgs { host }.resolve("thinkpad")
    }

    #[test]
    fn the_host_flag_chooses_the_layer() {
        assert_eq!(layer(false), Layer::Common);
        assert_eq!(layer(true), Layer::Host("thinkpad".into()));
    }

    #[test]
    fn bare_invocation_has_no_subcommand() {
        let cli = Cli::try_parse_from(["mpm"]).expect("bare mpm parses");
        assert!(cli.command.is_none());
    }

    #[test]
    fn a_version_is_only_ever_recorded_deliberately() {
        let plain = Cli::try_parse_from(["mpm", "add", "-m", "cargo", "ripgrep"]).expect("parses");
        let Some(Command::Add(args)) = plain.command else { panic!("expected add") };
        assert!(!args.pin);

        let asked =
            Cli::try_parse_from(["mpm", "add", "-m", "cargo", "--pin", "ripgrep"]).expect("parses");
        let Some(Command::Add(args)) = asked.command else { panic!("expected add") };
        assert!(args.pin);
    }

    #[test]
    fn add_requires_at_least_one_package() {
        assert!(Cli::try_parse_from(["mpm", "add", "-m", "cargo"]).is_err());
        assert!(Cli::try_parse_from(["mpm", "add", "-m", "cargo", "ripgrep"]).is_ok());
    }

    #[test]
    fn a_pin_is_one_quoted_argument() {
        let cli = Cli::try_parse_from(["mpm", "add", "-m", "cargo", "ripgrep 14.1.0"]).expect("parses");
        let Some(Command::Add(args)) = cli.command else { panic!("expected add") };
        assert_eq!(args.packages.packages, vec!["ripgrep 14.1.0"]);
    }
}
