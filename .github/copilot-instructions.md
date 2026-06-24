# 一龙云端开发平台 — AI 代理全局指令

> **所有 AI 代理（Copilot / Codex / Codex CLI / Copilot CLI）的规则权威来源。**
> Copilot 自动加载本文件；Codex/Codex CLI 通过 `AGENTS.md` 路由后手动读取。
> 请在回答任何编码问题前先理解本文件内容。

## 项目定位

**云端APK开发平台**：用户在手机APK上用自然语言和AI对话，描述自己想要的功能；AI 在真实 Git 工作区修改代码，构建产物后上传到服务器部署或分发，最后把新的APK下载链接发回用户手机。用户无需任何编程知识即可定制自己的移动应用。

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

涉及 APK 可安装端能力的任务，PR、分支推送、`assembleDebug` 都不算发布完成；默认分成两层完成定义。并行任务里，代码进入远端主线是本代理的硬完成；发布脚本只负责发布“运行脚本时的最新 main”，如果构建期间被后续提交或服务器版本超越，按脚本提示汇报“代码已合并，发布交由后续最新 main”，不要反复 rebase 重跑。

1. **代码同步完成**
   - 业务代码已 `commit` 并 `push origin main`
   - 若用户明确要求“先同步代码”“先合并远端”“发布稍后再说”或“不要求这次发布成功”，到这里即可收尾
   - 可用：
     ```powershell
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed
     ```

2. **APK 发布完成**
   - 只有当用户明确要求“给我可安装 APK / 下载链接 / 立即发布到线上”时，才把发布成功作为完成定义
   - 这时运行：
     ```powershell
     scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
     powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
     ```

最终回复要明确区分：本次是“代码已同步”还是“APK 已发布”。  
发布脚本仍然负责版本号申请、构建、上传、并发保护和 finish；但**发布失败或被更新的 main 抢先覆盖，不影响代码已经同步到远端这一事实**。

最终回复必须包含代码提交 SHA、push 状态，以及 APK 发布状态（已发布 / 已被更新 main 超越 / 未尝试发布）。发布成功时再附版本号、发布 SHA、服务器 `/app/version.json` 校验结果和下载地址。

**脚本内置防慢构建覆盖和并发保护**。并发保护触发时不要为了让本代理“发布成功”继续追最新 main；强制覆盖仅用于明确的发布协调任务，参数为 `-Force`。

> 详细流程见：`docs/ai-agent-workflow.md`

---

## 关键原则（AI 代理必须遵守）

