//! CLI boundary for Egolint's JavaScript/TypeScript dependency architecture profile.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use egolint::error::exit_code;
use egolint::{ArchitectureRunOptions, run_javascript_architecture};

const INTERNAL_EXIT_CODE: u8 = 4;

#[derive(Debug, Parser)]
#[command(name = "egolint-architecture", version, about)]
struct Cli {
    /// Repository workspace to inspect.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,

    /// Canonical Egolint JavaScript architecture profile.
    #[arg(long, default_value = ".config/rules/javascript-architecture.v1.json")]
    profile: PathBuf,

    /// Repository-owned profile overlay. Repeatable.
    #[arg(long = "overlay")]
    overlays: Vec<PathBuf>,

    /// Auditable exception evaluation date in YYYY-MM-DD form.
    #[arg(long)]
    evaluation_date: String,

    /// Normalized JSON report path under `.reports/egolint`.
    #[arg(long, default_value = ".reports/egolint/javascript-architecture.json")]
    output: PathBuf,

    /// Normalized SARIF report path under `.reports/egolint`.
    #[arg(long, default_value = ".reports/egolint/javascript-architecture.sarif")]
    sarif: PathBuf,

    /// Optional DOT graph artifact path under `.reports/egolint`.
    #[arg(long)]
    graph: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let workspace = match cli.workspace.canonicalize() {
        Ok(workspace) if workspace.is_dir() => workspace,
        Ok(_) => {
            eprintln!("egolint-architecture: workspace is not a directory");
            return ExitCode::from(exit_code::CONFIGURATION as u8);
        }
        Err(error) => {
            eprintln!("egolint-architecture: could not resolve workspace: {error}");
            return ExitCode::from(exit_code::CONFIGURATION as u8);
        }
    };
    let options = ArchitectureRunOptions {
        workspace: &workspace,
        profile_path: &cli.profile,
        overlay_paths: &cli.overlays,
        evaluation_date: &cli.evaluation_date,
        json_output: &cli.output,
        sarif_output: &cli.sarif,
        graph_output: cli.graph.as_deref(),
    };
    match run_javascript_architecture(&options) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(INTERNAL_EXIT_CODE)),
        Err(error) => {
            eprintln!("egolint-architecture: {error}");
            ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(INTERNAL_EXIT_CODE))
        }
    }
}
