//! Closed consumer inputs and data-only verification of the pinned composer format.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    CATALOG, GitignoreProbe, GitignoreSource, RepositoryGitignorePolicy, digest, safe_path,
    scope_path,
};

type Checked<T> = std::result::Result<T, String>;

#[derive(Deserialize)]
pub(super) struct Catalog {
    pub source: GitignoreSource,
    pub probes: Vec<GitignoreProbe>,
}

pub(super) fn catalog() -> Checked<Catalog> {
    serde_json::from_str(CATALOG).map_err(|_| "bundled ignore catalog is invalid".to_owned())
}

fn require(condition: bool, message: &str) -> Checked<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn is_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn unique(values: &[String]) -> bool {
    values.iter().all(|value| text(value, 256))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn review(owner: &str, reason: &str, approval: &str) -> Checked<()> {
    require(
        text(owner, 256)
            && text(reason, 2048)
            && text(approval, 2048)
            && approval.starts_with("https://"),
        "review records need an owner, reason, and HTTPS approval reference",
    )
}

pub(super) fn ignore_path(path: &str) -> Checked<()> {
    safe_path(path)?;
    let parts: Vec<_> = path.split('/').collect();
    require(
        parts.last() == Some(&".gitignore")
            && !parts[..parts.len() - 1]
                .iter()
                .any(|part| part.eq_ignore_ascii_case(".gitignore")),
        "policy path must name an exact .gitignore outside another .gitignore",
    )
}

#[allow(clippy::too_many_lines)]
pub(super) fn validate_policy(policy: &RepositoryGitignorePolicy) -> Checked<()> {
    require(
        policy.schema_version == 1,
        "unsupported gitignore policy version",
    )?;
    require(
        text(&policy.id, 256)
            && text(&policy.repository, 256)
            && policy.repository.split('/').count() == 2
            && policy.repository.split('/').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
            }),
        "policy must identify a repository using owner/name",
    )?;
    require(
        is_digest(&policy.source_revision, 40) && is_digest(&policy.composition_sha256, 64),
        "source revision and composition digest must be immutable lowercase hashes",
    )?;
    if policy.source_root != "." {
        safe_path(&policy.source_root)?;
    }
    safe_path(&policy.composition_path)?;
    require(
        unique(&policy.profiles) && policy.profiles.iter().any(|profile| profile == "universal"),
        "profiles must be unique resolved IDs including universal",
    )?;
    require(
        !policy.scopes.is_empty()
            && policy.scopes.len() <= super::MAX_POLICIES
            && policy.scopes.iter().any(|scope| scope.root == "."),
        "declare root and bounded unique scopes",
    )?;
    let mut paths = BTreeSet::new();
    for scope in &policy.scopes {
        let path = scope_path(&scope.root, ".gitignore");
        ignore_path(&path)?;
        require(
            paths.insert(path.to_lowercase()) && unique(&scope.overlays),
            "duplicate scope or overlay",
        )?;
        require(
            scope.local_additions.len() <= super::MAX_FILE_BYTES
                && !scope.local_additions.contains(['\r', '\0'])
                && (scope.local_additions.is_empty() || scope.local_additions.ends_with('\n')),
            "local additions must be empty or LF-terminated text without CR or NUL",
        )?;
        require(
            scope.local_additions.is_empty()
                || policy.probes.iter().any(|probe| {
                    scope.root == "." || probe.path.starts_with(&format!("{}/", scope.root))
                }),
            "declare at least one behavior probe for each scope with local additions",
        )?;
    }
    let mut probes = BTreeSet::new();
    require(
        policy.probes.len() <= super::MAX_PROBES,
        "too many consumer probes",
    )?;
    for probe in &policy.probes {
        safe_path(&probe.path)?;
        require(probes.insert(&probe.path), "duplicate consumer probe")?;
    }
    require(
        policy.nested_policies.len() <= super::MAX_POLICIES
            && policy.exceptions.len() <= super::MAX_POLICIES,
        "reviewed policy/exception inventory exceeds the bounded limit",
    )?;
    for nested in &policy.nested_policies {
        ignore_path(&nested.path)?;
        require(
            paths.insert(nested.path.to_lowercase()) && is_digest(&nested.sha256, 64),
            "duplicate or invalid nested policy",
        )?;
        review(&nested.owner, &nested.reason, &nested.approval)?;
    }
    let mut exceptions = BTreeSet::new();
    for exception in &policy.exceptions {
        safe_path(&exception.path)?;
        ignore_path(&exception.policy_path)?;
        require(
            exceptions.insert(&exception.path)
                && is_digest(&exception.policy_sha256, 64)
                && (exception.ignored.is_some() || exception.allow_tracked),
            "invalid or duplicate exact-path exception",
        )?;
        require(
            paths.contains(&exception.policy_path.to_lowercase()),
            "exception must reference a declared policy",
        )?;
        review(&exception.owner, &exception.reason, &exception.approval)?;
        crate::contracts::validate_contract_date(&exception.expires_on)
            .map_err(|_| "exception requires a real Gregorian expiry date".to_owned())?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Composition {
    format: String,
    status: String,
    generator: String,
    foundation: String,
    repository: String,
    profiles: Vec<String>,
    source: CompositionSource,
    pub files: Vec<ComposedFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionSource {
    owner: String,
    catalog_sha256: String,
    resolved_manifest_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComposedFile {
    pub path: String,
    ownership: String,
    #[serde(rename = "override")]
    override_mode: Option<String>,
    layers: Vec<Value>,
    pub content: String,
    pub content_sha256: String,
}

pub(super) struct Sources {
    catalog: Value,
    fragments: BTreeMap<String, String>,
}

pub(super) fn canonical_digest(value: &Value) -> Checked<String> {
    // serde_json's default Map is key-sorted, including nested objects. No
    // preserve_order feature is enabled; arrays retain their declared order.
    serde_json::to_vec(value)
        .map(|bytes| digest(&bytes))
        .map_err(|_| "cannot canonicalize JSON".to_owned())
}

fn array<'a>(value: &'a Value, key: &str) -> Checked<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing source array {key}"))
}

fn string<'a>(value: &'a Value, key: &str) -> Checked<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing source field {key}"))
}

