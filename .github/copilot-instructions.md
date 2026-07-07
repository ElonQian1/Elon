# 一龙云端开发平台 — AI 代理全局指令

> **所有 AI 代理（Copilot / Codex / Codex CLI / Copilot CLI）的规则权威来源。**
> Copilot 自动加载本文件；Codex/Codex CLI 通过 `AGENTS.md` 路由后手动读取。
> 请在回答任何编码问题前先理解本文件内容。

## 项目定位

**云端APK开发平台**：用户在手机APK上用自然语言和AI对话，描述自己想要的功能；AI 在真实 Git 工作区修改代码，构建产物后上传到服务器部署或分发，最后把新的APK下载链接发回用户手机。用户无需任何编程知识即可定制自己的移动应用。

**讨论驱动的 AI 应用生产**：一龙不是要求用户一次性写清完整需求。用户通过持续讨论把产品目标逐步说清；对不确定性高的新产品需求，可先由低成本“预言家 AI（Demo Oracle）”生成可讨论 demo，再由总调度 AI 自动选择和组合 AI-to-AI Skill，生成 Matter、调度 Worker Bot、完成验收和发布。明确的小改动可以跳过 demo，不能让预演阶段增加不必要成本。Skill 主要面向 AI 调用，普通用户只需要表达目标并在关键节点判断方向。

当前先验证“预言家 demo + 官方 Skill Router + Matter 执行”闭环，不直接开放 Skill 交易市场。完整产品边界和阶段路线见 `docs/ai-to-ai-skill-oracle-roadmap.md`。

**elon APK 的双重身份**：
- **elon 平台客户端**：用户通过 APK 与服务器 AI 对话、管理项目
- **用户子项目开发入口**：用户在 APK 内描述需求，AI CLI 在服务器上修改/构建用户自己的子项目

**开发 elon 自项目的 AI CLI 不限于 Windows**：Copilot（VS Code）在 Windows 开发机上运行；Codex CLI / Claude CLI / Copilot CLI 等在 **Linux 服务器**上运行，同样会修改 elon 自身代码、发布 APK。两个环境都必须能独立完成完整发布链路。


### ⚠️ elon 自项目 vs 用户子项目：构建方式完全不同

**唯一判断规则**：看改的是不是 `elon` 仓库本身（`android/`、`server/`）—— 与用哪个 CLI、在哪台机器无关。

| 场景 | 判断方法 | 正确构建方式 |
|---|---|---|
| **改动 elon 自身**（本仓库任何文件） | 工作目录在 elon 仓库根（含 `scripts/publish-apk.*`） | Windows：`scripts\publish-apk.ps1`；Linux：`bash scripts/publish-apk.sh` → `assembleRelease` + 签名 |
| **给用户构建他们的子项目** | 通过 `build_project()` 工具调用，工作目录在 `/opt/elon/projects/<id>/` | `assembleDebug`（服务器无签名密钥，debug 正确） |

**绝不能**在改动 elon 自身代码后用 `assembleDebug` 或跳过签名脚本直接运行 `./gradlew assembleRelease`。
### Android 任务完成定义

涉及 APK 可安装端能力的任务，PR、分支推送、`assembleDebug` 都不算发布完成；默认分成两层完成定义。用户当前偏好：改动 elon APK/APP 后，代码同步完成后继续发布 APK，除非用户明确说只同步代码、暂不发布或发布稍后再说。并行任务里，代码进入远端主线是本代理的硬完成；发布脚本只负责发布“运行脚本时的最新 main”，如果构建期间被后续提交或服务器版本超越，按脚本提示汇报“代码已合并，发布交给最新主线”，不要反复 rebase 重跑。

1. **代码同步完成**
   - 业务代码已 `commit` 并 `git push origin HEAD:main` 进入 `origin/main`
   - 若用户明确要求“先同步代码”“先合并远端”“发布稍后再说”或“不要求这次发布成功”，到这里即可收尾
   - 可用：
     ```powershell
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed
     ```

2. **APK 发布完成**
   - 改动 elon APK/APP 的用户可见能力时默认继续发布 APK；只有用户明确要求“先同步代码”“发布稍后再说”或“不要求这次发布成功”时才跳过
   - 这时运行：
     ```powershell
     scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
     ```

