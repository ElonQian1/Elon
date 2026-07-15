---
applyTo: "**/*.md"
---

# 项目文档权威性与低 Token 检索

本规则供 Codex、Claude、Gemini、Copilot 及其它 AI 代理共同使用。任务涉及文档、笔记整理、需求追溯、规则冲突或知识检索时才读取。

## 路径权威上限

| 路径或类型 | 权威性 | 默认检索 |
|---|---|---|
| `AGENTS.md` | 所有 AI 的共享入口与路由 | 是 |
| `.github/copilot-instructions.md` | 仓库通用规则权威来源 | 是 |
| `.github/instructions/*.instructions.md` | 领域规则 | 任务命中时 |
| 当前规范、架构、需求、运行手册 | 对所属领域有效 | 按需 |
| `decisions/`、ADR | 已接受决定及原因 | 按需 |
| `drafts/`、`inbox/`、讨论、想法 | 未批准材料 | 否 |
| `reports/`、测试报告、交付证据 | 只能证明结果，不能定义需求 | 否 |
| `archive/`、`historical/`、旧讨论稿 | 历史材料 | 否 |
| 无法判断的 Markdown | `unclassified` | 否 |

正文或 frontmatter 可以把自身降为 `draft`、`deprecated`、`superseded` 或 `archived`，但不能突破路径权威上限。归档文件不能通过自称“权威”恢复为当前规则。

## 检索与冲突

1. 先由程序读取路径、标题、大小、哈希、标题层级和生命周期，不把全文送给模型。
2. 只加载当前任务命中的必须文档、领域指令和源码入口。
3. 只有 `ambiguous` 或冲突文档才按单篇读取；先目录，仍不足再读全文。
4. 草稿、报告、归档和未知文档默认排除；历史追溯时显式标注身份。
5. 冲突优先级：用户当前要求 > 通用规则 > 领域规则 > 当前规范/架构/需求 > 已接受决策 > 草稿/报告/历史材料。
6. 同级冲突报告 `DOC_CONFLICT`，不要静默任选一份。

AI 首轮只根据路径和元数据形成分类、冲突和迁移建议，不自动提升权威性；无法确认的内容留在 `unclassified`。报告实际读取的文档数、估算 token 和默认排除范围。

## 分区和 AI 建议

- `.elon/document-sections.json` 保存项目共享的虚拟分区和文档归类，不改变文件实际路径。
- `.elon/document-organization-suggestions.json` 保存 AI 的结构化整理建议；AI 整理任务只可写这一份建议文件。
- 发起整理任务前不得在基线工作区预创建建议占位文件；建议 JSON 只能由隔离 AI 任务产出并进入正常 Git 收尾。
- 默认 `git_backed_full`：先提交整理前原始文档，再自动应用新分区、虚拟归类及结构化建议中的项目内 Markdown 重命名/移动，最后提交整理结果。
- 用户可切换 `review_all`（逐项审核）或 `suggestions_only`（只生成建议）；所有 AI 供应商使用同名模式。
- `review_all` 才要求用户逐项核对并授予本次 `rename`/`move`；`suggestions_only` 禁止应用。
- 实体整理只允许项目内 Markdown，必须校验源文件哈希，禁止覆盖、删除、越界、改写代码或自动 push。当前分类操作不改正文；正文修改、引用重写、归档和删除未来也必须纳入同一整理前/后 Git 事务。
