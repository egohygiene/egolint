//! Egolint command-line interface.

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use egolint::error::exit_code;
use egolint::rules::{
    IntelligenceEnforcement, PortabilityRuleSet, RepositoryContract, RepositoryContractEvaluator,
    RepositoryIntelligenceEvaluator, RepositoryIntelligencePolicy, RepositoryIntelligenceReport,
    RepositoryInventory, RepositoryPresentationEvaluator, RepositoryPresentationPolicy,
    RepositoryPresentationReport, RepresentedCommit, collect_commit_history,
    write_intelligence_report_atomic, write_presentation_report_atomic,
};
use egolint::rules::{
    PresentationMode, REPOSITORY_INTELLIGENCE_REPORT, REPOSITORY_PRESENTATION_REPORT,
    apply_suppressions,
};
use egolint::sarif::{EGOLINT_SARIF_REPORT, write_sarif_atomic};
use egolint::{
    CONTRACT_VERSION, Enforcement, EvidenceKind, EvidenceReference, Finding, ReportCompleteness,
    Severity, Suppression, ToolResult, ToolStatus, apply_reviewed_fix, normalize_workspace,
    run_isolated_fix,
};
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
    /// Run linters without granting broad repository write access.
    #[command(alias = "check")]
    Lint(RunArgs),
    /// Generate a bounded fix patch in an isolated repository copy.
    Fix(RunArgs),
    /// Apply a reviewed fix after digest, base, and post-tree verification.
    ApplyFix(ApplyFixArgs),
    /// Validate configuration and native rules without starting a container.
    Validate(RunArgs),
    /// Print the redacted execution plan without starting a container.
    Plan(RunArgs),
    /// Validate configuration and container-runtime readiness.
    Doctor(RunArgs),
    /// Inspect effective configuration and provenance.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Show effective configuration and ordered provenance.
    Explain {
        /// Output encoding.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
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

    /// Versioned repository contract to evaluate. Repeatable.
    #[arg(long = "repository-contract")]
    repository_contracts: Vec<PathBuf>,

    /// Versioned Repository Intelligence semantic policy.
    #[arg(long, requires = "represented_commit")]
    repository_intelligence: Option<PathBuf>,

    /// Versioned repository-presentation validation policy.
    #[arg(long, requires = "represented_commit")]
    repository_presentation: Option<PathBuf>,

    /// Full represented Git commit, `unknown`, or `not-applicable`.
    #[arg(long)]
    represented_commit: Option<String>,

    /// One versioned suppression JSON document. Repeatable.
    #[arg(long = "suppression", requires = "evaluation_date")]
    suppressions: Vec<PathBuf>,

    /// Auditable suppression evaluation date in YYYY-MM-DD form.
    #[arg(long, requires = "suppressions")]
    evaluation_date: Option<String>,

    /// Output encoding for plan and doctor commands.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
struct ApplyFixArgs {
    /// SHA-256 printed by the reviewed `egolint fix` preview.
    #[arg(long)]
    patch_sha256: String,

    /// Git commit printed by the reviewed `egolint fix` preview.
    #[arg(long)]
    base_commit: String,

    /// Git tree printed by the reviewed `egolint fix` preview.
    #[arg(long)]
    post_tree: String,
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
    Profile,
    Finding,
    Suppression,
    Evidence,
    ToolResult,
    Plan,
    Report,
    Debt,
    RepositoryContract,
    RepositoryIntelligence,
    RepositoryIntelligenceReport,
    RepositoryPresentation,
    RepositoryPresentationReport,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => process_exit_code(code),
        Err(error) => {
            eprintln!("egolint: {}", bounded_console_text(&error.to_string()));
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
        Command::Lint(arguments) => execute_lint(&workspace, cli.config.as_deref(), &arguments),
        Command::Fix(arguments) => {
            execute_fix_preview(&workspace, cli.config.as_deref(), &arguments)
        }
        Command::ApplyFix(arguments) => {
            apply_reviewed_fix(
                &workspace,
                &arguments.patch_sha256,
                &arguments.base_commit,
                &arguments.post_tree,
            )?;
            println!(
                "Applied and staged reviewed patch; reverse with: git apply --reverse --index {}",
                egolint::FIX_PATCH_PATH
            );
            Ok(0)
        }
        Command::Validate(arguments) => {
            execute_validate(&workspace, cli.config.as_deref(), &arguments)
        }
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
        }
        | Command::Explain { format } => explain(&workspace, cli.config.as_deref(), format),
        Command::Schema { kind } => {
            print_schema(kind)?;
            Ok(0)
        }
    }
}