最终回复要明确区分：本次是“代码已同步”还是“APK 已发布”。  
发布脚本仍然负责版本号申请、构建、上传、并发保护和 finish；但**发布失败或被更新的 main 抢先覆盖，不影响代码已经同步到远端这一事实**。

最终回复必须包含代码提交 SHA、push 状态，以及 APK 发布状态（已发布 / 已被更新 main 超越 / 未尝试发布）。发布成功时再附版本号、发布 SHA、服务器 `/app/version.json` 校验结果和下载地址。

**脚本内置防慢构建覆盖和并发保护**。并发保护触发时不要为了让本代理“发布成功”继续追最新 main；强制覆盖仅用于明确的发布协调任务，参数为 `-Force`。

### Windows PC 节点客户端完成定义

涉及 Win 端节点客户端、启动器、安装/自更新、节点托盘、`elon-pc-node` 二进制或 `scripts/publish-node-agent.ps1` 的用户可见修复，不能只停在代码 push。默认分成两层：

1. **代码同步完成**
   - 业务代码已 `commit` 并 `git push origin HEAD:main` 进入 `origin/main`
   - 可用：
     ```powershell
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed
     ```

2. **Win 节点发布完成**
   - 明确影响用户 Win 端运行体验时，继续运行：
     ```powershell
     scripts\publish-node-agent.ps1
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind NodeAgent
     ```
   - `publish-node-agent.ps1` 会构建 Windows 客户端包，上传 `/api/node-agent/version` 指向的新版本，并默认调用 `/api/admin/nodes/push-update` 通知在线节点自动更新；优先使用本机 `ADMIN_TOKEN` / `ELON_ADMIN_TOKEN`，没有时通过 SSH 在服务器本机读取 `ADMIN_TOKEN` 后调用 `127.0.0.1:8080`，token 不回传。
   - 在线节点收到推送后会更新并自动重连；离线节点或未收到推送的节点会在下次启动/更新检查时读取 `/api/node-agent/version` 自动补上。

最终回复必须明确区分：本次是“代码已同步”还是“Win 节点客户端已发布并推送更新”。

### PC 工作台前端完成定义

涉及 `pc-frontend/`、`/pc`、`/pc-next` 或用户可见 PC 工作台 UI 的修复，不能只停在 `CodePushed`。`/pc` 加载的是服务器 `$DATA_DIR/pc-next-dist/`，代码进入 `origin/main` 不等于用户页面已经更新。

1. **代码同步完成**
   - 业务代码已 `commit` 并 `git push origin HEAD:main` 进入 `origin/main`
   - `pc-frontend` 至少通过 `npm run build`
   - 运行：
     ```powershell
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed
     ```

2. **PC 前端线上完成**
   - 除非用户明确说“只同步代码”“暂不发布”或“不要求这次发布成功”，继续运行：
     ```powershell
     scripts\publish-server.ps1
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind Server
     ```
   - 发布后必须确认 `/pc` 可访问，并确认 `/api/server/version` 的 `gitSha` 等于本次提交或脚本明确提示已被更新主线接管。
   - 截图、遮挡、错位、层级、弹窗或按图修复类问题，完成前必须先把截图区域定位到真实组件/样式文件，并用本地预览、浏览器截图、DOM/坐标/层级检查之一做视觉验收；无法截图时必须说明替代证据，不能只凭构建通过宣称已解决。

最终回复必须明确区分：本次是“代码已同步”还是“PC 前端已发布到线上 / 被最新主线接管 / 未尝试发布”。

> 详细流程见：`docs/ai-agent-workflow.md`

---

## 关键原则（AI 代理必须遵守）

