//! Egolint command-line interface.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use egolint::error::exit_code;
use egolint::{
    Config, ConfigOverrides, EgolintError, ExecutionPlan, NetworkMode, Operation, PlanOptions,
    Profile, PullPolicy, ResolvedConfig, RunReport, Runtime,
};

const INTERNAL_EXIT_CODE: u8 = 4;

#[derive(Debug, Parser)]
#[command(name = "egolint", version, about)]
#[command(propagate_version = true)]
struct Cli {
    /// Repository workspace to inspect.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit Egolint TOML configuration file.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Analyze a repository without granting broad write access.
    Check(RunArgs),
    /// Explicitly allow lint tools to update repository files.
    Fix(RunArgs),
    /// Print the redacted execution plan without starting a container.
    Plan(RunArgs),
    /// Validate configuration and container-runtime readiness.
    Doctor(RunArgs),
    /// Inspect effective configuration and provenance.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Emit machine-readable JSON Schemas.
    Schema {
        /// Schema contract to emit.
        #[arg(value_enum, default_value_t = SchemaKind::Config)]
        kind: SchemaKind,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Show effective values and every applied source.
    Explain {
        /// Output encoding.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Args, Default)]
struct RunArgs {
    /// Named lint profile.
    #[arg(long, value_enum)]
    profile: Option<Profile>,

    /// Container runtime.
    #[arg(long, value_enum)]
    runtime: Option<Runtime>,

    /// Exact image reference, ideally including a digest.
    #[arg(long)]
    image: Option<String>,

    /// Image pull behavior.
    #[arg(long, value_enum)]
    pull_policy: Option<PullPolicy>,

    /// Container network mode.
    #[arg(long, value_enum)]
    network: Option<NetworkMode>,

    /// Repository-local `MegaLinter` configuration file.
    #[arg(long)]
    megalinter_config: Option<PathBuf>,

    /// Validate only changed files.
    #[arg(long)]
    changed_only: bool,

    /// Enable one canonical `MegaLinter` identifier. Repeatable.
    #[arg(long = "enable-linter")]
    enable_linters: Vec<String>,

    /// Disable one canonical `MegaLinter` identifier. Repeatable.
    #[arg(long = "disable-linter")]
    disable_linters: Vec<String>,

    /// Output encoding for plan and doctor commands.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum SchemaKind {
    #[default]
    Config,
    Plan,
    Report,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => process_exit_code(code),
        Err(error) => {
            eprintln!("egolint: {error}");
            process_exit_code(error.exit_code())
        }
    }
}

fn process_exit_code(code: i32) -> ExitCode {
    debug_assert_eq!(i32::from(INTERNAL_EXIT_CODE), exit_code::INTERNAL);
    ExitCode::from(u8::try_from(code).unwrap_or(INTERNAL_EXIT_CODE))
}

fn run(cli: Cli) -> Result<i32, EgolintError> {
    let workspace = canonical_workspace(&cli.workspace)?;
    match cli.command {
        Command::Check(arguments) => execute(
            &workspace,
            cli.config.as_deref(),
            Operation::Check,
            &arguments,
        ),
        Command::Fix(arguments) => execute(
            &workspace,
            cli.config.as_deref(),
            Operation::Fix,
            &arguments,
        ),
        Command::Plan(arguments) => {
            let (_, plan) = build(
                &workspace,
                cli.config.as_deref(),
                Operation::Check,
                &arguments,
                false,
            )?;
            print_plan(&plan, arguments.format)?;
            Ok(0)
        }
        Command::Doctor(arguments) => {
            let (_, plan) = build(
                &workspace,
                cli.config.as_deref(),
                Operation::Check,
                &arguments,
                true,
            )?;
            plan.doctor()?;
            print_plan(&plan, arguments.format)?;
            Ok(0)
        }
        Command::Config {
            command: ConfigCommand::Explain { format },
        } => {
            let resolved = Config::resolve(
                &workspace,
                cli.config.as_deref(),
                &ConfigOverrides::default(),
            )?;
            print_config(&resolved, format)?;
            Ok(0)
        }
        Command::Schema { kind } => {
            print_schema(kind)?;
            Ok(0)
        }
    }
}

fn canonical_workspace(path: &Path) -> Result<PathBuf, EgolintError> {
    if !path.is_dir() {
        return Err(EgolintError::MissingPath(path.to_path_buf()));
    }
    path.canonicalize()
        .map_err(|source| EgolintError::Filesystem {
            path: path.to_path_buf(),
            source,
        })
}

fn build(
    workspace: &Path,
    config_path: Option<&Path>,
    operation: Operation,
    arguments: &RunArgs,
    require_runtime: bool,
) -> Result<(ResolvedConfig, ExecutionPlan), EgolintError> {
    let overrides = ConfigOverrides {
        profile: arguments.profile,
        runtime: arguments.runtime,
        image: arguments.image.clone(),
        pull_policy: arguments.pull_policy,
        network: arguments.network,
        megalinter_config: arguments.megalinter_config.clone(),
    };
    let resolved = Config::resolve(workspace, config_path, &overrides)?;
    let options = PlanOptions {
        changed_only: arguments.changed_only,
        enable_linters: arguments.enable_linters.clone(),
        disable_linters: arguments.disable_linters.clone(),
    };
    let plan = ExecutionPlan::build(workspace, &resolved, operation, &options, require_runtime)?;
    Ok((resolved, plan))
}

fn execute(
    workspace: &Path,
    config_path: Option<&Path>,
    operation: Operation,
    arguments: &RunArgs,
) -> Result<i32, EgolintError> {
    let (_, plan) = build(workspace, config_path, operation, arguments, true)?;
    let status = plan.execute()?;
    let report = RunReport::from_plan(&plan.view, status.code());
    plan.validate_report_path()?;
    report.write_atomic(&plan.report_path().join("run.json"))?;
    Ok(report.status.exit_code())
}

fn print_plan(plan: &ExecutionPlan, format: OutputFormat) -> Result<(), EgolintError> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&plan.view)?),
        OutputFormat::Text => {
            println!("Egolint execution plan");
            println!("  operation: {:?}", plan.view.operation);
            println!("  profile: {:?}", plan.view.profile);
            println!("  runtime: {}", plan.view.runtime);
            println!("  image: {}", plan.view.image);
            println!("  workspace: {}", plan.view.workspace.display());
            println!("  reports: {}", plan.view.report_directory.display());
            println!("  sources:");
            for source in &plan.view.config_sources {
                println!("    - {source}");
            }
            println!("  argv (environment values redacted; not shell syntax):");
            for (index, argument) in plan.view.argv.iter().enumerate() {
                println!("    [{index}] {}", serde_json::to_string(argument)?);
            }
        }
    }
    Ok(())
}

fn print_config(resolved: &ResolvedConfig, format: OutputFormat) -> Result<(), EgolintError> {
    let mut redacted = resolved.config.clone();
    for value in redacted.environment.values_mut() {
        "<redacted>".clone_into(value);
    }
    match format {
        OutputFormat::Json => {
            let value = serde_json::json!({
                "config": redacted,
                "sources": resolved.sources,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        OutputFormat::Text => {
            println!(
                "{}",
                toml::to_string_pretty(&redacted).map_err(|error| {
                    EgolintError::Configuration(format!("could not render configuration: {error}"))
                })?
            );
            println!("# sources (lowest to highest precedence)");
            for source in &resolved.sources {
                println!("# - {source}");
            }
        }
    }
    Ok(())
}

fn print_schema(kind: SchemaKind) -> Result<(), EgolintError> {
    let schema = match kind {
        SchemaKind::Config => schemars::schema_for!(Config),
        SchemaKind::Plan => schemars::schema_for!(egolint::plan::PlanView),
        SchemaKind::Report => schemars::schema_for!(RunReport),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_rejects_report_directory_override() {
        assert!(Cli::try_parse_from(["egolint", "plan", "--report-directory", "src",]).is_err());
    }
}
