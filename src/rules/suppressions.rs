//! Deterministic application of reviewed, expiring suppressions.

use std::collections::BTreeSet;

use std::path::PathBuf;

use crate::contracts::{
    CONTRACT_VERSION, EvidenceKind, EvidenceReference, Finding, RuleIdentity, RuleOwnership,
    Severity, SourceLocation, Suppression, SuppressionState,
};
use crate::error::{EgolintError, Result};

/// Apply exact suppressions and record their observed states.
///
/// `today` uses `YYYY-MM-DD`; a suppression remains active through its expiry
/// date and becomes expired on the following day. A rule-specific callback
/// prevents generic suppression code from weakening non-suppressible rules.
/// Suppressions are exact by rule plus path and/or fingerprint; broad rule-only
/// declarations are rejected.
///
/// # Errors
///
/// Returns an error for invalid contracts, duplicate IDs, ambiguous matches,
/// broad selectors, an invalid evaluation date, or a rule the caller marks as
/// non-suppressible.
pub fn apply_suppressions<F>(
    findings: &mut Vec<Finding>,
    suppressions: &mut [Suppression],
    today: &str,
    is_suppressible: F,
) -> Result<()>
where
    F: Fn(&RuleIdentity) -> bool,
{
    crate::contracts::validate_contract_date(today)?;
    let mut ids = BTreeSet::new();
    let mut expired_findings = Vec::new();
    for suppression in suppressions.iter_mut() {
        suppression.validate()?;
        if suppression.evidence.is_empty() {
            return Err(EgolintError::Configuration(format!(
                "suppression {} must include reviewed evidence",
                suppression.id
            )));
        }
        if !ids.insert(suppression.id.as_str()) {
            return Err(EgolintError::Configuration(format!(
                "duplicate suppression id {}",
                suppression.id
            )));
        }
        if suppression.path.is_none() && suppression.fingerprint.is_none() {
            return Err(EgolintError::Configuration(format!(
                "suppression {} must select an exact path and/or fingerprint",
                suppression.id
            )));
        }
        if !is_suppressible(&suppression.rule) {
            return Err(EgolintError::Configuration(format!(
                "rule {}/{} may not be suppressed",
                suppression.rule.tool_id, suppression.rule.rule_id
            )));
        }
        suppression.state = if suppression.expires_on.as_str() < today {
            SuppressionState::Expired
        } else {
            SuppressionState::Unmatched
        };
        if suppression.state == SuppressionState::Expired {
            expired_findings.push(expired_suppression_finding(suppression));
        }
    }

    findings.extend(expired_findings);

    for finding in findings.iter_mut() {
        finding.validate()?;
        if finding.suppressed_by.is_some() {
            return Err(EgolintError::Configuration(format!(
                "finding {} was already suppressed before rule evaluation",
                finding.id
            )));
        }
        let matches = suppressions
            .iter()
            .enumerate()
            .filter(|(_, suppression)| suppression.state != SuppressionState::Expired)
            .filter(|(_, suppression)| suppression.rule == finding.rule)
            .filter(|(_, suppression)| selector_matches(finding, suppression))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            let ids = matches
                .iter()
                .map(|index| suppressions[*index].id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(EgolintError::Configuration(format!(
                "finding {} matches multiple suppressions: {ids}",
                finding.id
            )));
        }
        if let Some(index) = matches.first().copied() {
            finding.suppressed_by = Some(suppressions[index].id.clone());
            suppressions[index].state = SuppressionState::Applied;
        }
        finding.validate()?;
    }
    findings.extend(
        suppressions
            .iter()
            .filter(|suppression| suppression.state == SuppressionState::Unmatched)
            .map(unmatched_suppression_finding),
    );
    Ok(())
}

fn selector_matches(finding: &Finding, suppression: &Suppression) -> bool {
    let path_matches = suppression.path.as_ref().is_none_or(|expected| {
        finding
            .location
            .as_ref()
            .is_some_and(|location| location.path == *expected)
    });
    let fingerprint_matches = suppression
        .fingerprint
        .as_ref()
        .is_none_or(|expected| finding.fingerprint.as_ref() == Some(expected));
    path_matches && fingerprint_matches
}