- **每次修改都要 git commit**，commit message 用中文描述用户的需求
- **修改代码前先读懂上下文**，不随意删除已有功能
- **编译失败必须回滚或修复**，不允许将编译错误的代码部署
- **APK 签名密钥不得泄露**，相关操作只走自动化脚本
- **每个用户的修改是隔离的**，不能让一个用户的操作影响其他用户
- **代码变更记录用户身份**，commit 信息中包含用户标识
- **任务开始先跑机器预检**：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`；脚本会先同步本地 `main` 基线，再从最新 `origin/main` 派生独立任务 worktree。只要输出 `WORKTREE_CREATED=true`，必须切过去执行。脚本输出的 `EDIT_ROOT` 是本轮唯一允许编辑、格式化、测试、提交的目录；若为 `BLOCKED_CREATE_WORKTREE_FIRST`，当前目录不能继续改
- **PC 节点 MCP 会话 worktree 例外**：如果当前目录已经位于一龙平台创建的 `conversation-worktrees/<project>/<conversation>`，或当前分支形如 `ai/session/<project>/<conversation>`，说明平台已经完成隔离工作区准备。此时不要再运行 `ai-task-preflight.ps1 -CreateWorktree` 创建嵌套 worktree；直接在当前 worktree 运行 `git status --short --branch`，读取当前任务需要的规则后继续完成用户的直接任务。不能只回复“已读取规则、后续会执行”就结束。
- **PowerShell 版本边界**：Windows `powershell.exe` 5.1 只用于 bootstrap/兼容脚本；任何头部带 `#requires -Version 7.0` 的脚本必须用 `pwsh` 运行。先用 `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-pwsh7.ps1` 检查本机；缺 PowerShell 7 时安装 `winget install --id Microsoft.PowerShell --source winget` 或转到有 `pwsh` 的环境。绝不要删除 `#requires`、降级语法或复制一套低版本逻辑来绕过 PS7 要求；需要 PS5 入口时只写薄 wrapper 或明确命名的 `*-ps5.ps1`，并保留原 PS7 脚本。
- **主工作区只做 main 基线**：`main` checkout 是共享同步基线，不作为业务编辑区。多个本地 AI 并行时，各自只能在预检脚本创建的 `codex/task-*` worktree 中修改、验证、提交和推送，避免互相占用 `main`
- **并行 rebase 只由 push 拒绝触发**：`origin/main` 前进是多 AI 并行的正常现象，不代表当前任务必须 rebase、重跑验证或重新发布。正确流程是：开工前从当时最新 `origin/main` 派生隔离 worktree；提交后立即 `git push origin HEAD:main`；只有 push 被 non-fast-forward 拒绝时才 `git fetch origin` + `git rebase origin/main` 后重推；一旦本任务 HEAD 已包含在 `origin/main`，代码层面即完成。后续 `origin/main` 再前进，不得为了“让本代理保持最新 HEAD / 发布成功”反复 rebase 或重跑构建。
- **预检/worktree 流程改动必须跑门禁**：修改 `scripts/ai-task-preflight.*`、`scripts/cleanup-task-worktrees.*` 或相关并行 AI/Git 工作流说明后，运行 `powershell -ExecutionPolicy Bypass -File scripts\test-ai-task-preflight-workflow.ps1`；该测试会锁住 `-CreateWorktree`、`WORKTREE_PATH` 和 `main` 基线规则
- **有未提交改动时先判断归属**：属于本任务则先在当前隔离 worktree 内完成提交，再按“push 被拒绝才 rebase”的规则处理；来源不明或属于其他任务时必须从 `origin/main` 新建 worktree，不得在脏工作区硬拉远端
- **隔离 worktree 推送后同步主工作区**：回到原主工作区执行 `git fetch origin` + `git pull --ff-only origin main`，只同步已跟踪文件；不 stage、不 stash、不删除/移动未跟踪文件，遇到同名路径冲突就报告
- **任务完成后清理 worktree**：push 并同步主工作区后，运行 `powershell -ExecutionPolicy Bypass -File scripts\cleanup-task-worktrees.ps1 -Apply`（Linux：`bash scripts/cleanup-task-worktrees.sh --apply`）回收已合并的 AI worktree（含 `*-task-*` 与 `codex/*` 分支 worktree）。脚本只删"已合并到 origin/main + 工作树干净"的，绝对安全；带未提交改动的会被自动保留
- **手机触发的开发流程优先让 CLI 自愈**：Git 预检失败不是最终失败，应作为上下文交给 CLI；只有 CLI 判定无法克服时再友好提示用户
- **长期主义模块化**：新建源文件 ≤500 行，超 800 行必须拆分，入口文件只做组装。`scripts/check-source-size.ps1` 和 pre-push hook 会执行增量门禁：历史红区文件允许存在但不得继续变大，新增/改动文件不得跨入红区。详见 `.github/instructions/modular-architecture.instructions.md`
- **PC 前端迁移规则**：PC 工作台新功能默认进入 `pc-frontend/`；旧 `server/src/assets/pc_*.html/js/css` 进入收缩期，只做 bugfix、兼容桥接和迁移删除。涉及 `/pc`、`pc_*`、React/Vite/TypeScript、前端构建或发布链路时，必须读取 `.github/instructions/pc-frontend-migration.instructions.md` 和 `docs/pc-frontend-migration.md`。
- **PC 前端用户可见改动默认要发布**：修改 `/pc` 新前端的用户可见页面、布局、交互、遮挡、登录/账号卡、聊天页或项目工作台时，`CodePushed` 只表示代码同步，不表示用户问题已解决。除非用户明确说“只同步代码/暂不发布”，否则 push 后必须运行 `scripts\publish-server.ps1` 或 `scripts/publish-server.sh` 上传 `$DATA_DIR/pc-next-dist/`，再运行 `scripts\check-task-complete.ps1 -Kind PcFrontend`；最终回复必须区分“代码已同步”和“线上 `/pc` 已发布并校验”。
- **APP UI 设计规范**：任何 APK/APP UI、主题、按钮、卡片、底部导航、状态胶囊或配色调整，必须先读取并遵守 `docs/Design.md` 和 `docs/APP 颜色规范.md`；只有用户明确要求更新设计规范时，才修改这些文件。
- **按图复刻 UI 必须做视觉验收**：用户给截图、红框、手绘稿或要求“按比例复刻”时，AI 完成后必须对照原图检查排版对齐、板块比例、字体层级、间距、触控尺寸和视觉重心；必须通过截图/预览自查，发现按钮偏位、左右/上下不齐、元素比例失真、文字挤压或间距不合理时，先修正再提交/发布，不能把“差不多”的 UI 交付给用户。
- **APK UI ↔ 网页 UI 同步**：改动 APK 任何 layout XML、Toolbar、Tab、气泡、颜色主题时，必须在同一 commit 同步更新 `server/src/assets/web_page.html`。对照规则见 `.github/instructions/apk-web-ui-sync.instructions.md`
- **后端运行代码变更**：先 commit + push 到 `origin/main`，再运行 `.\scripts\publish-server.ps1`；脚本会 POST `/api/release/claim` 让服务器原子分配新版本号，再编译、上传 binary、部署、`/api/release/finish`。版本号通过 `option_env!("ELON_BUILD_VERSION")` 编译期注入，**不再写入 git**。并行任务若发布被后续 main 或服务器版本超越，汇报代码已推送和发布被最新主线接管，不要重复 rebase 重跑。`server/Cargo.toml` 的 version 字段是冷启动兜底，禁止手动递增并提交。发布脚本会屏蔽全局 `target-cpu=native`，强制使用通用 `-C target-cpu=x86-64` 生成服务器可运行产物
- **Android 新功能默认先同步代码到远端主线，再继续发布 APK**；只有用户明确要求只同步代码、暂不发布或发布稍后再说时，才不把发布成功作为完成定义
- **新建文件必须显式 `git add`**：`git add server/src/main.rs` 不会自动包含同目录新建的 `.rs` 文件；提交前必须检查 `git status --short | Select-String "^\?\?"` 确认无遗漏——遗漏新文件会导致其他开发者编译失败
- **Rust 日常验证必须走带锁共享缓存脚本**：不要在同一个 worktree 或共享 target 上并行裸跑 `cargo check` / `cargo test` / `cargo build` / `cargo clippy`，否则多个 AI 会争写 `target` 下的 dep-info / fingerprint 临时文件。Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 check --manifest-path server\Cargo.toml`，Linux/macOS 用 `bash scripts/cargo-dev.sh check --manifest-path server/Cargo.toml`；脚本会读取 `.env.local` / `ELON_DEV_CARGO_TARGET_DIR`，设置开发用 `CARGO_TARGET_DIR` 并加锁串行化写入。发布构建仍使用 `RUST_SERVER_MUSL_TARGET_DIR` / `scripts\publish-server.ps1`，不要混用开发 target 和发布 target。
- **Rust 格式化必须带 crate edition，且纯格式化拆提交**：不要在仓库根裸跑 `rustfmt` 或无 manifest 的 `cargo fmt`；全量检查用 `powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1`（Linux：`bash scripts/format-rust.sh`），全量写入用 `-Apply` / `--apply`。增量格式化指定文件用 `scripts\format-rust.ps1 -Apply -Files <file...>`（Linux：`bash scripts/format-rust.sh --apply --files <file...>`）。脚本会逐个 `Cargo.toml` 或按文件所属 crate 读取显式 `edition`。若确实需要或已经产生全量 Rust 格式化，确认 diff 只有 rustfmt 机械变化后不要回退，必须单独提交为 `style(rust): ...`；业务/文案/逻辑改动另起提交，不得混在同一个 commit。

---

## 🚀 部署速查（服务端改动后必看）

```
改后端代码 → git add（只加 .rs 业务文件）→ git commit → git push origin HEAD:main → check-task-complete -Kind CodePushed → 运行 scripts/publish-server.ps1（脚本自动 claim 版本号 → 编译 → 部署 → finish；被后续 main 超越则停止追车）→ 校验 /api/server/version
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
| Win 节点版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/api/node-agent/version` |
| APK 下载 | `http://43.139.149.158:8080/app/ElonSpeed-latest.apk` |
| Win 节点客户端包 | `http://43.139.149.158:8080/api/node-agent/download/windows-client` |
| 部署脚本 | `scripts/publish-server.ps1`（自动 worktree 隔离，SHA staging，并发安全） |
| Win 节点发布脚本 | `scripts/publish-node-agent.ps1`（上传后默认推送在线节点更新） |
| 服务日志 | `ssh -o ProxyCommand=none root@43.139.149.158 'tail -50 /root/elon-server.log'` |

