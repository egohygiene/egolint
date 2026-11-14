//! ASCII path-prefix collisions through actual Git trees and the public CLI.

// These intentionally conflicting physical paths need the qualified Linux
// case-sensitive checkout. Host-independent inventory controls cover semantics.
#![cfg(target_os = "linux")]

use std::path::Path;
use std::process::Command;

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn native_ascii_topology_findings_are_stable_for_real_git_trees() {
    let cases: &[(&[&str], usize)] = &[
        (&["Readme", "README/page.md"], 1),
        (&["README", "Readme/page.md"], 1),
        (&["src/A/a.md", "src/a/b.md"], 1),
        (&["src/A/x/File.md", "src/a/x/file.md"], 2),
        (&["Docs/a.md", "docs/a.md"], 1),
        (&["Readme.md", "README.md"], 1),
        (&["src/a.md", "src/b.md"], 0),
        (&["Ä/a.md", "ä/b.md"], 0),
    ];
    for (paths, count) in cases {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), &["init", "--quiet", "--template="]);
        std::fs::create_dir_all(root.path().join(".git/info")).unwrap();
        std::fs::write(root.path().join(".git/info/exclude"), ".reports/\n").unwrap();
        for path in *paths {
            let file = root.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, b"fixture\n").unwrap();
        }
        git(root.path(), &["add", "--all"]);
        let tree = git(root.path(), &["write-tree"]);
        let mut previous = None;
        for _ in 0..2 {
            let output = Command::new(env!("CARGO_BIN_EXE_egolint"))
                .env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap())
                .env("CI", "true")
                .arg("--workspace")
                .arg(root.path())
                .arg("validate")
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(i32::from(*count > 0)),
                "{paths:?}: {output:?}"
            );
            let report: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.path().join(".reports/egolint/run.json")).unwrap(),
            )
            .unwrap();
            let findings = report["findings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|finding| finding["rule"]["rule_id"] == "EGO-PORT-CASE-001")
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(findings.len(), *count, "{paths:?}: {report}");
            for finding in &findings {
                assert!(paths.contains(&finding["location"]["path"].as_str().unwrap()));
            }
            if let Some(expected) = &previous {
                assert_eq!(&findings, expected);
            }
            previous = Some(findings);
            assert_eq!(tree, git(root.path(), &["write-tree"]));
        }
    }
}
