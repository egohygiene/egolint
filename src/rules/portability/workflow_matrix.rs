//! Bounded static matrix expansion, never GitHub expression execution.

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::{Mapping, Value};

use super::mapping_value;

const MAX_COMBINATIONS: usize = 256;
const MAX_STATIC_NODES: usize = 4096;
const MAX_STATIC_DEPTH: usize = 16;
type Row<'a> = BTreeMap<String, &'a Value>;

pub(super) fn job_runs_on_multiple_operating_systems(job: &Mapping) -> bool {
    operating_system_families(job).is_some_and(|families| families.len() > 1)
}

fn operating_system_families(job: &Mapping) -> Option<BTreeSet<&'static str>> {
    let selector = selector_path(mapping_value(job, "runs-on")?.as_str()?)?;
    let matrix = mapping_value(mapping_value(job, "strategy")?.as_mapping()?, "matrix")?;
    let mut remaining = MAX_STATIC_NODES;
    if !static_value(matrix, 0, &mut remaining) {
        return None;
    }
    let mut families = BTreeSet::new();
    for row in expand_matrix(matrix.as_mapping()?)? {
        families.insert(operating_system_family(
            selected_value(&row, &selector)?.as_str()?,
        )?);
    }
    Some(families)
}

fn selector_path(value: &str) -> Option<Vec<&str>> {
    let expression = value.trim().strip_prefix("${{")?.strip_suffix("}}")?.trim();
    let selector = expression.strip_prefix("matrix.")?;
    let path = selector.split('.').collect::<Vec<_>>();
    path.iter()
        .all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
        .then_some(path)
}

fn static_value(value: &Value, depth: usize, remaining: &mut usize) -> bool {
    if depth > MAX_STATIC_DEPTH || *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => true,
        Value::String(value) => !value.contains("${{"),
        Value::Sequence(values) => values
            .iter()
            .all(|value| static_value(value, depth + 1, remaining)),
        Value::Mapping(values) => values.iter().all(|(key, value)| {
            key.as_str().is_some_and(|key| !key.contains("${{"))
                && static_value(value, depth + 1, remaining)
        }),
        Value::Tagged(_) => false,
    }
}

fn additional_rows<'a>(matrix: &'a Mapping, key: &str) -> Option<Vec<Row<'a>>> {
    let Some(value) = mapping_value(matrix, key) else {
        return Some(Vec::new());
    };
    value
        .as_sequence()?
        .iter()
        .map(|value| {
            value
                .as_mapping()?
                .iter()
                .map(|(key, value)| Some((key.as_str()?.to_owned(), value)))
                .collect()
        })
        .collect()
}

fn matches(row: &Row<'_>, pattern: &Row<'_>) -> bool {
    pattern
        .iter()
        .all(|(key, value)| row.get(key) == Some(value))
}

fn expand_matrix(matrix: &Mapping) -> Option<Vec<Row<'_>>> {
    let includes = additional_rows(matrix, "include")?;
    let excludes = additional_rows(matrix, "exclude")?;
    let mut rows = vec![Row::new()];
    let mut has_axes = false;
    for (key, values) in matrix {
        let key = key.as_str()?;
        if matches!(key, "include" | "exclude") {
            continue;
        }
        has_axes = true;
        let values = values.as_sequence()?;
        if values.is_empty() || rows.len().checked_mul(values.len())? > MAX_COMBINATIONS {
            return None;
        }
        rows = rows
            .into_iter()
            .flat_map(|row| {
                values.iter().map(move |value| {
                    let mut result = row.clone();
                    result.insert(key.to_owned(), value);
                    result
                })
            })
            .collect();
    }
    // An include-only matrix creates one job per row, not a phantom empty base job.
    if !has_axes {
        rows.clear();
    }
    let mut original = rows
        .into_iter()
        .filter(|row| !excludes.iter().any(|pattern| matches(row, pattern)))
        .map(|row| (row.clone(), row))
        .collect::<Vec<_>>();
    let mut added = Vec::new();
    for include in includes {
        let mut merged = false;
        for (base, selected) in &mut original {
            // Includes may overwrite earlier added metadata, never an original axis.
            if include
                .iter()
                .all(|(key, value)| base.get(key).is_none_or(|prior| prior == value))
            {
                selected.extend(include.clone());
                merged = true;
            }
        }
        // Later includes never merge with these newly added combinations.
        if !merged {
            added.push(include);
        }
        if original.len() + added.len() > MAX_COMBINATIONS {
            return None;
        }
    }
    Some(
        original
            .into_iter()
            .map(|(_, row)| row)
            .chain(added)
            .collect(),
    )
}