fn definition(catalog: &Value) -> Checked<&Value> {
    array(catalog, "artifacts")?
        .iter()
        .find(|artifact| artifact["id"] == "gitignore")
        .and_then(|artifact| artifact.get("composition"))
        .ok_or_else(|| "source lacks ignore definition".to_owned())
}

pub(super) fn load_sources(
    workspace: &Path,
    policy: &RepositoryGitignorePolicy,
    pin: &GitignoreSource,
) -> Checked<Sources> {
    require(
        policy.source_revision == pin.revision,
        "unsupported Empathy revision; review an EgoLint source-pin update",
    )?;
    let root = if policy.source_root == "." {
        String::new()
    } else {
        format!("{}/", policy.source_root)
    };
    let raw = super::inventory::read_file(workspace, &format!("{root}{}", pin.catalog_path))?;
    let catalog: Value =
        serde_json::from_slice(&raw).map_err(|_| "invalid Empathy catalog JSON".to_owned())?;
    require(
        canonical_digest(&catalog)? == pin.catalog_sha256,
        "Empathy catalog digest mismatch",
    )?;
    let definition = definition(&catalog)?;
    let mut fragments = BTreeMap::new();
    let baseline = definition
        .get("baseline")
        .ok_or_else(|| "missing source baseline".to_owned())?;
    for fragment in std::iter::once(baseline).chain(array(definition, "overlays")?) {
        let path = string(fragment, "path")?;
        let raw = super::inventory::read_file(workspace, &format!("{root}{path}"))?;
        require(
            digest(&raw) == string(fragment, "sha256")?,
            "Empathy fragment digest mismatch",
        )?;
        let text =
            String::from_utf8(raw).map_err(|_| "Empathy fragment must be UTF-8".to_owned())?;
        fragments.insert(string(fragment, "id")?.to_owned(), text);
    }
    Ok(Sources { catalog, fragments })
}

