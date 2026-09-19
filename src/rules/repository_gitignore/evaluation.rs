//! Separate source, content, effective policy, and index evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::contracts::CONTRACT_VERSION;
use crate::error::{EgolintError, Result};

use super::contract::{self, Composition};
use super::git::{Git, Match};
use super::inventory::{self, Snapshot};
use super::{
    GitignoreBehaviorEvidence, GitignoreCheckStatus as Check, GitignoreChecks, GitignoreEvaluation,
    GitignoreException, GitignoreFileEvidence, GitignoreValidationStatus as Status, MAX_PROBES,
    RepositoryGitignorePolicy, RepositoryGitignoreReport, digest, findings, issue, scope_of,
    scope_path,
};

// Keep evidence acquisition and its end-of-run consistency checks in order.
#[allow(clippy::too_many_lines)]
pub(super) fn evaluate(
    workspace: &Path,
    policy: &RepositoryGitignorePolicy,
    policy_path: &Path,
    evaluation_date: &str,
    executable: &Path,
) -> Result<GitignoreEvaluation> {
    let catalog = contract::catalog().map_err(EgolintError::Configuration)?;
    let mut report = new_report(policy, policy_path, evaluation_date, catalog.source)?;
    let sources = match contract::load_sources(workspace, policy, &report.source) {
        Ok(sources) => {
            report.checks.source_integrity = Check::Passed;
            Some(sources)
        }
        Err(message) => {
            report.checks.source_integrity = Check::Failed;
            issue(
                &mut report,
                "SOURCE",
                Check::Failed,
                None,
                message,
                "Supply the reviewed immutable Empathy export; do not fetch or rewrite it during validation.",
            );
            None
        }
    };
    let composition = sources.as_ref().and_then(|sources| {
        match contract::load_composition(workspace, policy, &report.source, sources) {
            Ok(composition) => { report.checks.composition = Check::Passed; Some(composition) }
            Err(message) => { report.checks.composition = Check::Failed;
                issue(&mut report, "COMPOSITION", Check::Failed, Some(&policy.composition_path), message,
                      "Regenerate with Empathy, preserve explicit local additions, and review selection, ordering, and digests."); None }
        }
    });
    let snapshot = inventory::collect(workspace);
    inventory_evidence(policy, composition.as_ref(), &snapshot, &mut report);
    let git = Git::new(executable);
    let tracked = match git
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|git| git.tracked(workspace))
    {
        Ok(paths) => {
            report.checks.tracked_files = Check::Passed;
            Some(paths)
        }
        Err(message) => {
            report.checks.tracked_files = Check::Unavailable;
            issue(
                &mut report,
                "EVIDENCE",
                Check::Unavailable,
                None,
                message,
                "Provide Git and a complete readable worktree/index; no tracked-file pass is claimed.",
            );
            None
        }
    };
    if let Some(composition) = &composition {
        evaluate_behavior(
            policy,
            composition,
            &catalog.probes,
            &snapshot,
            tracked.as_ref(),
            &git,
            &mut report,
        );
    }
    if inventory::collect(workspace) != snapshot {
        report.checks.coverage = Check::Incomplete;
        issue(
            &mut report,
            "EVIDENCE",
            Check::Incomplete,
            None,
            "Repository policy or pathname evidence changed during evaluation.",
            "Pause other writers and rerun against a stable worktree.",
        );
    }
    if let (Ok(git), Some(tracked)) = (&git, &tracked) {
        if git.tracked(workspace).as_ref() != Ok(tracked) {
            report.checks.coverage = Check::Incomplete;
            issue(
                &mut report,
                "EVIDENCE",
                Check::Incomplete,
                None,
                "Tracked pathname evidence changed during evaluation.",
                "Pause index writers and rerun against a stable worktree.",
            );
        }
    }
    if sources.is_some() && contract::load_sources(workspace, policy, &report.source).is_err()
        || composition.is_some()
            && sources.as_ref().is_some_and(|source| {
                contract::load_composition(workspace, policy, &report.source, source).is_err()
            })
    {
        report.checks.coverage = Check::Incomplete;
        issue(
            &mut report,
            "EVIDENCE",
            Check::Incomplete,
            None,
            "Provider or composition evidence changed during validation.",
            "Restore reviewed input bytes and rerun.",
        );
    }
    finish(report)
}

