// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

//! Offline presentation integrity through the actual native CLI.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const PACKAGE: &str = "assets/identity/repository-presentation.json";
const MANIFEST: &str = "assets/identity/repository-presentation-manifest.json";
const REVISION: &str = "1111111111111111111111111111111111111111";
const PROFILE_REVISION: &str = "cb2ed63425d29abada2d2bbb43a3b3e59d11aeb8";
const PROFILE_DIGEST: &str = "44e0881519350e6747723995939c79c6fb4659e38a74b2c32e409866e7a186ba";
const ASSETS: [&str; 6] = [
    "banner-light.svg",
    "banner-light.png",
    "banner-dark.svg",
    "banner-dark.png",
    "badge.svg",
    "badge.png",
];

fn write(root: &Path, path: &str, content: impl AsRef<[u8]>) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn write_json(root: &Path, path: &str, value: &Value) {
    write(root, path, serde_json::to_vec_pretty(value).unwrap());
}

fn read_json(root: &Path, path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root.join(path)).unwrap()).unwrap()
}

fn checksum(bytes: &[u8]) -> Value {
    json!({ "sha256": format!("{:x}", Sha256::digest(bytes)), "bytes": bytes.len(), "mediaType": "application/octet-stream" })
}

fn bind_package(root: &Path) {
    let mut manifest = read_json(root, MANIFEST);
    manifest["files"]["repository-presentation.json"] =
        checksum(&std::fs::read(root.join(PACKAGE)).unwrap());
    write_json(root, MANIFEST, &manifest);
}

fn consumer(mode: &str) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new("git")
        .args(["init", "--quiet"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let profile: Value = serde_json::from_str(include_str!(
        "fixtures/repository-presentation/hygiene-profile.v1.json.fixture"
    ))
    .unwrap();
    write(
        root.path(),
        "vendor/hygiene/repository-presentation-profile.json",
        include_bytes!("fixtures/repository-presentation/hygiene-profile.v1.json.fixture"),
    );
    write(
        root.path(),
        "policy.toml",
        include_str!("../examples/repository-presentation.toml")
            .replace("mode = \"advisory\"", &format!("mode = \"{mode}\"")),
    );
    let mut requirements = BTreeMap::new();
    for slot in profile["slots"].as_array().unwrap() {
        requirements.insert(
            slot["id"].as_str().unwrap().to_owned(),
            slot["default_requirement"].as_str().unwrap().to_owned(),
        );
    }
    for (group, axis) in [
        ("type_overrides", "tool"),
        ("visibility_overrides", "public"),
        ("lifecycle_overrides", "incubating"),
    ] {
        if let Some(overrides) = profile[group][axis].as_object() {
            for (slot, requirement) in overrides {
                requirements.insert(slot.clone(), requirement.as_str().unwrap().to_owned());
            }
        }
    }
    let mut readme = String::from(
        "# Synthetic repository\n\n<!-- repository-presentation:begin owner=egohygiene/identity profile=1.0.0-alpha.1 -->\n![Example banner](assets/identity/banner-light.svg)\n![Hygienic](assets/identity/badge.svg)\n<!-- repository-presentation:end -->\n",
    );
    for slot in requirements.keys() {
        readme.push_str(&format!("\n<!-- repository-presentation:slot {slot} -->\n"));
    }
    write(root.path(), "README.md", readme);
    let evidence = json!({
        "schema": "egohygiene.repository-presentation-evidence/v1",
        "profile": { "id": "egohygiene.repository-presentation-profile/v1", "version": "1.0.0-alpha.1", "status": "proposed" },
        "repository": { "name": "egohygiene/example", "type": "tool", "visibility": "public", "lifecycle": "incubating", "represented_commit": REVISION },
        "assessment": { "state": "passing" },
        "badge": { "label": "Hygienic", "state": "passing", "message": "repository profile passing", "profile_version": "1.0.0-alpha.1", "represented_commit": REVISION, "evidence_url": "evidence/repository-presentation.json" },
        "slots": requirements.iter().map(|(id, requirement)| json!({ "id": id, "requirement": requirement, "state": "passing", "evidence": [{ "freshness": "current" }] })).collect::<Vec<_>>()
    });
    write_json(
        root.path(),
        "evidence/repository-presentation.json",
        &evidence,
    );
    let package = json!({
        "schema": "identity.repository-presentation-package/v1", "version": "1.0.0",
        "project": { "id": "example", "displayName": "Example", "repository": "https://github.com/egohygiene/example", "visibility": "public" },
        "source": {},
        "profile": { "id": "egohygiene.repository-presentation-profile/v1", "version": "1.0.0-alpha.1", "status": "proposed", "commit": PROFILE_REVISION, "sha256": PROFILE_DIGEST },
        "banner": { "altText": "Example banner", "fallbackText": "Example", "variants": [
            { "svg": "banner-light.svg", "raster": "banner-light.png" },
            { "svg": "banner-dark.svg", "raster": "banner-dark.png" }
        ] },
        "badge": { "label": "Hygienic", "state": "passing", "profileVersion": "1.0.0-alpha.1", "representedCommit": REVISION, "evidenceUrl": "evidence/repository-presentation.json", "svg": "badge.svg", "raster": "badge.png" },
        "consumerBoundary": { "editsReadme": false, "evaluatesEvidence": false, "networkRequired": false, "generatedRegionsOnly": true }
    });
    write_json(root.path(), PACKAGE, &package);
    let mut files = serde_json::Map::new();
    for asset in ASSETS {
        let bytes = format!("synthetic bytes for {asset}");
        write(root.path(), &format!("assets/identity/{asset}"), &bytes);
        files.insert(asset.to_owned(), checksum(bytes.as_bytes()));
    }
    write_json(
        root.path(),
        MANIFEST,
        &json!({
            "schema": "identity.repository-presentation-package-manifest/v1", "version": "1.0.0", "projectionSchema": "identity.repository-presentation-package/v1",
            "projectId": "example", "sourceDigest": "0".repeat(64),
            "profile": { "id": "egohygiene.repository-presentation-profile/v1", "version": "1.0.0-alpha.1", "status": "proposed", "commit": PROFILE_REVISION, "digest": PROFILE_DIGEST },
            "evidence": { "state": "passing", "representedCommit": REVISION, "url": "evidence/repository-presentation.json", "digest": "0".repeat(64) }, "files": files
        }),
    );
    bind_package(root.path());
    root
}

fn validate(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_egolint"))
        .arg("--workspace")
        .arg(root)
        .args([
            "validate",
            "--repository-presentation",
            "policy.toml",
            "--represented-commit",
            REVISION,
        ])
        .output()
        .unwrap()
}

fn focused(root: &Path) -> Value {
    read_json(root, ".reports/egolint/repository-presentation.json")
}

fn assert_invalid(root: &Path, mode: &str) {
    let before = std::fs::read(root.join("README.md")).unwrap();
    let output = validate(root);
    assert_eq!(
        output.status.code(),
        Some(i32::from(mode == "blocking")),
        "{output:?}"
    );
    let report = focused(root);
    assert_eq!(report["status"], "invalid", "{report:#}");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["rule_id"] == "EGO-PRESENT-MANIFEST-001")
    );
    if mode == "advisory" {
        assert_eq!(report["summary"]["blocking_diagnostics"], 0);
    }
    assert_eq!(std::fs::read(root.join("README.md")).unwrap(), before);
}