fn expired_suppression_finding(suppression: &Suppression) -> Finding {
    let location_path = suppression
        .path
        .clone()
        .or_else(|| {
            suppression
                .evidence
                .first()
                .map(|evidence| evidence.path.clone())
        })
        .unwrap_or_else(|| PathBuf::from(".egolint/suppressions.toml"));
    let fingerprint = stable_expiry_fingerprint(suppression);
    let evidence = if suppression.evidence.is_empty() {
        vec![EvidenceReference {
            schema_version: CONTRACT_VERSION,
            kind: EvidenceKind::Configuration,
            path: PathBuf::from(".egolint/suppressions.toml"),
            sha256: None,
            description: Some("Expired suppression declaration.".to_owned()),
        }]
    } else {
        suppression.evidence.clone()
    };
    Finding {
        schema_version: CONTRACT_VERSION,
        id: format!("EGO-SUPPRESSION-EXPIRED-{fingerprint}"),
        rule: RuleIdentity {
            tool_id: "EGOLINT_SUPPRESSIONS".to_owned(),
            rule_id: "EGO-SUPPRESSION-EXPIRED".to_owned(),
        },
        severity: Severity::Error,
        message: format!(
            "suppression {} owned by {} expired on {}; renew with reviewed evidence or remove it",
            suppression.id, suppression.owner, suppression.expires_on
        ),
        location: Some(SourceLocation {
            path: location_path,
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        }),
        ownership: RuleOwnership {
            owner: "egohygiene/egolint".to_owned(),
            policy_source: "docs/suppressions.md".to_owned(),
            configuration_path: None,
        },
        fingerprint: Some(fingerprint),
        evidence,
        suppressed_by: None,
    }
}

fn unmatched_suppression_finding(suppression: &Suppression) -> Finding {
    let location_path = suppression
        .path
        .clone()
        .or_else(|| {
            suppression
                .evidence
                .first()
                .map(|evidence| evidence.path.clone())
        })
        .unwrap_or_else(|| PathBuf::from(".egolint/suppressions.toml"));
    let fingerprint = stable_unmatched_fingerprint(suppression);
    Finding {
        schema_version: CONTRACT_VERSION,
        id: format!("EGO-SUPPRESSION-UNMATCHED-{fingerprint}"),
        rule: RuleIdentity {
            tool_id: "EGOLINT_SUPPRESSIONS".to_owned(),
            rule_id: "EGO-SUPPRESSION-UNMATCHED".to_owned(),
        },
        severity: Severity::Warning,
        message: format!(
            "suppression {} owned by {} matched no current finding; review or remove it",
            suppression.id, suppression.owner
        ),
        location: Some(SourceLocation {
            path: location_path,
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        }),
        ownership: RuleOwnership {
            owner: "egohygiene/egolint".to_owned(),
            policy_source: "docs/suppressions.md".to_owned(),
            configuration_path: None,
        },
        fingerprint: Some(fingerprint),
        evidence: suppression.evidence.clone(),
        suppressed_by: None,
    }
}

fn stable_expiry_fingerprint(suppression: &Suppression) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in suppression
        .id
        .bytes()
        .chain([0])
        .chain(suppression.expires_on.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("suppression-expiry-v1-{hash:016x}")
}