fn evaluate_behavior(
    policy: &RepositoryGitignorePolicy,
    composition: &Composition,
    probes: &[super::GitignoreProbe],
    snapshot: &Snapshot,
    tracked: Option<&BTreeSet<String>>,
    git: &std::result::Result<Git, String>,
    report: &mut RepositoryGitignoreReport,
) {
    let paths = probe_paths(policy, probes, snapshot, tracked);
    match (git.as_ref(), paths) {
        (Ok(git), Ok(paths)) => {
            let expected: BTreeMap<_, _> = composition
                .files
                .iter()
                .map(|file| (file.path.clone(), file.content.as_bytes().to_vec()))
                .collect();
            let behavior = git
                .evaluate("expected", &expected, &snapshot.directories, &paths)
                .and_then(|reference| {
                    git.evaluate("actual", &snapshot.policies, &snapshot.directories, &paths)
                        .map(|actual| (reference, actual))
                });
            match behavior {
                Ok((reference, actual)) => {
                    compare_behavior(policy, snapshot, tracked, &reference, &actual, report);
                }
                Err(message) => unavailable(report, message),
            }
        }
        (Err(message), _) => unavailable(report, message.clone()),
        (_, Err(message)) => unavailable(report, message),
    }
}

fn new_report(
    policy: &RepositoryGitignorePolicy,
    policy_path: &Path,
    evaluation_date: &str,
    source: super::GitignoreSource,
) -> Result<RepositoryGitignoreReport> {
    Ok(RepositoryGitignoreReport {
        schema_version: CONTRACT_VERSION, contract: "egolint.repository-gitignore-report/v1".to_owned(),
        repository: policy.repository.clone(), policy_path: policy_path.to_path_buf(),
        policy_sha256: contract::canonical_digest(&serde_json::to_value(policy)?)
            .map_err(EgolintError::Configuration)?, source,
        composition_sha256: policy.composition_sha256.clone(), evaluation_date: evaluation_date.to_owned(),
        status: Status::Incomplete,
        checks: GitignoreChecks { source_integrity: Check::NotEvaluated, composition: Check::NotEvaluated,
            presence: Check::Passed, content: Check::NotEvaluated, effective_behavior: Check::NotEvaluated,
            nested_policy: Check::Passed, tracked_files: Check::NotEvaluated, coverage: Check::Passed },
        files: Vec::new(), behavior: Vec::new(), diagnostics: Vec::new(),
        limitations: vec![
            "Finite path probes compare repository .gitignore policies in isolated Git repositories; this is not a proof over every possible future pathname.".to_owned(),
            "Ambient global/system excludes and .git/info/exclude are intentionally excluded; findings describe portable repository policy, not every developer's local visibility.".to_owned(),
            "Only ignore/input policies, path metadata, and index metadata are read. File payloads, history, secret scanning, forced additions, and future worktree changes are outside coverage.".to_owned(),
            "The upstream resolved-manifest digest is retained in the composition; full foundation manifest resolution remains Empathy's check-gitignore-plan boundary.".to_owned(),
            "Inventory is bounded to 50000 entries/probes, 256 policies, 1 MiB per input, and 8 MiB total ignore text. Symlinks, nested Git repositories, unreadable paths, and budget exhaustion prevent complete coverage.".to_owned(),
        ],
    })
}

fn unavailable(report: &mut RepositoryGitignoreReport, message: String) {
    report.checks.effective_behavior = Check::Unavailable;
    issue(
        report,
        "EVIDENCE",
        Check::Unavailable,
        None,
        message,
        "Provide supported Git and complete bounded policy/path evidence; semantic conformance is not established.",
    );
}