#[test]
fn complete_map_passes_and_damaged_file_is_rejected() {
    for mode in ["advisory", "blocking"] {
        let root = consumer(mode);
        let output = validate(root.path());
        assert!(output.status.success(), "{output:?}");
        assert_eq!(focused(root.path())["status"], "valid");
        assert_eq!(focused(root.path())["summary"]["manifest_files_checked"], 7);
        write(root.path(), "assets/identity/banner-light.svg", "damaged");
        assert_invalid(root.path(), mode);
    }
}

#[test]
fn every_referenced_asset_and_package_descriptor_needs_checksum_coverage() {
    for asset in ASSETS.into_iter().chain(["repository-presentation.json"]) {
        for mode in ["advisory", "blocking"] {
            let root = consumer(mode);
            let mut manifest = read_json(root.path(), MANIFEST);
            manifest["files"].as_object_mut().unwrap().remove(asset);
            write_json(root.path(), MANIFEST, &manifest);
            assert_invalid(root.path(), mode);
        }
    }
}

#[test]
fn empty_checksum_map_is_invalid() {
    let root = consumer("advisory");
    let mut manifest = read_json(root.path(), MANIFEST);
    manifest["files"] = json!({});
    write_json(root.path(), MANIFEST, &manifest);
    assert_invalid(root.path(), "advisory");
}

#[test]
fn required_package_descriptors_cannot_be_removed_or_replaced() {
    for descriptor in [
        "source",
        "banner",
        "badge",
        "project",
        "profile",
        "consumerBoundary",
    ] {
        for replacement in [None, Some(Value::Null), Some(json!([]))] {
            let root = consumer("advisory");
            let mut package = read_json(root.path(), PACKAGE);
            if let Some(value) = replacement {
                package[descriptor] = value;
            } else {
                package.as_object_mut().unwrap().remove(descriptor);
            }
            write_json(root.path(), PACKAGE, &package);
            bind_package(root.path());
            assert_invalid(root.path(), "advisory");
        }
    }
}

#[test]
fn malformed_manifest_metadata_is_invalid() {
    for mutation in [
        "empty-media-type",
        "missing-media-type",
        "unknown-field",
        "unknown-root",
        "null-files",
    ] {
        let root = consumer("advisory");
        let mut manifest = read_json(root.path(), MANIFEST);
        match mutation {
            "empty-media-type" => manifest["files"]["badge.svg"]["mediaType"] = json!(""),
            "missing-media-type" => {
                manifest["files"]["badge.svg"]
                    .as_object_mut()
                    .unwrap()
                    .remove("mediaType");
            }
            "unknown-field" => manifest["files"]["badge.svg"]["extra"] = json!(true),
            "unknown-root" => manifest["extra"] = json!(true),
            "null-files" => manifest["files"] = Value::Null,
            _ => unreachable!(),
        }
        write_json(root.path(), MANIFEST, &manifest);
        assert_invalid(root.path(), "advisory");
    }
}