fn explain(
    workspace: &Path,
    config_path: Option<&Path>,
    format: OutputFormat,
) -> Result<i32, EgolintError> {
    let resolved = Config::resolve(workspace, config_path, &ConfigOverrides::default())?;
    print_config(&resolved, format)?;
    Ok(0)
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
    let options = plan_options(arguments);
    let plan = ExecutionPlan::build(workspace, &resolved, operation, &options, require_runtime)?;
    Ok((resolved, plan))
}

fn plan_options(arguments: &RunArgs) -> PlanOptions {
    PlanOptions {
        changed_only: arguments.changed_only,
        enable_linters: arguments.enable_linters.clone(),
        disable_linters: arguments.disable_linters.clone(),
    }
}

fn execute_lint(
    workspace: &Path,
    config_path: Option<&Path>,
    arguments: &RunArgs,
) -> Result<i32, EgolintError> {
    let (_, plan) = build(workspace, config_path, Operation::Check, arguments, true)?;
    // Load every repository-owned policy input before invoking the untrusted
    // adapter. The check container receives only a read-only repository mount,
    // but this ordering also prevents a compromised runtime or report mount
    // from selecting a different contract for post-processing.
    let native = evaluate_native(workspace, arguments)?;
    let mut suppressions = load_suppressions(workspace, arguments)?;
    let status = plan.execute()?;
    plan.validate_report_path()?;

    let adapter = normalize_workspace(workspace, plan.view.profile)?;
    let mut tool_results = adapter
        .as_ref()
        .map_or_else(Vec::new, |normalized| normalized.tool_results.clone());
    let mut findings = native.findings;
    let mut evidence = native.evidence;
    let intelligence_report = native.intelligence_report;
    let intelligence_enforcement = native.intelligence_enforcement;
    let presentation_report = native.presentation_report;
    let presentation_mode = native.presentation_mode;
    let mut completeness = if let Some(normalized) = adapter {
        findings.extend(normalized.findings);
        evidence.extend(normalized.evidence);
        normalized.completeness
    } else {
        ReportCompleteness::Partial
    };
    if !matches!(status.code(), Some(exit_code::CLEAN | exit_code::FINDINGS)) {
        completeness = ReportCompleteness::Partial;
    }
    add_runtime_failure(status.code(), &mut tool_results);
    evaluate_suppressions(
        &mut findings,
        &mut suppressions,
        arguments.evaluation_date.as_deref(),
    )?;
    add_native_tool_results(
        &mut tool_results,
        &findings,
        !arguments.repository_contracts.is_empty(),
        intelligence_enforcement,
        presentation_mode,
        !suppressions.is_empty(),
    );
    reconcile_suppressed_tool_results(&mut tool_results, &findings);

    let mut report = RunReport::from_plan(&plan.view, status.code());
    report.set_normalized(tool_results, findings, suppressions, evidence, completeness)?;
    write_run_outputs(
        &plan,
        &report,
        plan.view.profile == Profile::DependencyDebt,
        intelligence_report.as_ref(),
        presentation_report.as_ref(),
    )?;
    print_findings(&report);
    Ok(report.status.exit_code())
}

fn execute_validate(
    workspace: &Path,
    config_path: Option<&Path>,
    arguments: &RunArgs,
) -> Result<i32, EgolintError> {
    let (_, plan) = build(workspace, config_path, Operation::Check, arguments, false)?;
    plan.prepare_report_directory()?;
    let native = evaluate_native(workspace, arguments)?;
    let mut findings = native.findings;
    let intelligence_report = native.intelligence_report;
    let intelligence_enforcement = native.intelligence_enforcement;
    let presentation_report = native.presentation_report;
    let presentation_mode = native.presentation_mode;
    let mut suppressions = load_suppressions(workspace, arguments)?;
    evaluate_suppressions(
        &mut findings,
        &mut suppressions,
        arguments.evaluation_date.as_deref(),
    )?;
    let mut tool_results = Vec::new();
    add_native_tool_results(
        &mut tool_results,
        &findings,
        !arguments.repository_contracts.is_empty(),
        intelligence_enforcement,
        presentation_mode,
        !suppressions.is_empty(),
    );
    let mut report = RunReport::from_plan(&plan.view, Some(exit_code::CLEAN));
    report.set_normalized(
        tool_results,
        findings,
        suppressions,
        native.evidence,
        ReportCompleteness::Partial,
    )?;
    write_run_outputs(
        &plan,
        &report,
        false,
        intelligence_report.as_ref(),
        presentation_report.as_ref(),
    )?;
    print_findings(&report);
    Ok(report.status.exit_code())
}

