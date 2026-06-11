//! Default AI instruction documents for newly-created user projects.
use anyhow::Result;
use homecli_proto::ProjectDocumentEntry;
use std::path::Path;

struct DefaultProjectDoc {
    path: &'static str,
    title: &'static str,
    content: &'static str,
}

const DEFAULT_PROJECT_DOCS: &[DefaultProjectDoc] = &[
    DefaultProjectDoc {
        path: "AGENTS.md",
        title: "项目 AI 工作入口",
        content: r#"# 项目 AI 工作入口

本项目由一龙 APK 创建和维护。AI 代理开始任何开发任务前，必须先读取本文件，再按任务需要读取其它项目文档。

## 基本规则

- 先确认当前项目目录、Git 状态、用户需求和可验证的完成标准。
- 修改代码前先理解现有结构，不把无关项目的规则套到本项目。
- 优先做小而完整的改动：实现、验证、提交，并说明结果。
- 不覆盖用户已有文件；发现冲突、脏工作区或缺失依赖时，先诊断再处理。
- 需要构建或发布时，使用项目内已有脚本和文档，不手搓发布流程。

## 必读顺序

1. `AGENTS.md`
2. `CODEX.md`
3. `.github/copilot-instructions.md`
4. 与任务相关的 `.github/instructions/*.md` 或 `docs/*.md`
"#,
    },
    DefaultProjectDoc {
        path: "CODEX.md",
        title: "Codex 执行说明",
        content: r#"# Codex 执行说明

Codex 在本项目中负责把用户的自然语言需求落成可验证的代码改动。

## 工作方式

- 先读项目文档和现有源码，再决定实现位置。
- 倾向复用项目已有框架、脚本、目录结构和命名风格。
- 新功能应有清楚边界，避免把大型逻辑堆进入口文件。
- 完成后运行最小但有效的验证命令，例如编译、测试或页面检查。
- 每次有意义的代码改动都应提交到 Git；提交信息说明用户可见变化。

## 输出要求

最终说明要包含：做了什么、验证结果、代码是否已提交/推送、是否已发布或未发布。
"#,
    },
    DefaultProjectDoc {
        path: ".github/copilot-instructions.md",
        title: "Copilot 共享指令",
        content: r#"# Copilot 共享指令

本文件给 Copilot、Codex 和其它 AI 代理提供共享项目规则。

## 项目约定

- 以用户目标为准，优先交付能运行、能验证的结果。
- 修改前先搜索相关文件，确认真实调用链。
- 不删除或回滚来源不明的用户改动。
- UI 改动要关注移动端可读性、触控尺寸和状态反馈。
- 后端改动要关注鉴权、数据隔离、错误信息和可观测性。

## 文档维护

如果项目后续出现更具体的构建、发布、设计或业务规则，应补充到 `.github/instructions/` 或 `docs/`，并在 `AGENTS.md` 中路由。
"#,
    },
    DefaultProjectDoc {
        path: ".github/instructions/project-workflow.instructions.md",
        title: "项目开发流程",
        content: r#"# 项目开发流程

## 开始任务

1. 查看 Git 状态和当前分支。
2. 阅读 `AGENTS.md`、`CODEX.md` 和任务相关文档。
3. 找到真实入口、数据模型和调用链。

## 实现任务

- 小步修改，保持模块职责清楚。
- 数据结构优先使用现有模型和 API。
- 对用户可见行为补足错误态、空态和加载态。
- 高风险改动先加测试或用现有验证命令覆盖。

## 完成任务

1. 运行必要验证。
2. 提交代码。
3. 如果有远端，按项目规则推送。
4. 汇报验证命令和发布状态。
"#,
    },
    DefaultProjectDoc {
        path: "docs/project-readme.md",
        title: "项目说明模板",
        content: r#"# 项目说明模板

这里记录本项目给人和 AI 共同阅读的说明。

## 项目目标

请在这里补充：这个项目解决什么问题，主要用户是谁，最重要的使用场景是什么。

## 技术栈

请在这里补充：前端、后端、移动端、数据库、构建工具和部署方式。

## 常用命令

请在这里补充：安装依赖、运行开发环境、测试、构建、发布等命令。

## 注意事项

请在这里补充：权限、密钥、环境变量、第三方服务、兼容性和禁止操作。
"#,
    },
];

pub(crate) fn default_project_documents() -> Vec<ProjectDocumentEntry> {
    DEFAULT_PROJECT_DOCS
        .iter()
        .map(|doc| ProjectDocumentEntry {
            path: doc.path.to_string(),
            title: doc.title.to_string(),
            content: doc.content.trim().to_string(),
            truncated: false,
            byte_len: doc.content.trim().len() as u64,
        })
        .collect()
}

pub(crate) fn ensure_default_docs_in_workspace(workspace: &Path) -> Result<usize> {
    std::fs::create_dir_all(workspace)?;
    let mut created = 0usize;
    for doc in DEFAULT_PROJECT_DOCS {
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
        let _ = std::fs::remove_dir_all(&root);

        assert!(created > 0);
        assert_eq!(agents, "# User Rules\nkeep me");
    }
}
