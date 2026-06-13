# 项目 AI 工作入口

本项目由一龙 APK 创建。所有 AI 代理开始任务前，都先读取本文件，再跳转到同一套共享规则。

## 规则权威来源

`.github/copilot-instructions.md` 是本项目唯一的通用规则来源。Codex、Claude、Gemini、Copilot CLI 等工具不要各自维护一份重复规则；如果规则需要调整，优先修改 Copilot 主规则和 `.github/instructions/` 下的专项文档。

## 必读顺序

1. 读 `.github/copilot-instructions.md`。
2. 按任务类型读取 `.github/instructions/*.instructions.md`。
3. 如需了解项目目标、技术栈或常用命令，读取 `docs/project-readme.md`。

## 按需文档

| 任务类型 | 继续读取 |
|---|---|
| Git、提交、分支、发布、回滚 | `.github/instructions/git-workflow.instructions.md` |
| Android、APK、Gradle、移动端构建 | `.github/instructions/android.instructions.md` |
| UI、交互、样式、移动端页面 | `.github/instructions/ui.instructions.md` |
| 后端、API、数据库、鉴权、服务部署 | `.github/instructions/backend.instructions.md` |
| 不确定从哪里开始、任务卡住、跨模块改动 | `.github/instructions/project-workflow.instructions.md` |

## 文件职责

- `AGENTS.md`：所有 AI 工具都能识别的轻量入口。
- `CODEX.md`：Codex 专用桥接文件，只指向 Copilot 主规则。
- `CLAUDE.md`：Claude 专用桥接文件，只指向 Copilot 主规则。
- `GEMINI.md`：Gemini 专用桥接文件，只指向 Copilot 主规则。
- `.github/copilot-instructions.md`：共享规则权威来源。
- `.github/instructions/*.instructions.md`：任务相关的按需规则。

不要在桥接文件里复制大段规则，避免不同 CLI 看到不一致的项目要求。
