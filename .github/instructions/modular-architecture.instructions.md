---
applyTo: "**"
---

# 一龙项目 — 模块化与长期维护规则

> AI 代理编辑任何文件前，本规则自动生效。目标是避免继续制造巨型文件，让多 AI 并行开发时边界清楚、冲突更少、后续维护更稳。

## 核心原则

- 新功能默认按职责建模块，不把 UI、状态、网络、存储、Git、发布、诊断、prompt、解析等不同职责继续塞进同一个文件。
- 入口文件只负责组装和路由；业务逻辑、数据结构、协议解析、持久化、UI 构造、后台任务和外部命令执行应放到独立模块。
- 修改已有巨型文件时，优先把本次触碰到的成块职责抽到新文件；不要在 1500 行以上的文件里继续追加大段新逻辑。
- 拆分应保持行为不变，先搬迁再改功能；每次提交聚焦一个边界，避免“重构 + 新功能 + 文案 + 发布脚本”混在一起。
- 多 AI 并行时按模块分工，避免两个代理同时编辑同一个巨型文件；开始前先 `git fetch origin main` 并查看远端是否已有相同拆分。
- 新建模块必须显式 `git add`，并同步更新 `mod`/import/路由注册/测试入口，避免只提交引用文件漏掉新文件。
- 对 Rust 模块，优先使用 `server/src/<domain>/` 或明确命名的 sibling module；对 Android Kotlin，优先按 feature/helper/service 拆文件，不把所有行为留在 `MainActivity.kt`。
- 对用户项目也遵守相同长期主义：如果项目没有自己的模块边界说明，先按最小职责边界拆小，不要生成新的巨型入口文件。

## 判断阈值

- 单文件超过 800 行：新增逻辑前先考虑是否已有更合适模块。
- 单文件超过 1500 行：除小修外，新增功能应优先抽模块。
- 单函数超过 120 行或同时处理三类以上职责：优先拆成小函数或独立 helper。
- 一次改动预计超过 5 个文件或跨多个职责：先拆成多个提交或多个任务。

## 本仓库重点治理对象

- `android/app/src/main/kotlin/com/elon/app/MainActivity.kt`：只保留 Activity 生命周期、顶层导航和模块组装；输入框、附件、会话列表、项目工作流、CLI 输出清洗、证据展示、账号/版本等职责应继续下沉。
- `android/app/src/main/kotlin/com/elon/app/McpDebugServer.kt`：HTTP/MCP 协议、工具注册、诊断工具、任务控制、网络探测、JSON 组装、鉴权应拆分。
- `server/src/project_api.rs`：HTTP handlers、WebSocket job、附件、Git/worktree、APK 分发、部署 key、项目状态应按领域拆分。
- `server/src/ai_cli.rs`：prompt 构建、CLI 进程执行、stream parser、native session/prewarm、intent gate、环境检查应保持分模块。