- **每次修改都要 git commit**，commit message 用中文描述用户的需求
- **修改代码前先读懂上下文**，不随意删除已有功能
- **编译失败必须回滚或修复**，不允许将编译错误的代码部署
- **APK 签名密钥不得泄露**，相关操作只走自动化脚本
- **每个用户的修改是隔离的**，不能让一个用户的操作影响其他用户
- **代码变更记录用户身份**，commit 信息中包含用户标识
- **任务开始先跑机器预检**：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`；脚本会先同步本地 `main` 基线，再从最新 `origin/main` 派生独立任务 worktree。只要输出 `WORKTREE_CREATED=true`，必须切过去执行
- **主工作区只做 main 基线**：`main` checkout 是共享同步基线，不作为业务编辑区。多个本地 AI 并行时，各自只能在预检脚本创建的 `codex/task-*` worktree 中修改、验证、提交和推送，避免互相占用 `main`
- **有未提交改动时先判断归属**：属于本任务可 stash/rebase/pop；来源不明或属于其他任务时必须从 `origin/main` 新建 worktree，不得在脏工作区硬拉远端
- **隔离 worktree 推送后同步主工作区**：回到原主工作区执行 `git fetch origin` + `git pull --ff-only origin main`，只同步已跟踪文件；不 stage、不 stash、不删除/移动未跟踪文件，遇到同名路径冲突就报告
- **任务完成后清理 worktree**：push 并同步主工作区后，运行 `powershell -ExecutionPolicy Bypass -File scripts\cleanup-task-worktrees.ps1 -Apply`（Linux：`bash scripts/cleanup-task-worktrees.sh --apply`）回收已合并的 AI worktree（含 `*-task-*` 与 `codex/*` 分支 worktree）。脚本只删"已合并到 origin/main + 工作树干净"的，绝对安全；带未提交改动的会被自动保留
- **手机触发的开发流程优先让 CLI 自愈**：Git 预检失败不是最终失败，应作为上下文交给 CLI；只有 CLI 判定无法克服时再友好提示用户
- **长期主义模块化**：新建源文件 ≤500 行，超 800 行必须拆分，入口文件只做组装。详见 `.github/instructions/modular-architecture.instructions.md`
- **APP 颜色规范**：任何 APK/APP UI、主题、按钮、卡片、底部导航、状态胶囊或配色调整，必须先读取并遵守 `docs/APP 颜色规范.md`；只有用户明确要求更新颜色规范时，才修改该文件。
- **APK UI ↔ 网页 UI 同步**：改动 APK 任何 layout XML、Toolbar、Tab、气泡、颜色主题时，必须在同一 commit 同步更新 `server/src/assets/web_page.html`。对照规则见 `.github/instructions/apk-web-ui-sync.instructions.md`
- **后端运行代码变更**：先 commit + push 到 `origin/main`，再运行 `.\scripts\publish-server.ps1`；脚本会 POST `/api/release/claim` 让服务器原子分配新版本号，再编译、上传 binary、部署、`/api/release/finish`。版本号通过 `option_env!("ELON_BUILD_VERSION")` 编译期注入，**不再写入 git**。并行任务若发布被后续 main 或服务器版本超越，汇报代码已推送和发布被最新主线接管，不要重复 rebase 重跑。`server/Cargo.toml` 的 version 字段是冷启动兜底，禁止手动递增并提交。发布脚本会屏蔽全局 `target-cpu=native`，强制使用通用 `-C target-cpu=x86-64` 生成服务器可运行产物
- **Android 新功能默认先同步代码到远端主线**；只有用户明确要求交付安装包、下载链接或线上 APK 时，才把发布成功作为完成定义
- **新建文件必须显式 `git add`**：`git add server/src/main.rs` 不会自动包含同目录新建的 `.rs` 文件；提交前必须检查 `git status --short | Select-String "^\?\?"` 确认无遗漏——遗漏新文件会导致其他开发者编译失败
- **Rust 格式化必须带 crate edition**：不要在仓库根裸跑 `rustfmt` 或无 manifest 的 `cargo fmt`；全量检查用 `powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1`（Linux：`bash scripts/format-rust.sh`），全量写入用 `-Apply` / `--apply`。增量格式化指定文件用 `scripts\format-rust.ps1 -Apply -Files <file...>`（Linux：`bash scripts/format-rust.sh --apply --files <file...>`）。脚本会逐个 `Cargo.toml` 或按文件所属 crate 读取显式 `edition`。

---

## 🚀 部署速查（服务端改动后必看）

```
改后端代码 → git add（只加 .rs 业务文件）→ git commit → git push origin main → check-task-complete -Kind CodePushed → 运行 scripts/publish-server.ps1（脚本自动 claim 版本号 → 编译 → 部署 → finish；被后续 main 超越则停止追车）→ 校验 /api/server/version
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

## VS Code Copilot 工作方式记忆（Copilot 专属）

- 规则体系：全局规则 → 本文件；局部规则 → `.github/instructions/*.instructions.md`；重复任务 → `.github/prompts/*.prompt.md`；角色工作流 → `.github/agents/*.agent.md`。详见 `docs/vscode-copilot-working-model.md`。
- 本项目已有 `/elon-dev-task`、`/elon-apk-release` prompt 和 `elon-planner`、`elon-implementer`、`elon-reviewer` agent，优先使用。

---

## 参考文档（按需读取）

| 文档 | 内容 |
|---|---|
| `.github/instructions/modular-architecture.instructions.md` | 模块化、巨型文件治理、多 AI 并行拆分边界 |
| `.github/instructions/apk-web-ui-sync.instructions.md` | APK UI 改动时必须同步更新网页端的对照规则和检查清单 |
| `docs/APP 颜色规范.md` | APP 暗色 UI 配色 token、按钮、卡片、导航、状态胶囊颜色规范 |
| `docs/system-architecture.md` | 系统架构详细设计、组件交互、数据流 |
| `docs/ai-agent-workflow.md` | AI代理如何执行代码修改→编译→部署的完整流程 |
| `docs/android-setup.md` | Android 新机器首次配置：Gradle 测速、缓存修复、全局镜像 |
| `docs/vscode-copilot-working-model.md` | VS Code Copilot 最新 agent / instructions / prompt files / custom agents 工作方式速记 |
| `AGENTS.md` | 多 AI 工具共享入口和 VS Code 快捷工作流索引 |
| `.github/skills/cloud-apk-dev/SKILL.md` | VS Code 官方 Agent Skills 入口，封装云端 APK 开发/部署流程 |
| `.github/skills/modular-long-term-dev/SKILL.md` | 可复制到其他项目的模块化长期主义 skill，约束 AI 避免巨型文件 |