fn execute_fix_preview(
    workspace: &Path,
    config_path: Option<&Path>,
    arguments: &RunArgs,
) -> Result<i32, EgolintError> {
    let (resolved, plan) = build(workspace, config_path, Operation::Fix, arguments, false)?;
    let outcome = run_isolated_fix(workspace, &resolved, &plan_options(arguments))?;
    let native = evaluate_native(workspace, arguments)?;
    let mut findings = native.findings;
    let intelligence_report = native.intelligence_report;
    let intelligence_enforcement = native.intelligence_enforcement;
    let presentation_report = native.presentation_report;
    let presentation_mode = native.presentation_mode;
    let mut suppressions = load_suppressions(workspace, arguments)?;
    evaluate_suppressions(
        &mut findings,
        &mut suppressions,
        arguments.evaluation_date.as_deref(),
    )?;
    let mut tool_results = Vec::new();
    add_native_tool_results(
        &mut tool_results,
        &findings,
        !arguments.repository_contracts.is_empty(),
        intelligence_enforcement,
        presentation_mode,
        !suppressions.is_empty(),
    );
    let mut report = RunReport::from_plan(&plan.view, outcome.adapter_exit_code);
    let mut evidence = native.evidence;
    evidence.push(EvidenceReference {
        schema_version: CONTRACT_VERSION,
        kind: EvidenceKind::Other,
        path: outcome.patch_path.clone(),
        sha256: Some(outcome.patch_sha256.clone()),
        description: Some(
            "Bounded fix preview generated outside the original worktree.".to_owned(),
        ),
    });
    report.set_normalized(
        tool_results,
        findings,
        suppressions,
        evidence,
        ReportCompleteness::Partial,
    )?;
    write_run_outputs(
        &plan,
        &report,
        false,
        intelligence_report.as_ref(),
        presentation_report.as_ref(),
    )?;
    print_findings(&report);
    println!("Fix preview: {}", outcome.patch_path.display());
    println!("Patch SHA-256: {}", outcome.patch_sha256);
    println!("Base commit: {}", outcome.base_commit);
    println!("Expected post-tree: {}", outcome.post_tree);
    if outcome.changed {
        println!(
            "After review, apply exactly this preview with: egolint --workspace {} apply-fix --patch-sha256 {} --base-commit {} --post-tree {}",
            workspace.display(),
            outcome.patch_sha256,
            outcome.base_commit,
            outcome.post_tree
        );
    } else {
        println!("No source changes were proposed.");
    }
    Ok(report.status.exit_code())
}

struct NativeEvaluation {
    findings: Vec<Finding>,
    evidence: Vec<EvidenceReference>,
    intelligence_report: Option<RepositoryIntelligenceReport>,
    intelligence_enforcement: Option<IntelligenceEnforcement>,
    presentation_report: Option<RepositoryPresentationReport>,
    presentation_mode: Option<PresentationMode>,
}