fn selected_value<'a>(row: &Row<'a>, selector: &[&str]) -> Option<&'a Value> {
    let mut value = *row.get(*selector.first()?)?;
    for field in &selector[1..] {
        value = mapping_value(value.as_mapping()?, field)?;
    }
    Some(value)
}

fn operating_system_family(value: &str) -> Option<&'static str> {
    let value = value.to_ascii_lowercase();
    if value.contains("windows") {
        Some("windows")
    } else if value.contains("macos") || value.contains("osx") {
        Some("macos")
    } else if value.contains("ubuntu") || value.contains("linux") {
        Some("linux")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn families(yaml: &str) -> Option<BTreeSet<&'static str>> {
        let job: Value = serde_yaml::from_str(yaml).unwrap();
        operating_system_families(job.as_mapping().unwrap())
    }

    #[test]
    fn direct_and_include_only_representations_resolve_identically() {
        let direct = "runs-on: ${{ matrix.os }}\nstrategy:\n  matrix:\n    os: [ubuntu-24.04, windows-2025]\n";
        let includes = "runs-on: ${{ matrix.os }}\nstrategy:\n  matrix:\n    include:\n      - os: ubuntu-24.04\n      - os: windows-2025\n";
        assert_eq!(families(direct), families(includes));
        assert_eq!(families(direct).unwrap().len(), 2);
    }

    #[test]
    fn exclusions_refine_combinations_and_includes_can_restore_them() {
        let base = "runs-on: ${{ matrix.os }}\nstrategy:\n  matrix:\n    os: [ubuntu-24.04, windows-2025]\n    version: [18, 20]\n    exclude:\n";
        let partial = format!("{base}      - os: windows-2025\n        version: 18\n");
        assert_eq!(families(&partial).unwrap().len(), 2);
        let removed = format!("{base}      - os: windows-2025\n");
        assert_eq!(families(&removed).unwrap(), BTreeSet::from(["linux"]));
        let restored =
            format!("{removed}    include:\n      - os: windows-2025\n        version: 18\n");
        assert_eq!(families(&restored).unwrap().len(), 2);
    }

    #[test]
    fn include_overlays_follow_original_combinations_and_order() {
        let matrix: Value = serde_yaml::from_str("fruit: [apple, pear]\nanimal: [cat, dog]\ninclude:\n  - color: green\n  - color: pink\n    animal: cat\n  - fruit: banana\n  - fruit: banana\n    animal: cat\n").unwrap();
        let rows = expand_matrix(matrix.as_mapping().unwrap()).unwrap();
        assert_eq!(rows.len(), 6);
        for row in &rows[..4] {
            let expected = if row["animal"].as_str() == Some("cat") {
                "pink"
            } else {
                "green"
            };
            assert_eq!(row["color"].as_str(), Some(expected));
        }
        assert_eq!(rows[4].len(), 1);
        assert_eq!(rows[5].len(), 2);
    }

    #[test]
    fn selectors_are_exact_and_can_follow_static_object_axes() {
        let nested = "runs-on: '${{ matrix.runner.os }}'\nstrategy:\n  matrix:\n    runner: [{os: ubuntu-24.04}, {os: windows-2025}]\n";
        assert_eq!(families(nested).unwrap().len(), 2);
        assert!(
            families(&nested.replace("matrix.runner.os", "matrix.runner.os-version")).is_none()
        );
        let prefix = "runs-on: ${{ matrix.os-version }}\nstrategy:\n  matrix:\n    os: [ubuntu-24.04, windows-2025]\n    os-version: [macos-15]\n";
        assert_eq!(families(prefix).unwrap(), BTreeSet::from(["macos"]));
        assert!(selector_path("prefix-${{ matrix.os }}").is_none());
        assert!(selector_path("${{ matrix.os || 'ubuntu-latest' }}").is_none());
        assert!(selector_path("${{ matrix['os'] }}").is_none());
    }

    #[test]
    fn dynamic_inputs_and_expansion_limits_stay_unknown() {
        for matrix in [
            "${{ fromJSON(inputs.matrix) }}",
            "{os: ['${{ inputs.runner }}', windows-2025]}",
            "{include: '${{ inputs.rows }}'}",
        ] {
            let job = format!("runs-on: ${{{{ matrix.os }}}}\nstrategy:\n  matrix: {matrix}\n");
            assert!(families(&job).is_none(), "{matrix}");
        }
        let oversized = format!(
            "runs-on: ${{{{ matrix.os }}}}\nstrategy:\n  matrix:\n    os: [ubuntu-24.04, windows-2025]\n    version: [{}]\n",
            (0..129)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        assert!(families(&oversized).is_none());
    }
}