fn stable_unmatched_fingerprint(suppression: &Suppression) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in suppression
        .id
        .bytes()
        .chain([0])
        .chain(suppression.rule.tool_id.bytes())
        .chain([0])
        .chain(suppression.rule.rule_id.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("suppression-unmatched-v1-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::contracts::{
        CONTRACT_VERSION, EvidenceKind, EvidenceReference, RuleOwnership, Severity, SourceLocation,
    };

    use super::*;

    fn finding(path: &str) -> Finding {
        Finding {
            schema_version: CONTRACT_VERSION,
            id: "EGO-PORT-CMD-001-test".to_owned(),
            rule: RuleIdentity {
                tool_id: "EGOLINT_PORTABILITY".to_owned(),
                rule_id: "EGO-PORT-CMD-001".to_owned(),
            },
            severity: Severity::Warning,
            message: "portable command assumption".to_owned(),
            location: Some(SourceLocation {
                path: PathBuf::from(path),
                start_line: Some(2),
                start_column: Some(1),
                end_line: Some(2),
                end_column: None,
            }),
            ownership: RuleOwnership {
                owner: "egohygiene/egolint".to_owned(),
                policy_source: ".config/rules/portability.toml".to_owned(),
                configuration_path: Some(PathBuf::from(".config/rules/portability.toml")),
            },
            fingerprint: Some("portability-v1-example".to_owned()),
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_VERSION,
                kind: EvidenceKind::Fixture,
                path: PathBuf::from("tests/fixtures/contracts/suppression.toml"),
                sha256: None,
                description: Some("Synthetic suppression fixture.".to_owned()),
            }],
            suppressed_by: None,
        }
    }

    fn suppression(id: &str, path: &str, expires_on: &str) -> Suppression {
        Suppression {
            schema_version: CONTRACT_VERSION,
            id: id.to_owned(),
            rule: RuleIdentity {
                tool_id: "EGOLINT_PORTABILITY".to_owned(),
                rule_id: "EGO-PORT-CMD-001".to_owned(),
            },
            path: Some(PathBuf::from(path)),
            fingerprint: None,
            owner: "egohygiene/example".to_owned(),
            justification: "Temporary platform migration with a named owner.".to_owned(),
            expires_on: expires_on.to_owned(),
            state: SuppressionState::Unmatched,
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_VERSION,
                kind: EvidenceKind::Fixture,
                path: PathBuf::from("tests/fixtures/contracts/suppression.toml"),
                sha256: None,
                description: Some("Synthetic suppression fixture.".to_owned()),
            }],
        }
    }

    #[test]
    fn exact_active_suppression_is_applied() {
        let mut findings = vec![finding("scripts/install.sh")];
        let mut suppressions = vec![suppression(
            "SUP-PORT-001",
            "scripts/install.sh",
            "2026-09-01",
        )];

        apply_suppressions(&mut findings, &mut suppressions, "2026-08-19", |_| true)
            .expect("suppression application");

        assert_eq!(findings[0].suppressed_by.as_deref(), Some("SUP-PORT-001"));
        assert_eq!(suppressions[0].state, SuppressionState::Applied);
    }

    #[test]
    fn expired_or_unmatched_suppression_cannot_hide_a_finding() {
        let mut findings = vec![finding("scripts/install.sh")];
        let mut suppressions = vec![
            suppression("SUP-EXPIRED", "scripts/install.sh", "2026-08-18"),
            suppression("SUP-UNMATCHED", "scripts/other.sh", "2026-09-01"),
        ];

        apply_suppressions(&mut findings, &mut suppressions, "2026-08-19", |_| true)
            .expect("suppression evaluation");

        assert!(findings.iter().any(|finding| {
            finding.rule.rule_id == "EGO-SUPPRESSION-EXPIRED" && finding.severity == Severity::Error
        }));
        assert!(findings.iter().any(|finding| {
            finding.rule.rule_id == "EGO-PORT-CMD-001" && finding.suppressed_by.is_none()
        }));
        assert_eq!(suppressions[0].state, SuppressionState::Expired);
        assert_eq!(suppressions[1].state, SuppressionState::Unmatched);
        assert!(findings.iter().any(|finding| {
            finding.rule.rule_id == "EGO-SUPPRESSION-UNMATCHED"
                && finding.severity == Severity::Warning
        }));
    }

    #[test]
    fn ambiguous_or_nonsuppressible_declarations_are_rejected() {
        let mut findings = vec![finding("scripts/install.sh")];
        let mut ambiguous = vec![
            suppression("SUP-ONE", "scripts/install.sh", "2026-09-01"),
            suppression("SUP-TWO", "scripts/install.sh", "2026-09-01"),
        ];
        assert!(apply_suppressions(&mut findings, &mut ambiguous, "2026-08-19", |_| true).is_err());

        let mut findings = vec![finding("scripts/install.sh")];
        let mut denied = vec![suppression(
            "SUP-DENIED",
            "scripts/install.sh",
            "2026-09-01",
        )];
        assert!(apply_suppressions(&mut findings, &mut denied, "2026-08-19", |_| false).is_err());

        let mut findings = vec![finding("scripts/install.sh")];
        let mut missing_evidence = vec![suppression(
            "SUP-NO-EVIDENCE",
            "scripts/install.sh",
            "2026-09-01",
        )];
        missing_evidence[0].evidence.clear();
        assert!(
            apply_suppressions(&mut findings, &mut missing_evidence, "2026-08-19", |_| true)
                .is_err()
        );
    }
}