#[allow(clippy::too_many_lines)]
fn evaluate_native(
    workspace: &Path,
    arguments: &RunArgs,
) -> Result<NativeEvaluation, EgolintError> {
    let inventory = RepositoryInventory::discover(workspace)?;
    let portability = PortabilityRuleSet::bundled()?;
    let mut findings = portability.evaluate(&inventory)?;
    let mut evidence = vec![EvidenceReference {
        schema_version: CONTRACT_VERSION,
        kind: EvidenceKind::Policy,
        path: PathBuf::from(".config/rules/portability.toml"),
        sha256: None,
        description: Some("Egolint's embedded versioned portability rule catalog.".to_owned()),
    }];
    for contract_path in &arguments.repository_contracts {
        let (relative, contents) = read_workspace_file(workspace, contract_path, 4 * 1024 * 1024)?;
        let contents = std::str::from_utf8(&contents).map_err(|_| {
            EgolintError::Configuration(format!(
                "repository contract must contain UTF-8: {}",
                relative.display()
            ))
        })?;
        let contract = RepositoryContract::from_toml(contents, &relative)?;
        let evaluator = RepositoryContractEvaluator::new(&contract, &relative)?;
        findings.extend(evaluator.evaluate(&inventory)?);
        evidence.push(EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::Configuration,
            path: relative,
            sha256: None,
            description: Some("Pinned local repository-contract projection.".to_owned()),
        });
    }
    let mut intelligence_report = None;
    let mut intelligence_enforcement = None;
    if let Some(policy_path) = &arguments.repository_intelligence {
        let (relative, contents) = read_workspace_file(workspace, policy_path, 4 * 1024 * 1024)?;
        let contents = std::str::from_utf8(&contents).map_err(|_| {
            EgolintError::Configuration(format!(
                "repository-intelligence policy must contain UTF-8: {}",
                relative.display()
            ))
        })?;
        let policy = RepositoryIntelligencePolicy::from_toml(contents, &relative)?;
        let represented = RepresentedCommit::parse(
            arguments
                .represented_commit
                .as_deref()
                .expect("clap requires represented commit with intelligence policy"),
        )?;
        let history = collect_commit_history(
            workspace,
            &represented,
            policy.commit_history.maximum_commits,
        )?;
        let evaluator = RepositoryIntelligenceEvaluator::new(&policy, &relative, represented)?;
        let evaluation = evaluator.evaluate(&inventory, &history)?;
        findings.extend(evaluation.findings);
        intelligence_enforcement = Some(policy.profile.enforcement);
        intelligence_report = Some(evaluation.report);
        evidence.push(EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::Configuration,
            path: relative,
            sha256: None,
            description: Some(
                "Versioned Repository Intelligence semantic policy and adoption profile."
                    .to_owned(),
            ),
        });
    }
    let mut presentation_report = None;
    let mut presentation_mode = None;
    if let Some(policy_path) = &arguments.repository_presentation {
        let (relative, contents) = read_workspace_file(workspace, policy_path, 4 * 1024 * 1024)?;
        let contents = std::str::from_utf8(&contents).map_err(|_| {
            EgolintError::Configuration(format!(
                "repository-presentation policy must contain UTF-8: {}",
                relative.display()
            ))
        })?;
        let policy = RepositoryPresentationPolicy::from_toml(contents, &relative)?;
        let represented = RepresentedCommit::parse(
            arguments
                .represented_commit
                .as_deref()
                .expect("clap requires represented commit with presentation policy"),
        )?;
        let evaluator = RepositoryPresentationEvaluator::new(&policy, &relative, represented)?;
        let evaluation = evaluator.evaluate(&inventory)?;
        findings.extend(evaluation.findings);
        presentation_mode = Some(policy.mode);
        presentation_report = Some(evaluation.report);
        evidence.push(EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::Configuration,
            path: relative,
            sha256: None,
            description: Some(
                "Versioned repository-presentation policy with pinned Hygiene and Identity inputs."
                    .to_owned(),
            ),
        });
    }
    if arguments.represented_commit.is_some()
        && arguments.repository_intelligence.is_none()
        && arguments.repository_presentation.is_none()
    {
        return Err(EgolintError::Configuration(
            "represented-commit requires repository-intelligence or repository-presentation"
                .to_owned(),
        ));
    }
    findings.sort_by(finding_order);
    Ok(NativeEvaluation {
        findings,
        evidence,
        intelligence_report,
        intelligence_enforcement,
        presentation_report,
        presentation_mode,
    })
}

fn load_suppressions(
    workspace: &Path,
    arguments: &RunArgs,
) -> Result<Vec<Suppression>, EgolintError> {
    let mut suppressions = Vec::new();
    for suppression_path in &arguments.suppressions {
        let (relative, contents) = read_workspace_file(workspace, suppression_path, 1024 * 1024)?;
        let suppression = serde_json::from_slice::<Suppression>(&contents).map_err(|error| {
            EgolintError::Configuration(format!(
                "invalid suppression JSON at {}: {error}",
                relative.display()
            ))
        })?;
        suppressions.push(suppression);
    }
    Ok(suppressions)
}

