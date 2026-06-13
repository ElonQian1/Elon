use std::{
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;

use super::{
    model::{CargoIndex, CargoPackageSummary},
    repo_snapshot::relative_path,
};

pub(crate) fn collect_cargo_index(workspace: &Path) -> CargoIndex {
    let Some(manifest) = find_manifest(workspace) else {
        return CargoIndex {
            warnings: vec!["未发现 Cargo.toml，跳过 Cargo workspace 索引。".to_string()],
            ..CargoIndex::default()
        };
    };

    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(&manifest)
        .current_dir(workspace)
        .output();

    let Ok(output) = output else {
        return CargoIndex {
            manifest_path: Some(relative_path(workspace, &manifest)),
            warnings: vec!["cargo metadata 启动失败，跳过 Cargo workspace 索引。".to_string()],
            ..CargoIndex::default()
        };
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return CargoIndex {
            manifest_path: Some(relative_path(workspace, &manifest)),
            warnings: vec![format!(
                "cargo metadata 失败：{}",
                compact(stderr.trim(), 240)
            )],
            ..CargoIndex::default()
        };
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_metadata(workspace, &manifest, &text)
}

fn find_manifest(workspace: &Path) -> Option<PathBuf> {
    let candidates = [
        workspace.join("Cargo.toml"),
        workspace.join("server").join("Cargo.toml"),
    ];
    candidates.into_iter().find(|path| path.is_file())
}

fn parse_metadata(workspace: &Path, manifest: &Path, text: &str) -> CargoIndex {
    let parsed = serde_json::from_str::<Value>(text);
    let Ok(value) = parsed else {
        return CargoIndex {
            manifest_path: Some(relative_path(workspace, manifest)),
            warnings: vec!["cargo metadata 输出不是合法 JSON。".to_string()],
            ..CargoIndex::default()
        };
    };

    let workspace_root = value
        .get("workspace_root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .map(|path| relative_path(workspace, &path));
    let mut packages = Vec::new();
    if let Some(raw_packages) = value.get("packages").and_then(Value::as_array) {
        for package in raw_packages {
            let Some(name) = package.get("name").and_then(Value::as_str) else {
                continue;
            };
            let version = package
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let manifest_path = package
                .get("manifest_path")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .map(|path| relative_path(workspace, &path))
                .unwrap_or_else(|| relative_path(workspace, manifest));
            let mut target_paths = Vec::new();
            let targets = package
                .get("targets")
                .and_then(Value::as_array)
                .map(|targets| {
                    targets
                        .iter()
                        .filter_map(|target| {
                            let name = target.get("name").and_then(Value::as_str)?;
                            if let Some(src_path) = target
                                .get("src_path")
                                .and_then(Value::as_str)
                                .map(PathBuf::from)
                                .map(|path| relative_path(workspace, &path))
                            {
                                target_paths.push(src_path);
                            }
                            let kinds = target
                                .get("kind")
                                .and_then(Value::as_array)
                                .map(|items| {
                                    items
                                        .iter()
                                        .filter_map(Value::as_str)
                                        .collect::<Vec<_>>()
                                        .join("+")
                                })
                                .unwrap_or_default();
                            Some(if kinds.is_empty() {
                                name.to_string()
                            } else {
                                format!("{name}:{kinds}")
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mut features = package
                .get("features")
                .and_then(Value::as_object)
                .map(|features| features.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            features.sort();

            packages.push(CargoPackageSummary {
                name: name.to_string(),
                version,
                manifest_path,
                targets,
                target_paths,
                features,
            });
        }
    }

    CargoIndex {
        manifest_path: Some(relative_path(workspace, manifest)),
        workspace_root,
        packages,
        warnings: Vec::new(),
    }
}

fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_metadata_package_summary() {
        let text = r#"{
          "workspace_root": "C:\\repo\\server",
          "packages": [{
            "name": "demo",
            "version": "0.1.0",
            "manifest_path": "C:\\repo\\server\\Cargo.toml",
            "targets": [{"name": "demo", "kind": ["bin"]}],
            "features": {"default": [], "sqlite": []}
          }]
        }"#;

        let index = parse_metadata(
            Path::new("C:/repo"),
            Path::new("C:/repo/server/Cargo.toml"),
            text,
        );

        assert_eq!(index.packages[0].name, "demo");
        assert!(index.packages[0].targets[0].contains("bin"));
        assert!(index.packages[0].features.contains(&"sqlite".to_string()));
    }
}