#[allow(clippy::too_many_lines)]
pub(super) fn load_composition(
    workspace: &Path,
    policy: &RepositoryGitignorePolicy,
    pin: &GitignoreSource,
    sources: &Sources,
) -> Checked<Composition> {
    let raw = super::inventory::read_file(workspace, &policy.composition_path)?;
    let value: Value =
        serde_json::from_slice(&raw).map_err(|_| "invalid composition JSON".to_owned())?;
    require(
        canonical_digest(&value)? == policy.composition_sha256,
        "composition digest mismatch; regenerate and review inputs",
    )?;
    let composition: Composition = serde_json::from_value(value)
        .map_err(|_| "unsupported or malformed composition fields".to_owned())?;
    require(
        composition.format == pin.format
            && composition.foundation == pin.foundation
            && composition.status == "plan-only"
            && composition.generator == "tools/foundation.py plan-gitignore",
        "unsupported Empathy composition contract",
    )?;
    require(
        composition.repository == policy.repository && composition.profiles == policy.profiles,
        "composition repository/profile selection mismatch",
    )?;
    require(
        composition.source.owner == pin.repository
            && composition.source.catalog_sha256 == pin.catalog_sha256
            && is_digest(&composition.source.resolved_manifest_sha256, 64),
        "composition provenance mismatch",
    )?;
    let profiles = array(&sources.catalog, "profiles")?;
    for id in &policy.profiles {
        let profile = profiles
            .iter()
            .find(|item| item["id"] == *id)
            .ok_or_else(|| "unknown profile".to_owned())?;
        for required in array(profile, "requires")? {
            require(
                required
                    .as_str()
                    .is_some_and(|name| policy.profiles.iter().any(|item| item == name)),
                "unresolved profile dependencies",
            )?;
        }
        for conflict in array(profile, "conflicts")? {
            require(
                !conflict
                    .as_str()
                    .is_some_and(|name| policy.profiles.iter().any(|item| item == name)),
                "conflicting profiles",
            )?;
        }
    }
    let definition = definition(&sources.catalog)?;
    let baseline = &definition["baseline"];
    require(
        composition.files.len() == policy.scopes.len(),
        "composition must contain exactly the selected scopes",
    )?;
    for (file, scope) in composition.files.iter().zip(&policy.scopes) {
        require(
            file.path == scope_path(&scope.root, ".gitignore")
                && file.ownership == "repository-owned"
                && matches!(file.override_mode.as_deref(), None | Some("preserve")),
            "composition path/ownership mismatch",
        )?;
        let mut layers = Vec::new();
        let mut content =
            "# Composed ignore rules; see the foundation plan for source hashes.\n".to_owned();
        for id in &scope.overlays {
            let source = array(definition, "overlays")?
                .iter()
                .find(|item| item["id"] == *id)
                .ok_or_else(|| "unknown selected overlay".to_owned())?;
            require(
                policy
                    .profiles
                    .iter()
                    .any(|profile| source["profile"] == *profile),
                "overlay requires selected profile",
            )?;
            let mut layer = source.clone();
            layer["kind"] = json!("overlay");
            layer["owner"] = json!(pin.repository);
            layers.push(layer);
            content.push_str(&format!("\n# Profile: {id}\n{}", sources.fragments[id]));
        }
        layers.push(json!({"kind":"local", "owner":policy.repository, "sha256":digest(scope.local_additions.as_bytes())}));
        content.push_str(&format!(
            "\n# Repository-owned local additions.\n{}",
            scope.local_additions
        ));
        let mut layer = baseline.clone();
        layer["kind"] = json!("baseline");
        layer["owner"] = json!(pin.repository);
        layers.push(layer);
        content.push_str(&format!(
            "\n# Universal baseline (last in every declared scope).\n{}",
            sources.fragments[string(baseline, "id")?]
        ));
        require(
            file.layers == layers
                && file.content == content
                && file.content_sha256 == digest(content.as_bytes()),
            "composition layers, exact local text, baseline ordering, or output digest mismatch",
        )?;
    }
    Ok(composition)
}
