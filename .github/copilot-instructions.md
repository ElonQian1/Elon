# 一龙云端开发平台 — AI 代理全局指令

> 本文件被所有 AI 代理自动加载。请在回答任何编码问题前先理解本文件内容。

## 项目定位

**云端APK开发平台**：用户在手机APK上用自然语言和AI对话，描述自己想要的功能；AI 在真实 Git 工作区修改代码，在本地开发机/桌面版 Codex 当前机器构建产物，再把后端 binary / APK 上传到服务器部署或分发，最后把新的APK下载链接发回用户手机。用户无需任何编程知识即可定制自己的移动应用。

---

## 系统组件一览

```
[用户手机 APK]
    │ 自然语言对话
    ▼
[AI 对话后端]  ←─── 理解需求，规划代码修改
    │
    ├─► [Rust 服务端代码]   (server/)
    ├─► [Android APK 代码]  (android/)
    └─► [前端 Web 代码]      (frontend/)
    │
    ▼
[自动化流水线]
    git commit → 编译构建 → 部署上线 → APK 打包签名
    │
    ▼
[推送下载链接] → 用户手机
```

---

## 技术栈

| 层 | 技术 | 说明 |
|---|---|---|
| 移动端 | Android (Kotlin/Java) | 用户使用的 APK，含 AI 对话界面 |
| 服务端 | Rust | 核心业务逻辑、API 接口 |
| 前端 | (待定) | Web 管理界面或 H5 内嵌页 |
| AI 对话 | LLM API | 理解用户需求，生成代码修改方案 |
| CI/CD | 自动化脚本 | git → 编译 → 部署 → APK签名分发 |
| 版本控制 | Git | 所有代码变更走 git 管理 |

---

## 核心工作流（AI 代理必须理解）

1. **需求理解**：用户在APK内用自然语言描述需求
2. **代码定位**：AI确定需要修改哪些文件（Rust/Android/前端）
3. **安全修改**：在当前真实 Git 工作区修改对应代码，保持代码风格一致
4. **提交构建**：`git add → git commit → 触发构建流水线`
5. **编译部署**：Rust 后端在本地开发机交叉编译后上传 binary；Android 在本地开发机打包签名后上传 APK；生产服务器只负责运行和分发产物
6. **反馈用户**：将新APK下载链接通过对话界面发回用户手机

### Android 新功能完成定义

涉及 APK 可安装端能力的任务，PR、分支推送、`assembleDebug` 都不算完成。只要改动触及 `android/app/src/main/**`、`AndroidManifest.xml`、聊天链路、更新链路、权限、后台服务或手机端调试能力，除非用户明确要求“只改代码不发布 APK”，否则必须运行：

```powershell
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

最终回复必须包含 APK 发布状态、版本号、发布 SHA、服务器 `/app/version.json` 校验结果和下载地址。

> 详细流程见：`docs/ai-agent-workflow.md`

---

## VS Code Copilot 工作方式记忆

- 把 VS Code Copilot 理解为 agent loop：先组装上下文，再用工具读取/编辑/运行命令，工具结果回到上下文后继续迭代，最后验证和交付。
- 上下文来自系统指令、customizations、用户消息、会话历史、隐式编辑器/Git 状态、显式 `#` 引用和工具输出；没有进入上下文的内容对模型不可见。
- 项目级稳定规则放在 `.github/copilot-instructions.md`；局部规则放在 `.github/instructions/*.instructions.md`；重复任务放在 `.github/prompts/*.prompt.md`；角色和工具受限的流程放在 `.github/agents/*.agent.md`。
- VS Code 也会识别 `AGENTS.md`、`CLAUDE.md` 和组织级 instructions；管理入口优先使用 `Chat: Open Customizations`，避免多处复制长规则。
- 本项目已提供 `/elon-dev-task`、`/elon-apk-release` prompt，以及 `elon-planner`、`elon-implementer`、`elon-reviewer` agents；优先用这些入口执行标准工作流。
- 本项目同时提供 `.github/skills/cloud-apk-dev/SKILL.md` 作为 VS Code 官方 Agent Skills 入口；用 diagnostics 确认 customization 加载状态。
- 复杂任务先用 Plan/规划思路做 discovery、alignment、design、refinement；计划确认后再进入实现。
- 修改 AI customization 时，保持规则短、自包含、可版本化；需要完整背景时引用 `docs/vscode-copilot-working-model.md`，不要在多个文件重复长规则。