fn evaluate_suppressions(
    findings: &mut Vec<Finding>,
    suppressions: &mut [Suppression],
    evaluation_date: Option<&str>,
) -> Result<(), EgolintError> {
    if suppressions.is_empty() {
        return Ok(());
    }
    let evaluation_date = evaluation_date.ok_or_else(|| {
        EgolintError::Configuration(
            "--evaluation-date is required when --suppression is used".to_owned(),
        )
    })?;
    let portability = PortabilityRuleSet::bundled()?;
    apply_suppressions(findings, suppressions, evaluation_date, |rule| {
        if rule.tool_id == "EGOLINT_PORTABILITY" {
            portability.is_suppressible(&rule.rule_id)
        } else {
            !matches!(
                rule.tool_id.as_str(),
                "EGOLINT_SUPPRESSIONS" | "EGOLINT_REPOSITORY_INTELLIGENCE"
            )
        }
    })
}

fn read_workspace_file(
    workspace: &Path,
    requested: &Path,
    maximum_bytes: u64,
) -> Result<(PathBuf, Vec<u8>), EgolintError> {
    let Some(text) = requested.to_str() else {
        return Err(EgolintError::Configuration(
            "repository input paths must contain valid UTF-8".to_owned(),
        ));
    };
    if requested.as_os_str().is_empty()
        || requested.is_absolute()
        || text.contains('\\')
        || requested
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(EgolintError::Configuration(format!(
            "repository input must be a normalized workspace-relative path: {}",
            requested.display()
        )));
    }
    if requested
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == ".reports")
    {
        return Err(EgolintError::Configuration(format!(
            "repository policy input may not use Egolint's writable report boundary: {}",
            requested.display()
        )));
    }
    let mut current = workspace.to_path_buf();
    for component in requested.components() {
        let Component::Normal(part) = component else {
            unreachable!("components were validated above");
        };
        current.push(part);
        let metadata =
            std::fs::symlink_metadata(&current).map_err(|source| EgolintError::Filesystem {
                path: current.clone(),
                source,
            })?;
        if metadata.file_type().is_symlink() {
            return Err(EgolintError::Configuration(format!(
                "repository input path may not contain links: {}",
                requested.display()
            )));
        }
    }
    let metadata =
        std::fs::symlink_metadata(&current).map_err(|source| EgolintError::Filesystem {
            path: current.clone(),
            source,
        })?;
    if !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(EgolintError::Configuration(format!(
            "repository input must be a regular file no larger than {maximum_bytes} bytes: {}",
            requested.display()
        )));
    }
    let contents = std::fs::read(&current).map_err(|source| EgolintError::Filesystem {
        path: current,
        source,
    })?;
    if contents.len() as u64 > maximum_bytes {
        return Err(EgolintError::Configuration(format!(
            "repository input grew beyond the {maximum_bytes}-byte limit: {}",
            requested.display()
        )));
    }
    Ok((requested.to_path_buf(), contents))
}

fn add_runtime_failure(adapter_exit_code: Option<i32>, tool_results: &mut Vec<ToolResult>) {
    if matches!(
        adapter_exit_code,
        Some(exit_code::CLEAN | exit_code::FINDINGS)
    ) {
        return;
    }
    tool_results.push(ToolResult {
        schema_version: CONTRACT_VERSION,
        tool_id: "MEGALINTER_ADAPTER".to_owned(),
        owner: "egohygiene/egolint".to_owned(),
        policy_source: ".config/megalinter/policy.yml".to_owned(),
        status: ToolStatus::ExecutionError,
        enforcement: Enforcement::Blocking,
        finding_count: 0,
        warning_count: 0,
        duration_ms: None,
        evidence: Vec::new(),
    });
}

