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
        path: "AI_PROJECT.md",
        title: Some("AI 项目定位"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/AI_PROJECT.md"
        )),
    },
    DefaultProjectFile {
        path: "AI_ARCHITECTURE.md",
        title: Some("AI 架构索引"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/AI_ARCHITECTURE.md"
        )),
    },
    DefaultProjectFile {
        path: "AI_INDEX.md",
        title: Some("AI 代码索引"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/AI_INDEX.md"
        )),
    },
    DefaultProjectFile {
        path: "AI_RULES.md",
        title: Some("AI 规则桥接"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/AI_RULES.md"
        )),
    },
    DefaultProjectFile {
        path: "AI_TASK_TEMPLATE.md",
        title: Some("AI 任务模板"),
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/AI_TASK_TEMPLATE.md"
        )),
    },
    DefaultProjectFile {
        path: ".aiignore",
        title: None,
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../default-project-docs/files/.aiignore"
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
            source: "platform_default".to_string(),
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
#[path = "project_default_docs_tests.rs"]
mod tests;
