use super::{
    relevance::RelevantFile, repo_snapshot::RepoSnapshot, rust_project::RustProjectSummary,
};

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ValidationPlan {
    pub(crate) commands: Vec<ValidationCommand>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ValidationCommand {
    pub(crate) command: String,
    pub(crate) reason: String,
    pub(crate) required: bool,
}

pub(crate) fn build_validation_plan(
    snapshot: &RepoSnapshot,
    rust_project: Option<&RustProjectSummary>,
    relevant_files: &[RelevantFile],
) -> ValidationPlan {
    let mut commands = Vec::new();
    let mut notes = vec![
        "这份验证计划是预检建议，不替代真实构建/测试结果。".to_string(),
        "执行修改前先读取真实文件确认上下文事实。".to_string(),
    ];

    let touches_rust =
        relevant_files.iter().any(|file| file.path.ends_with(".rs")) || rust_project.is_some();
    if touches_rust {
        commands.push(ValidationCommand {
            command: "rustfmt <changed-rs-files>".to_string(),
            reason: "Rust 改动只格式化本次变更文件，避免污染历史格式。".to_string(),
            required: true,
        });
        if let Some(manifest) = preferred_cargo_manifest(snapshot, rust_project, relevant_files) {
            commands.push(ValidationCommand {
                command: format!("cargo check --manifest-path {manifest} --all-targets"),
                reason: "覆盖目标 crate 的主程序、测试和示例编译边界。".to_string(),
                required: true,
            });
            if relevant_files.iter().any(|file| file.role == "test") {
                commands.push(ValidationCommand {
                    command: format!("cargo test --manifest-path {manifest}"),
                    reason: "相关文件包含测试代码，行为改动后应跑对应测试。".to_string(),
                    required: false,
                });
            }
        } else {
            notes.push("检测到 Rust 文件，但未找到 Cargo.toml；需要人工确认验证命令。".to_string());
        }
    }

    if relevant_files.iter().any(|file| {
        file.path.ends_with(".kt")
            || file.path.ends_with(".java")
            || file.path.ends_with(".gradle")
            || file.path.ends_with(".kts")
            || file.path.starts_with("android/")
    }) {
        notes.push(
            "涉及 Android/APK 时，代码同步和 APK 发布是两层完成定义；发布只走项目脚本。"
                .to_string(),
        );
    }

    if snapshot.git_dirty {
        notes.push(
            "当前工作区存在未提交改动；下游 agent 修改前需要确认归属，避免覆盖并行工作。"
                .to_string(),
        );
    }

    ValidationPlan { commands, notes }
}

impl ValidationPlan {
    pub(crate) fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Validation Plan\n\n");
        if self.commands.is_empty() {
            out.push_str("No deterministic validation command was inferred. Read project docs before changing files.\n\n");
        } else {
            out.push_str("## Commands\n\n");
            for command in &self.commands {
                let required = if command.required {
                    "required"
                } else {
                    "recommended"
                };
                out.push_str(&format!(
                    "- `{}` ({}) - {}\n",
                    command.command, required, command.reason
                ));
            }
            out.push('\n');
        }
        if !self.notes.is_empty() {
            out.push_str("## Notes\n\n");
            for note in &self.notes {
                out.push_str(&format!("- {note}\n"));
            }
        }
        out
    }
}

fn preferred_cargo_manifest(
    snapshot: &RepoSnapshot,
    rust_project: Option<&RustProjectSummary>,
    relevant_files: &[RelevantFile],
) -> Option<String> {
    if snapshot.manifests.iter().any(|path| path == "Cargo.toml") {
        return Some("Cargo.toml".to_string());
    }

    let manifests = rust_project.map(|project| project.manifests.as_slice())?;
    for file in relevant_files {
        let mut best = None;
        for manifest in manifests {
            let crate_root = manifest.path.trim_end_matches("/Cargo.toml");
            if !crate_root.is_empty() && file.path.starts_with(crate_root) {
                best = Some(manifest.path.clone());
            }
        }
        if best.is_some() {
            return best;
        }
    }
    manifests.first().map(|manifest| manifest.path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_compiler::rust_project::RustManifestSummary;

    #[test]
    fn recommends_cargo_check_for_relevant_rust_crate() {
        let snapshot = RepoSnapshot {
            git_head: Some("abc123".to_string()),
            git_branch: Some("main".to_string()),
            git_dirty: false,
            git_status_short: Vec::new(),
            has_origin: true,
            top_level_entries: Vec::new(),
            instruction_docs: Vec::new(),
            manifests: Vec::new(),
            large_files: Vec::new(),
            source_file_count: 1,
        };
        let rust = RustProjectSummary {
            root_package: None,
            workspace: true,
            workspace_members: vec!["server".to_string()],
            manifests: vec![RustManifestSummary {
                path: "server/Cargo.toml".to_string(),
                package_name: Some("elon-server".to_string()),
                workspace: false,
            }],
            toolchain: None,
        };
        let relevant = vec![RelevantFile {
            path: "server/src/context_compiler/mod.rs".to_string(),
            score: 10,
            lines: 90,
            role: "source",
            reasons: Vec::new(),
            matches: Vec::new(),
        }];

        let plan = build_validation_plan(&snapshot, Some(&rust), &relevant);

        assert!(plan
            .commands
            .iter()
            .any(|item| item.command
                == "cargo check --manifest-path server/Cargo.toml --all-targets"));
    }
}