// Keep the three policy-inventory dispositions together for review.
#[allow(clippy::too_many_lines)]
fn inventory_evidence(
    policy: &RepositoryGitignorePolicy,
    composition: Option<&Composition>,
    snapshot: &Snapshot,
    report: &mut RepositoryGitignoreReport,
) {
    if composition.is_some() {
        report.checks.content = Check::Passed;
    }
    for scope in &policy.scopes {
        let path = scope_path(&scope.root, ".gitignore");
        let actual = snapshot.policies.get(&path).map(|raw| digest(raw));
        let expected = composition
            .and_then(|composition| composition.files.iter().find(|file| file.path == path))
            .map(|file| file.content_sha256.clone());
        if actual.is_none() {
            report.checks.presence = Check::Failed;
            issue(
                report,
                "FILE",
                Check::Failed,
                Some(&path),
                "Declared ignore policy is missing, unsafe, or unreadable.",
                "Review and materialize the declared scope through Holon; file presence alone is insufficient.",
            );
        }
        if expected.is_some() && actual != expected {
            report.checks.content = Check::Failed;
            issue(
                report,
                "CONTENT",
                Check::Failed,
                Some(&path),
                "Consumer bytes differ from the exact declared composition.",
                "Preserve local edits, reconcile them explicitly in Empathy, then review a new Holon plan.",
            );
        }
        report.files.push(GitignoreFileEvidence {
            path,
            scope: scope.root.clone(),
            expected_sha256: expected,
            actual_sha256: actual,
            disposition: "declared-composition".to_owned(),
        });
    }
    for (path, raw) in &snapshot.policies {
        if report.files.iter().any(|file| &file.path == path) {
            continue;
        }
        let sha = digest(raw);
        let reviewed = policy
            .nested_policies
            .iter()
            .find(|nested| &nested.path == path);
        let disposition = if let Some(nested) = reviewed {
            if sha == nested.sha256 {
                "reviewed-inventory"
            } else {
                report.checks.nested_policy = Check::Failed;
                issue(
                    report,
                    "NESTED",
                    Check::Failed,
                    Some(path),
                    "Inherited policy changed after review.",
                    "Reconcile with its canonical owner and renew exact-byte review.",
                );
                "changed-after-review"
            }
        } else {
            if report.checks.nested_policy != Check::Failed {
                report.checks.nested_policy = Check::Incomplete;
            }
            issue(
                report,
                "NESTED",
                Check::Incomplete,
                Some(path),
                "Undeclared nested ignore policy has no owner/disposition evidence.",
                "Declare an Empathy scope or record a reviewed policy digest, owner, reason, and approval. Inventory review grants no behavioral exception.",
            );
            "unreviewed"
        };
        report.files.push(GitignoreFileEvidence {
            path: path.clone(),
            scope: super::scope_of(path),
            expected_sha256: reviewed.map(|nested| nested.sha256.clone()),
            actual_sha256: Some(sha),
            disposition: disposition.to_owned(),
        });
    }
    for nested in &policy.nested_policies {
        if !snapshot.policies.contains_key(&nested.path) {
            report.checks.nested_policy = Check::Failed;
            issue(
                report,
                "NESTED",
                Check::Failed,
                Some(&nested.path),
                "Reviewed inherited policy is missing or unsupported.",
                "Refresh the explicit inventory after owner review.",
            );
        }
    }
    for (path, message) in &snapshot.problems {
        report.checks.coverage = Check::Incomplete;
        issue(
            report,
            "EVIDENCE",
            Check::Incomplete,
            path.as_deref(),
            message.clone(),
            "Resolve the unsupported boundary or validate its repository separately; incomplete inventory cannot pass.",
        );
    }
    report
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
}

fn probe_paths(
    policy: &RepositoryGitignorePolicy,
    probes: &[super::GitignoreProbe],
    snapshot: &Snapshot,
    tracked: Option<&BTreeSet<String>>,
) -> std::result::Result<BTreeSet<String>, String> {
    let mut paths = snapshot.paths.clone();
    if let Some(tracked) = tracked {
        paths.extend(tracked.iter().cloned());
    }
    let roots: BTreeSet<_> = policy
        .scopes
        .iter()
        .map(|scope| scope.root.clone())
        .chain(snapshot.policies.keys().map(|path| scope_of(path)))
        .collect();
    for root in roots {
        paths.extend(probes.iter().map(|probe| scope_path(&root, &probe.path)));
        if paths.len() > MAX_PROBES {
            return Err("effective-behavior probes exceeded 50000 paths".to_owned());
        }
    }
    paths.extend(policy.probes.iter().map(|probe| probe.path.clone()));
    paths.extend(
        policy
            .exceptions
            .iter()
            .map(|exception| exception.path.clone()),
    );
    if paths.len() > MAX_PROBES {
        return Err("effective-behavior probes exceeded 50000 paths".to_owned());
    }
    Ok(paths)
}

fn applicable_exception<'a>(
    policy: &'a RepositoryGitignorePolicy,
    snapshot: &Snapshot,
    path: &str,
    actual: &Match,
    date: &str,
) -> Option<&'a GitignoreException> {
    let exception = policy
        .exceptions
        .iter()
        .find(|exception| exception.path == path)?;
    let winner = actual.rule.as_ref()?;
    (exception.expires_on.as_str() >= date
        && winner.path == exception.policy_path
        && snapshot
            .policies
            .get(&winner.path)
            .is_some_and(|raw| digest(raw) == exception.policy_sha256))
    .then_some(exception)
}