fn add_native_tool_results(
    tool_results: &mut Vec<ToolResult>,
    findings: &[Finding],
    contracts_evaluated: bool,
    intelligence_enforcement: Option<IntelligenceEnforcement>,
    presentation_mode: Option<PresentationMode>,
    suppressions_evaluated: bool,
) {
    tool_results.push(native_tool_result(
        "EGOLINT_PORTABILITY",
        findings,
        Enforcement::Blocking,
    ));
    if contracts_evaluated {
        tool_results.push(native_tool_result(
            "EGOLINT_REPOSITORY_CONTRACT",
            findings,
            Enforcement::Blocking,
        ));
    }
    if let Some(enforcement) = intelligence_enforcement {
        tool_results.push(native_tool_result(
            "EGOLINT_REPOSITORY_INTELLIGENCE",
            findings,
            match enforcement {
                IntelligenceEnforcement::Blocking => Enforcement::Blocking,
                IntelligenceEnforcement::Advisory => Enforcement::Advisory,
            },
        ));
    }
    if let Some(mode) = presentation_mode {
        tool_results.push(native_tool_result(
            "EGOLINT_REPOSITORY_PRESENTATION",
            findings,
            match mode {
                PresentationMode::Blocking => Enforcement::Blocking,
                PresentationMode::Advisory => Enforcement::Advisory,
            },
        ));
    }
    if suppressions_evaluated {
        tool_results.push(native_tool_result(
            "EGOLINT_SUPPRESSIONS",
            findings,
            Enforcement::Blocking,
        ));
    }
    tool_results.sort_by(|left, right| left.tool_id.cmp(&right.tool_id));
}

fn native_tool_result(tool_id: &str, findings: &[Finding], enforcement: Enforcement) -> ToolResult {
    let relevant = findings
        .iter()
        .filter(|finding| finding.rule.tool_id == tool_id)
        .collect::<Vec<_>>();
    let finding_count = relevant
        .iter()
        .filter(|finding| matches!(finding.severity, Severity::Error | Severity::Critical))
        .count() as u64;
    let warning_count = relevant.len() as u64 - finding_count;
    let has_blocking = relevant.iter().any(|finding| {
        finding.suppressed_by.is_none()
            && matches!(finding.severity, Severity::Error | Severity::Critical)
    });
    let status = if has_blocking {
        ToolStatus::FailedFindings
    } else if relevant.is_empty() {
        ToolStatus::Passed
    } else {
        ToolStatus::PassedWithWarnings
    };
    ToolResult {
        schema_version: CONTRACT_VERSION,
        tool_id: tool_id.to_owned(),
        owner: "egohygiene/egolint".to_owned(),
        policy_source: match tool_id {
            "EGOLINT_PORTABILITY" => ".config/rules/portability.toml",
            "EGOLINT_REPOSITORY_CONTRACT" => "docs/repository-contracts.md",
            "EGOLINT_REPOSITORY_INTELLIGENCE" => ".config/rules/repository-intelligence.v1.toml",
            "EGOLINT_REPOSITORY_PRESENTATION" => ".config/rules/repository-presentation.v1.toml",
            "EGOLINT_SUPPRESSIONS" => "docs/suppressions.md",
            _ => "README.md",
        }
        .to_owned(),
        status,
        enforcement,
        finding_count,
        warning_count,
        duration_ms: None,
        evidence: Vec::new(),
    }
}

fn reconcile_suppressed_tool_results(tool_results: &mut [ToolResult], findings: &[Finding]) {
    for result in tool_results {
        if result.status != ToolStatus::FailedFindings {
            continue;
        }
        let remains_blocking = findings.iter().any(|finding| {
            finding.rule.tool_id == result.tool_id
                && finding.suppressed_by.is_none()
                && matches!(finding.severity, Severity::Error | Severity::Critical)
        });
        if !remains_blocking {
            result.status = ToolStatus::PassedWithWarnings;
        }
    }
}

fn write_run_outputs(
    plan: &ExecutionPlan,
    report: &RunReport,
    include_debt: bool,
    intelligence_report: Option<&RepositoryIntelligenceReport>,
    presentation_report: Option<&RepositoryPresentationReport>,
) -> Result<(), EgolintError> {
    plan.validate_report_path()?;
    report.write_atomic(&plan.report_path().join("run.json"))?;
    plan.validate_report_path()?;
    write_sarif_atomic(report, &plan.view.workspace.join(EGOLINT_SARIF_REPORT))?;
    if let Some(intelligence_report) = intelligence_report {
        plan.validate_report_path()?;
        write_intelligence_report_atomic(
            intelligence_report,
            &plan.view.workspace.join(REPOSITORY_INTELLIGENCE_REPORT),
        )?;
    }
    if let Some(presentation_report) = presentation_report {
        plan.validate_report_path()?;
        write_presentation_report_atomic(
            presentation_report,
            &plan.view.workspace.join(REPOSITORY_PRESENTATION_REPORT),
        )?;
    }
    if include_debt {
        plan.validate_report_path()?;
        let debt = egolint::debt::DebtReport::from_run(report)?;
        egolint::debt::write_debt_reports(&debt, plan.report_path())?;
    }
    plan.validate_report_path()
}

