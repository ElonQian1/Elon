//! Default AI instruction documents for newly-created user projects.
use anyhow::Result;
use homecli_proto::ProjectDocumentEntry;
use std::path::Path;

struct DefaultProjectFile {
    path: &'static str,
    title: Option<&'static str>,
    content: &'static str,
}

const DEFAULT_PROJECT_FILES: &[DefaultProjectFile] = &[
    DefaultProjectFile {
        path: "AGENTS.md",
        title: Some("项目 AI 工作入口"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/AGENTS.md"
        )),
    },
    DefaultProjectFile {
        path: ".github/copilot-instructions.md",
        title: Some("Copilot 共享项目指令"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/github/copilot-instructions.md"
        )),
    },
    DefaultProjectFile {
        path: "CODEX.md",
        title: Some("Codex 桥接说明"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/CODEX.md"
        )),
    },
    DefaultProjectFile {
        path: "CLAUDE.md",
        title: Some("Claude 桥接说明"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/CLAUDE.md"
        )),
    },
    DefaultProjectFile {
        path: "GEMINI.md",
        title: Some("Gemini 桥接说明"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/GEMINI.md"
        )),
    },
    DefaultProjectFile {
        path: ".github/instructions/project-workflow.instructions.md",
        title: Some("项目开发流程"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/github/instructions/project-workflow.instructions.md"
        )),
    },
    DefaultProjectFile {
        path: ".github/instructions/git-workflow.instructions.md",
        title: Some("Git 与发布流程"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/github/instructions/git-workflow.instructions.md"
        )),
    },
    DefaultProjectFile {
        path: ".github/instructions/android.instructions.md",
        title: Some("Android 与 APK 任务"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/github/instructions/android.instructions.md"
        )),
    },
    DefaultProjectFile {
        path: ".github/instructions/ui.instructions.md",
        title: Some("UI 与交互任务"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/github/instructions/ui.instructions.md"
        )),
    },
    DefaultProjectFile {
        path: ".github/instructions/backend.instructions.md",
        title: Some("后端与 API 任务"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/github/instructions/backend.instructions.md"
        )),
    },
    DefaultProjectFile {
        path: "docs/project-readme.md",
        title: Some("项目说明"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/docs/project-readme.md"
        )),
    },
    DefaultProjectFile {
        path: ".elon/default-docs.json",
        title: None,
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/elon/default-docs.json"
        )),
    },
];

pub(crate) fn default_project_documents() -> Vec<ProjectDocumentEntry> {
    DEFAULT_PROJECT_FILES
        .iter()
        .filter_map(|doc| doc.title.map(|title| (doc, title)))
        .map(|(doc, title)| ProjectDocumentEntry {
            path: doc.path.to_string(),
            title: title.to_string(),
            content: doc.content.trim().to_string(),
            truncated: false,
            byte_len: doc.content.trim().len() as u64,
        })
        .collect()
}

pub(crate) fn ensure_default_docs_in_workspace(workspace: &Path) -> Result<usize> {
    std::fs::create_dir_all(workspace)?;
    let mut created = 0usize;
    for doc in DEFAULT_PROJECT_FILES {
        let path = workspace.join(doc.path);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("{}\n", doc.content.trim()))?;
        created += 1;
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_docs_seed_missing_files_without_overwriting_user_docs() {
        let root = std::env::temp_dir().join(format!(
            "elon-default-project-docs-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("AGENTS.md"), "# User Rules\nkeep me").unwrap();

        let created = ensure_default_docs_in_workspace(&root).unwrap();
        let agents = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
        let codex = std::fs::read_to_string(root.join("CODEX.md")).unwrap();
        let copilot =
            std::fs::read_to_string(root.join(".github/copilot-instructions.md")).unwrap();
        let metadata = std::fs::read_to_string(root.join(".elon/default-docs.json")).unwrap();
        let _ = std::fs::remove_dir_all(&root);

        assert!(created > 0);
        assert_eq!(agents, "# User Rules\nkeep me");
        assert!(codex.contains(".github/copilot-instructions.md"));
        assert!(copilot.contains("共享规则权威来源"));
        assert!(metadata.contains("copilot-primary-bridged-agents"));
    }
}