---

## 关键原则（AI 代理必须遵守）

- **每次修改都要 git commit**，commit message 用中文描述用户的需求
- **修改代码前先读懂上下文**，不随意删除已有功能
- **编译失败必须回滚或修复**，不允许将编译错误的代码部署
- **APK 签名密钥不得泄露**，相关操作只走自动化脚本
- **每个用户的修改是隔离的**，不能让一个用户的操作影响其他用户
- **代码变更记录用户身份**，commit 信息中包含用户标识
- **任务开始先跑机器预检**：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`；脚本创建 worktree 时必须切过去执行
- **有未提交改动时先判断归属**：属于本任务可 stash/rebase/pop；来源不明或属于其他任务时必须从 `origin/main` 新建 worktree，不得在脏工作区硬拉远端
- **手机触发的开发流程优先让 CLI 自愈**：Git 预检失败不是最终失败，应作为上下文交给 CLI；只有 CLI 判定无法克服时再友好提示用户
- **后端运行代码变更**：直接运行 `.\scripts\publish-server.ps1`，脚本自动递增 PATCH 版本号、commit、push、构建、部署；部署后校验 `/api/server/version`，APK 会动态展示后端版本；MINOR/MAJOR 变更时手动改好 `server/Cargo.toml` 后加 `-SkipVersionBump` 参数
- **Android 新功能必须发布 APK**，不能只停在 PR、Debug 包或本地验证

---

## 🚀 部署速查（服务端改动后必看）

```
改后端代码 → 递增 server/Cargo.toml version → git add → git commit → git push origin main → 运行 scripts/publish-server.ps1 → 校验 /api/server/version
```

| 项目 | 值 |
|---|---|
| Git 远端 | `git@github.com:ElonQian1/Elon.git` |
| 主分支 | `main` |
| 服务器 SSH | `root@43.139.149.158`（需加 `-o ProxyCommand=none` 绕代理） |
| 服务器端口 | `8080` |
| 健康检查 | `curl --noproxy '*' http://43.139.149.158:8080/health` |
| APK 版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/app/version.json` |
| 后端版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/api/server/version` |
| APK 下载 | `http://43.139.149.158:8080/app/ElonSpeed-latest.apk` |
| 部署脚本 | `scripts/publish-server.ps1`（自动 worktree 隔离，SHA staging，并发安全） |
| 服务日志 | `ssh -o ProxyCommand=none root@43.139.149.158 'tail -50 /root/elon-server.log'` |

> ⚠️ **绝对禁止**：改完代码不 commit 直接运行脚本部署——脚本基于 git HEAD，未提交内容不会进入部署。

## 参考文档（按需读取）

| 文档 | 内容 |
|---|---|
| `docs/system-architecture.md` | 系统架构详细设计、组件交互、数据流 |
| `docs/ai-agent-workflow.md` | AI代理如何执行代码修改→编译→部署的完整流程 |
| `docs/vscode-copilot-working-model.md` | VS Code Copilot 最新 agent / instructions / prompt files / custom agents 工作方式速记 |
| `AGENTS.md` | 多 AI 工具共享入口和 VS Code 快捷工作流索引 |
| `.github/skills/cloud-apk-dev/SKILL.md` | VS Code 官方 Agent Skills 入口，封装云端 APK 开发/部署流程 |

---

## 当前开发状态（2026-05-24 更新）

- [x] 项目整体架构设计
- [x] Rust 服务端基础框架（axum + tokio，运行中）
- [x] Android APK 基础框架（Kotlin + Jetpack Compose）
- [x] 本地交叉编译部署脚本（`scripts/publish-server.ps1`）
- [x] 服务器 systemd 服务（自动重启，日志 `/root/elon-server.log`）
- [x] P2P 同 WiFi APK 中继（`server/src/peer_relay.rs` + Android `PeerSeederManager.kt`）
- [x] APK 分发机制（P2P mirrors + 直链回退）
- [x] Web/APK 模型选择展示已简化为直接显示模型名
- [ ] AI 对话后端集成（待实现）
- [ ] 用户项目隔离系统（待实现）

> AI 代理在修改任何代码时，请先 `git fetch origin main` 和 `git status --short --branch`；工作区干净才 `git pull --rebase origin main`，否则按归属 stash 或使用 `origin/main` 新 worktree。