fn print_findings(report: &RunReport) {
    for finding in &report.findings {
        let severity = match finding.severity {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error | Severity::Critical => "error",
        };
        let message = bounded_console_text(&finding.message);
        if let Some(location) = &finding.location {
            if let Some(path) = matcher_safe_path(&location.path) {
                println!(
                    "egolint:{path}:{}:{}: {severity}: {message} [{}/{}]",
                    location.start_line.unwrap_or(1),
                    location.start_column.unwrap_or(1),
                    finding.rule.tool_id,
                    finding.rule.rule_id
                );
                continue;
            }
        }
        println!(
            "egolint: {severity}: {message} [{}/{}]",
            finding.rule.tool_id, finding.rule.rule_id
        );
    }
}

fn matcher_safe_path(path: &Path) -> Option<&str> {
    let text = path.to_str()?;
    if text.is_empty()
        || text.contains("::")
        || !text.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '/' | '.' | '_' | '-' | '@' | '+' | ' ')
        })
    {
        None
    } else {
        Some(text)
    }
}

fn bounded_console_text(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    sanitized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(1_024)
        .collect()
}

fn finding_order(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let left_path = left
        .location
        .as_ref()
        .map_or_else(|| Path::new(""), |location| location.path.as_path());
    let right_path = right
        .location
        .as_ref()
        .map_or_else(|| Path::new(""), |location| location.path.as_path());
    left_path
        .cmp(right_path)
        .then_with(|| left.rule.tool_id.cmp(&right.rule.tool_id))
        .then_with(|| left.rule.rule_id.cmp(&right.rule.rule_id))
        .then_with(|| left.id.cmp(&right.id))
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
        SchemaKind::Profile => schemars::schema_for!(egolint::ProfileDefinition),
        SchemaKind::Finding => schemars::schema_for!(egolint::Finding),
        SchemaKind::Suppression => schemars::schema_for!(egolint::Suppression),
        SchemaKind::Evidence => schemars::schema_for!(egolint::EvidenceReference),
        SchemaKind::ToolResult => schemars::schema_for!(egolint::ToolResult),
        SchemaKind::Plan => schemars::schema_for!(egolint::plan::PlanView),
        SchemaKind::Report => schemars::schema_for!(RunReport),
        SchemaKind::Debt => schemars::schema_for!(egolint::debt::DebtReport),
        SchemaKind::RepositoryContract => schemars::schema_for!(RepositoryContract),
        SchemaKind::RepositoryIntelligence => {
            schemars::schema_for!(RepositoryIntelligencePolicy)
        }
        SchemaKind::RepositoryIntelligenceReport => {
            schemars::schema_for!(RepositoryIntelligenceReport)
        }
        SchemaKind::RepositoryPresentation => {
            schemars::schema_for!(RepositoryPresentationPolicy)
        }
        SchemaKind::RepositoryPresentationReport => {
            schemars::schema_for!(RepositoryPresentationReport)
        }
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

    #[test]
    fn required_issue_two_commands_are_first_class() {
        assert!(matches!(
            Cli::try_parse_from(["egolint", "lint"])
                .expect("lint command")
                .command,
            Command::Lint(_)
        ));
        assert!(matches!(
            Cli::try_parse_from(["egolint", "validate"])
                .expect("validate command")
                .command,
            Command::Validate(_)
        ));
        assert!(matches!(
            Cli::try_parse_from(["egolint", "explain"])
                .expect("explain command")
                .command,
            Command::Explain { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["egolint", "doctor"])
                .expect("doctor command")
                .command,
            Command::Doctor(_)
        ));
    }

    #[test]
    fn legacy_check_and_nested_explain_remain_compatible() {
        assert!(matches!(
            Cli::try_parse_from(["egolint", "check"])
                .expect("check alias")
                .command,
            Command::Lint(_)
        ));
        assert!(matches!(
            Cli::try_parse_from(["egolint", "config", "explain"])
                .expect("nested explain compatibility")
                .command,
            Command::Config {
                command: ConfigCommand::Explain { .. }
            }
        ));
    }

    #[test]
    fn fix_is_preview_only_and_apply_requires_review_evidence() {
        let patch_sha256 = "a".repeat(64);
        let base_commit = "b".repeat(40);
        let post_tree = "c".repeat(40);
        assert!(Cli::try_parse_from(["egolint", "fix", "--apply"]).is_err());
        assert!(
            Cli::try_parse_from([
                "egolint",
                "apply-fix",
                "--patch-sha256",
                &patch_sha256,
                "--base-commit",
                &base_commit,
                "--post-tree",
                &post_tree,
            ])
            .is_ok()
        );
    }

    #[test]
    fn suppression_requires_an_explicit_evaluation_date() {
        assert!(
            Cli::try_parse_from(["egolint", "validate", "--suppression", "waiver.json"]).is_err()
        );
        assert!(
            Cli::try_parse_from([
                "egolint",
                "validate",
                "--suppression",
                "waiver.json",
                "--evaluation-date",
                "2026-08-19",
            ])
            .is_ok()
        );
    }

    #[test]
    fn repository_contract_schema_is_a_first_class_cli_surface() {
        assert!(matches!(
            Cli::try_parse_from(["egolint", "schema", "repository-contract"])
                .expect("repository-contract schema command")
                .command,
            Command::Schema {
                kind: SchemaKind::RepositoryContract
            }
        ));
    }

    #[test]
    fn repository_intelligence_requires_explicit_represented_commit() {
        assert!(
            Cli::try_parse_from([
                "egolint",
                "validate",
                "--repository-intelligence",
                "intelligence.toml",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "egolint",
                "validate",
                "--repository-intelligence",
                "intelligence.toml",
                "--represented-commit",
                "unknown",
            ])
            .is_ok()
        );
        assert!(matches!(
            Cli::try_parse_from(["egolint", "schema", "repository-intelligence"])
                .expect("repository-intelligence schema command")
                .command,
            Command::Schema {
                kind: SchemaKind::RepositoryIntelligence
            }
        ));
        assert!(matches!(
            Cli::try_parse_from(["egolint", "schema", "repository-intelligence-report"])
                .expect("repository-intelligence report schema command")
                .command,
            Command::Schema {
                kind: SchemaKind::RepositoryIntelligenceReport
            }
        ));
    }

    #[test]
    fn repository_presentation_requires_commit_and_exposes_schemas() {
        assert!(
            Cli::try_parse_from([
                "egolint",
                "validate",
                "--repository-presentation",
                "presentation.toml",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "egolint",
                "validate",
                "--repository-presentation",
                "presentation.toml",
                "--represented-commit",
                "unknown",
            ])
            .is_ok()
        );
        assert!(matches!(
            Cli::try_parse_from(["egolint", "schema", "repository-presentation"])
                .expect("repository-presentation schema command")
                .command,
            Command::Schema {
                kind: SchemaKind::RepositoryPresentation
            }
        ));
        assert!(matches!(
            Cli::try_parse_from(["egolint", "schema", "repository-presentation-report"])
                .expect("repository-presentation report schema command")
                .command,
            Command::Schema {
                kind: SchemaKind::RepositoryPresentationReport
            }
        ));
    }

    #[test]
    fn matcher_paths_reject_log_and_workflow_command_injection() {
        assert_eq!(
            matcher_safe_path(Path::new("src/portable-file.rs")),
            Some("src/portable-file.rs")
        );
        for unsafe_path in ["src/line\nbreak.rs", "src/line\rbreak.rs", "::warning::x"] {
            assert!(matcher_safe_path(Path::new(unsafe_path)).is_none());
        }
    }

    #[test]
    fn writable_report_boundary_cannot_supply_policy_inputs() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let report = directory.path().join(".reports/egolint");
        std::fs::create_dir_all(&report).expect("report directory");
        std::fs::write(report.join("suppression.json"), b"{}").expect("synthetic suppression");

        assert!(
            read_workspace_file(
                directory.path(),
                Path::new(".reports/egolint/suppression.json"),
                1_024,
            )
            .is_err()
        );
    }

    #[test]
    fn top_level_error_text_collapses_control_lines() {
        assert_eq!(
            bounded_console_text("first\n::stop-commands::token\rsecond\u{1b}[31m"),
            "first ::stop-commands::token second [31m"
        );
    }
}