#[allow(clippy::too_many_lines)]
fn compare_behavior(
    policy: &RepositoryGitignorePolicy,
    snapshot: &Snapshot,
    tracked: Option<&BTreeSet<String>>,
    reference: &BTreeMap<String, Match>,
    actual: &BTreeMap<String, Match>,
    report: &mut RepositoryGitignoreReport,
) {
    report.checks.effective_behavior = Check::Passed;
    let mut applied = BTreeSet::new();
    for (path, expected_match) in reference {
        let observed = &actual[path];
        let expected = policy
            .probes
            .iter()
            .find(|probe| &probe.path == path)
            .map_or(expected_match.ignored, |probe| probe.ignored);
        let is_tracked = tracked.is_some_and(|tracked| tracked.contains(path));
        let exception =
            applicable_exception(policy, snapshot, path, observed, &report.evaluation_date);
        let behavior_exception = exception
            .is_some_and(|exception| exception.ignored == Some(observed.ignored))
            && observed.ignored != expected;
        let tracked_exception = is_tracked
            && (observed.ignored || expected_match.ignored)
            && exception.is_some_and(|exception| exception.allow_tracked);
        if observed.ignored != expected && !behavior_exception {
            report.checks.effective_behavior = Check::Failed;
            let winner = observed.rule.as_ref().map_or_else(
                || "no matching rule".to_owned(),
                |rule| format!("{}:{} {:?}", rule.path, rule.line, rule.pattern),
            );
            issue(
                report,
                "BEHAVIOR",
                Check::Failed,
                Some(path),
                format!(
                    "Expected ignored={expected}, observed ignored={}; winning rule: {winner}.",
                    observed.ignored
                ),
                "Reconcile the owning ignore policy and excluded-parent semantics; a nested override needs an exact reviewed exception.",
            );
        }
        if is_tracked && (observed.ignored || expected_match.ignored) && !tracked_exception {
            report.checks.tracked_files = Check::Failed;
            issue(
                report,
                "TRACKED",
                Check::Failed,
                Some(path),
                "Git already tracks a path protected by the declared or effective ignore policy.",
                "Review tracked ownership and any exposure with the owner; ignore rules do not protect tracked files. No content was read or unstaged.",
            );
        }
        let exception_reference = if behavior_exception || tracked_exception {
            applied.insert(path.clone());
            let exception = exception.expect("an applied exception was validated");
            issue(
                report,
                "EXCEPTION",
                Check::Passed,
                Some(path),
                format!(
                    "Exact-path exception accepted for owner {} until {}; approval {}.",
                    exception.owner, exception.expires_on, exception.approval
                ),
                "This review does not grant a blanket exemption or establish payload safety.",
            );
            Some(exception.approval.clone())
        } else {
            None
        };
        report.behavior.push(GitignoreBehaviorEvidence {
            path: path.clone(),
            scope: observed
                .rule
                .as_ref()
                .map_or_else(|| ".".to_owned(), |rule| scope_of(&rule.path)),
            expected_ignored: expected,
            actual_ignored: observed.ignored,
            winning_rule: observed.rule.clone(),
            tracked: is_tracked,
            exception: exception_reference,
        });
    }
    for exception in &policy.exceptions {
        if !applied.contains(&exception.path) {
            report.checks.effective_behavior = Check::Failed;
            issue(
                report,
                "EXCEPTION",
                Check::Failed,
                Some(&exception.path),
                "Exception is expired, stale, unmatched, or unnecessary for the observed policy/index.",
                "Remove unused exceptions or renew exact path, policy digest, winning rule, owner, approval, and expiry evidence.",
            );
        }
    }
}

fn finish(mut report: RepositoryGitignoreReport) -> Result<GitignoreEvaluation> {
    let checks = [
        &report.checks.source_integrity,
        &report.checks.composition,
        &report.checks.presence,
        &report.checks.content,
        &report.checks.effective_behavior,
        &report.checks.nested_policy,
        &report.checks.tracked_files,
        &report.checks.coverage,
    ];
    report.status = if checks.iter().any(|status| **status == Check::Failed) {
        Status::Invalid
    } else if checks.iter().any(|status| **status != Check::Passed) {
        Status::Incomplete
    } else if report
        .behavior
        .iter()
        .any(|evidence| evidence.exception.is_some())
    {
        Status::ValidWithExceptions
    } else {
        Status::Valid
    };
    Ok(GitignoreEvaluation {
        findings: findings(&report)?,
        report,
    })
}