> ⚠️ **绝对禁止**：改完代码不 commit 直接运行脚本部署——脚本基于 git HEAD，未提交内容不会进入部署。

## VS Code Copilot 工作方式记忆（Copilot 专属）

- 规则体系：全局规则 → 本文件；局部规则 → `.github/instructions/*.instructions.md`；重复任务 → `.github/prompts/*.prompt.md`；角色工作流 → `.github/agents/*.agent.md`。详见 `docs/vscode-copilot-working-model.md`。
- 本项目已有 `/elon-dev-task`、`/elon-apk-release` prompt 和 `elon-planner`、`elon-implementer`、`elon-reviewer` agent，优先使用。

---

## 参考文档（按需读取）

| 文档 | 内容 |
|---|---|
| `.github/instructions/modular-architecture.instructions.md` | 模块化、巨型文件治理、多 AI 并行拆分边界 |
| `.github/instructions/pc-frontend-migration.instructions.md` | PC 工作台从原生静态 HTML/CSS/JS 迁移到 Vite + React + TypeScript 的规则 |
| `.github/instructions/apk-web-ui-sync.instructions.md` | APK UI 改动时必须同步更新网页端的对照规则和检查清单 |
| `docs/pc-frontend-migration.md` | PC 前端迁移路线、模块状态和多 AI 协作约定 |
| `docs/Design.md` | 项目级设计 DNA、AI UI 执行流程、UI Kit/Figma/MCP 协作规则 |
| `docs/APP 颜色规范.md` | APP 暗色 UI 配色 token、按钮、卡片、导航、状态胶囊颜色规范 |
| `docs/system-architecture.md` | 系统架构详细设计、组件交互、数据流 |
| `docs/ai-agent-workflow.md` | AI代理如何执行代码修改→编译→部署的完整流程 |
| `docs/android-setup.md` | Android 新机器首次配置：Gradle 测速、缓存修复、全局镜像 |
| `docs/vscode-copilot-working-model.md` | VS Code Copilot 最新 agent / instructions / prompt files / custom agents 工作方式速记 |
| `AGENTS.md` | 多 AI 工具共享入口和 VS Code 快捷工作流索引 |
| `.github/skills/cloud-apk-dev/SKILL.md` | VS Code 官方 Agent Skills 入口，封装云端 APK 开发/部署流程 |
| `.github/skills/modular-long-term-dev/SKILL.md` | 可复制到其他项目的模块化长期主义 skill，约束 AI 避免巨型文件 |


