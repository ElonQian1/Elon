# 一龙项目 AI 工作入口

本文件是 VS Code Copilot、Codex、Claude Code 等多 AI 工具共享的工作入口。  
**最后更新：2026-05-26**

---

## 🔒 最高级铁律（任何 AI、任何任务、任何语言都不得违反）

1. **写代码前先输出文件计划**：任何新功能，先用 5-15 行 JSON 说明"新建哪些文件、修改哪些文件、预估行数"，确认每个目标文件行数在预算内，再动手写代码。计划阶段发现冲突，改计划；不要先写代码再发现放不下。
2. **不允许制造新的巨型文件**：新建文件默认目标 ≤500 行；501-800 行可容忍但必须单一职责；超过 800 行必须拆分。入口/组装文件（`main.rs`、`router.rs`、`MainActivity.kt`）更严格，优先控制在 500 行以内。
3. **不允许在已有文件里继续叠加超额逻辑**：目标文件剩余预算不足时，必须先声明新模块接收本次新增逻辑，再动手。
4. **按职责模块化 + 长期主义思考**：先想"这块逻辑属于哪个领域、应该独立成什么模块"，再动手；不要为了"快"就塞回入口文件。
5. **任务起手 + 提交前都要 `git fetch origin main`**：本项目长期多 AI 并发拆分，避免和其他 AI 重复抽同一块代码。
6. **只 stage 本任务文件，不混合提交**：拆分用 `refactor(...)`，新功能用 `feat(...)`/`fix(...)`，不允许"重构 + 新功能"塞同一个 commit。

完整规则见 `.github/instructions/modular-architecture.instructions.md`（已 `applyTo: "**"` 自动加载）。

---

## ⚡ 任务开始前必做（无论哪台 PC）

共享文档里不要写某台 PC 的绝对路径。AI 如果已经在本仓库或任意子目录里启动，直接用当前 Git 仓库根目录；如果不是，先用本机环境变量 `ELON_REPO_PATH` 指向真实仓库。

```powershell
$repo = git rev-parse --show-toplevel 2>$null
if (-not $repo) { $repo = $env:ELON_REPO_PATH }
if (-not $repo) { throw "未找到一龙仓库：请在仓库目录启动，或先设置 ELON_REPO_PATH" }
Set-Location -LiteralPath $repo
powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree
```

如果脚本输出 `WORKTREE_CREATED=true`，必须切到 `WORKTREE_PATH` 继续本次任务；不要在原主工作区改代码。

Linux / macOS / 服务器 Codex CLI：

```bash
repo="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$repo" ]; then
  repo="${ELON_REPO_PATH:-}"
fi
if [ -z "$repo" ]; then
  echo "未找到一龙仓库：请在仓库目录启动，或先设置 ELON_REPO_PATH" >&2
  exit 1
fi
cd "$repo"
bash scripts/ai-task-preflight.sh --create-worktree
```

同步规则：

- 工作区干净：执行 `git pull --rebase origin main` 后开始任务。
- 未提交改动属于本任务：先 `git stash push -u -m "wip-before-sync"`，再 `git pull --rebase origin main`，然后 `git stash pop` 并解决冲突。
- 未提交改动不属于本任务或来源不明：**不要 stash / pull / pop 当前工作区**，必须从 `origin/main` 创建独立 worktree，例如 `git worktree add ..\Elon-task -b codex/task origin/main`。
- 提交前再执行一次 `git fetch origin main` + `git rebase origin/main`，确认本次提交叠在最新远端之上。
- 如果本次是在隔离 worktree 完成并推送到 `main`，任务收尾时应回到原主工作区执行 `git fetch origin` + `git pull --ff-only origin main`，让本地已跟踪文件同步到远端最新；不要 `git add`、不要 stash、不要删除或移动原主工作区的未跟踪文件。若未跟踪文件与远端新增路径冲突导致 fast-forward 失败，停止并报告具体路径。

### 🛡️ 新 clone 必须装一次 git hook（无论 Windows / Linux / 服务器 codex CLI）

```powershell
# Windows
pwsh scripts/install-hooks.ps1
```
```bash
# Linux / macOS / 服务器 codex CLI
bash scripts/install-hooks.sh
```

> 不依赖 AI 是否阅读文档：装好后，`git push` 会被 hook 自动拦截
> ——cargo check 失败或有未 add 的 `.rs` 文件时**直接拒绝 push**。
> 这是机器强制的防漏 add 守门（修复 56bad51 类事故）。
> v0.3.69+ 起，版本号由服务器 `/api/release/claim` 原子分配，不再写入 git，
> 因此 hook 不再检查 `build.gradle` 和 `server/Cargo.toml` 的 version 字段。
> 部署侧由服务器 flock 互斥锁 + CAS（compare-and-swap）保证不会并发覆盖。

---

## 🚀 服务端改动部署流程（铁律：先 git 再 deploy）

```
改代码
  → git status --short | Select-String "^\?\?"  ← ⚠️ 检查是否有新建文件未 add
  → git add <只加自己改的文件，含新建 .rs 文件>
  → git commit -m "type(scope): 描述"
  → git push origin main
  → 根据系统选择部署脚本（脚本会自动从服务器 claim 版本号，编译期注入，不写入 git）：
       Windows:       cd scripts; .\publish-server.ps1
       Linux/macOS:   bash scripts/publish-server.sh
  → 验证健康: curl --noproxy '*' http://43.139.149.158:8080/health
  → 验证后端版本: curl --noproxy '*' http://43.139.149.158:8080/api/server/version
```

> **绝对禁止**：改完代码不 commit 直接运行部署脚本 ——  
> 脚本基于 git HEAD 打包，未提交的代码**不会**进入部署。

> **并发保护**：两个脚本都内置 SHA 顺序检查。如果另一台 PC 已部署更新版本，
> 本机旧版编译完成后**会自动中止**，不会回退服务器版本。用 `-Force`/`--force` 强制覆盖。

---

## 📱 APK 新功能发布闭环（不能只 PR / Debug）

只要任务修改了 APK 用户可安装端能力，就不能停在 PR、分支、`assembleDebug` 或“代码已提交”。

适用范围包括但不限于：

- `android/app/src/main/**`
- `android/app/src/main/AndroidManifest.xml`
- Android 聊天链路、调试能力、更新链路、权限、前台/后台服务
- 任何用户需要在手机上安装后才能验证的新功能或修复

完成定义必须是：

```powershell
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

发布脚本会自动完成：同步 `main`、POST `/api/release/claim` 拿版本号、临时写入 `build.gradle`、构建 release APK、上传 `ElonSpeed-latest.apk` 和 `version.json`、写入 `.apk-deployed-sha`、POST `/api/release/finish` 释放槽位、还原 `build.gradle` 到 git 兜底值。版本号**不写入 git**。

发布脚本必须防止慢构建覆盖新版本：如果构建期间 `origin/main` 已前进，且服务器 `.apk-deployed-sha` 已包含本次基础提交，脚本应中止本地编译/发布并直接让 AI 测试线上新版；如果 `origin/main` 已前进但线上 APK 未确认包含本次基础提交，脚本必须停止上传，要求基于最新 `main` 重新发布。

最终汇报必须写清楚：

- APK 发布状态：已发布 / 未发布（原因）
- APK 版本：`versionName` + `versionCode`
- 发布 commit SHA
- 服务器 `/app/version.json` 校验结果
- 服务器 `/app/version.json` 中的 `gitSha` 是否等于发布 commit
- APK 下载地址

除非用户明确说“只改代码，不发布 APK”，否则 Android 新功能默认必须跑完上面的发布闭环。

---

## 🔑 关键信息速查

| 项目 | 值 |
|---|---|
| Git 远端 | `git@github.com:ElonQian1/Elon.git` |
| 主分支 | `main` |
| 服务器 SSH | `root@43.139.149.158`（加 `-o ProxyCommand=none` 绕代理） |
| 服务器端口 | `8080` |
| 健康检查 | `curl --noproxy '*' http://43.139.149.158:8080/health` |
| APK 版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/app/version.json` |
| 后端版本信息 | `curl --noproxy '*' http://43.139.149.158:8080/api/server/version` |
| APK 下载 | `http://43.139.149.158:8080/app/ElonSpeed-latest.apk` |
| 服务日志 | `ssh -o ProxyCommand=none root@43.139.149.158 'tail -50 /root/elon-server.log'` |

---

## 📂 必读文件顺序

1. `.github/copilot-instructions.md`：项目定位、当前状态、部署速查、全局 AI 原则。
2. `.github/instructions/git-deploy-workflow.instructions.md`：多 AI 并发、worktree 隔离、push 冲突处理。
3. `.github/instructions/modular-architecture.instructions.md`：模块化、巨型文件治理、长期维护边界。
4. `docs/ai-agent-workflow.md`：需求分析→代码修改→编译→部署完整流程。
5. `docs/system-architecture.md`：架构、模块边界、数据流。

---

## 🤖 服务器 Codex CLI 记忆规则

- Codex CLI 不会天然“记住”本项目流程；它每次进入项目时，必须先读取当前仓库里的 `AGENTS.md`、`CODEX.md`、`.github/instructions/*.md` 和相关 `docs/`。
- 本仓库把流程记忆固化在文件里；以后修改 Git、构建、部署、发布规则时，必须同步更新这些说明文件并提交。
- APK 里的一龙项目只是一个默认登记的 `local_path` 项目，和其他本地/GitHub 项目走同一条项目通道，不使用隐藏旁路。
- 任意本地项目都可以通过服务器环境变量 `ELON_PROJECT_<项目ID大写并把非字母数字替换为_>_PATH` 指向真实 Git 工作区。
- `local_path` / GitHub 项目必须是真实 Git 仓库，包含 `.git` 和可用远端；服务器不会为这类项目偷偷 `git init`。

---

## APK 项目并发规则

- 服务器按 `project_id + conversation_id` 设置会话级执行权：不同项目可以同时运行，同一项目的不同会话也可以并行编码。
- 每个开发会话使用独立 Git worktree 和 `ai/session/...` 分支，避免两个手机或两个会话同时 `git pull`、改文件、commit、push 到同一份工作区。
- 同一会话内仍按顺序排队；merge、版本号递增、APK 发布、服务器部署这些共享动作必须保留为串行。无法创建 worktree 的项目退回项目级共享工作区串行执行。
- 一龙自项目不走特殊旁路，它与其他 GitHub/local_path 项目遵守同一套并发、Git、构建、发布规则。

## 后端指挥 Codex CLI 的方式

- 后端负责“能不能做、在哪做、按什么顺序做”：检查项目、Git、权限、队列和锁。
- Codex CLI 负责“怎么改代码”：进入指定项目目录，先读项目文档，再按任务修改、验证、提交。
- 每次 APK 任务都要由后端把通用项目工作流写进 CLI 提示词；不能假设 CLI 自动记得上一轮讨论。
- APK 代码开发请求进入 Codex CLI 前，后端会做低成本源文件体量预检，只把超过 500 行的摘要注入提示词；Codex 必须先按 5-15 行文件计划控制 500/800/1500 行边界，再写代码。
- 以后即使接入其他 AI 模型，它们也只能作为旁路工具做轻量分类、摘要、图片或特殊分析；最终用户消息、旁路结论和后续动作必须整理后回灌到同一 APK 会话绑定的 Codex CLI 原生 session，不能打断 Codex CLI 上下文。
- CLI 返回后，后端继续负责任务状态、下载链接、版本号、发布、部署和并发保护。

---

## APK Codex Session Prewarm

- When the APK opens or resumes a project conversation, it may call `/api/.../prewarm` to create or reuse the native Codex CLI session id for that project/user/conversation.
- Prewarm is not a development task. It must not inspect files, run Git, edit code, build, deploy, publish, or inject the full project workflow.
- The first real user message still decides the route: ordinary chat uses the lightweight Codex prompt, and development requests enter the queued project workflow.
- The server records whether a native Codex session has already received the full chat/development bootstrap. Later turns in the same session should use the shorter resume prompt, and stale/expired native sessions should be marked stale and retried once with a fresh Codex session.
- When retrying with a fresh native session after stale resume, include the previous `codex://threads/<thread_id>` URI plus recent backend conversation messages so the new session can bridge the old context.
- Every traced APK/Web task should expose where time went. Preserve `trace_id` through backend routing into Codex CLI calls and record `codex_cli_start`, `codex_cli_done`, `codex_cli_error`, `codex_cli_retry`, and prewarm hit/skip events with prompt size, session hit/miss, operation, attempt, and elapsed time.

## Build And Attachment Transport Rules

- Desktop Codex work must build locally: `scripts/publish-server.ps1` cross-compiles the Rust server locally and uploads the binary; `scripts/publish-apk.ps1` builds/signs the Android APK locally and uploads the APK plus `version.json`.
- The production server is low-spec. Do not use it as the normal Rust release build machine or Android build machine.
- Shared scripts must not hardcode one PC's drive letter. Machine-specific build paths belong in local environment variables or untracked `.env.local`; for server builds use `ELON_BUILD_TARGET_DIR` when a PC needs a custom Cargo target cache.
- APK project messages with photos/files should upload attachments first through HTTP, then send only server-side attachment references in the chat payload. Avoid putting large base64 blobs into the project WebSocket message.

## 🌐 Android 编译环境首次配置（每台新机器必做）

每台远程开发机网络环境不同，**必须先测速再决定下载方式**，否则 Gradle 构建会因下载卡死。

### 第一步：测速（选择最快的下载路径）

```powershell
# 分别测试官方直连、官方不走代理、腾讯镜像，取 speed 最大的
$cases = @(
  @{Name='official-noproxy';   Url='https://services.gradle.org/distributions/gradle-8.6-bin.zip'; NoProxy=$true},
  @{Name='tencent-mirror';     Url='https://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip';  NoProxy=$true},
  @{Name='official-with-proxy';Url='https://services.gradle.org/distributions/gradle-8.6-bin.zip'; NoProxy=$false}
)
foreach ($c in $cases) {
  Write-Host "=== $($c.Name) ==="
  $a = @('-L','-r','0-10485759','-o','NUL','-s','-w','speed=%{speed_download}B/s total=%{time_total}s code=%{http_code}\n')
  if ($c.NoProxy) { $a += @('--noproxy','*') }
  $a += $c.Url
  & curl.exe @a
}
```

判断标准：
- `speed` 最大（> 3MB/s）且 `code=206` → 使用那条路径
- 官方源 `code=307` 且 speed=0 → 说明最终跳转到 GitHub，国内基本不可用，必须用镜像
- `code=000` → 路由不通

### 第二步：修复 Gradle Wrapper 缓存（如果卡下载）

如果 `~/.gradle/wrapper/dists/gradle-8.6-bin/` 下只有 `.part/.lck` 文件（无完整 zip），说明历史下载中断，必须手动用镜像重新灌入：

```powershell
$d = "$HOME\.gradle\wrapper\dists\gradle-8.6-bin\afr5mpiioh2wthjmwnkmdsd5w"
if (!(Test-Path $d)) { New-Item -ItemType Directory -Path $d | Out-Null }
Remove-Item "$d\*.part","$d\*.lck" -ErrorAction SilentlyContinue
# 按测速结果选择最快的 URL（中国大陆一般是腾讯镜像）
curl.exe -L --noproxy '*' -o "$d\gradle-8.6-bin.zip" "https://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip"
```

### 第三步：配置全局 Gradle 镜像（一次性，永久生效）

```powershell
# 1. 创建 ~/.gradle/init.gradle — 重定向所有 Maven 仓库到阿里云
# 注意：现代 AGP 用 FAIL_ON_PROJECT_REPOS，需用 settingsEvaluated 注入依赖仓库
$initFile = "$HOME\.gradle\init.gradle"
Set-Content $initFile -Encoding UTF8 @'
// buildscript classpath（插件解析）走 allprojects.buildscript
allprojects {
    buildscript {
        repositories {
            maven { url "https://maven.aliyun.com/repository/google" }
            maven { url "https://maven.aliyun.com/repository/central" }
            maven { url "https://maven.aliyun.com/repository/gradle-plugin" }
            maven { url "https://maven.aliyun.com/repository/public" }
        }
    }
}
// 依赖仓库通过 settingsEvaluated 注入，避免与 FAIL_ON_PROJECT_REPOS 冲突
settingsEvaluated { settings ->
    settings.dependencyResolutionManagement {
        repositories {
            maven { url "https://maven.aliyun.com/repository/google" }
            maven { url "https://maven.aliyun.com/repository/central" }
            maven { url "https://maven.aliyun.com/repository/gradle-plugin" }
            maven { url "https://maven.aliyun.com/repository/public" }
        }
    }
}
'@

# 2. 向 ~/.gradle/gradle.properties 添加禁用 JVM 系统代理
# （JVM 会读取系统 SOCKS 代理导致访问超时，必须关闭）
$props = "$HOME\.gradle\gradle.properties"
if (!(Test-Path $props)) { New-Item -ItemType File -Path $props | Out-Null }
$content = Get-Content $props | Where-Object { $_ -notmatch '^systemProp\.' }
$content += "systemProp.java.net.useSystemProxies=false"
Set-Content $props $content -Encoding UTF8
```

### 验证

```powershell
cd e:\lodex\Elon\android
.\gradlew.bat --version --no-daemon   # 应在几秒内输出 Gradle 版本，无下载提示
.\gradlew.bat :app:assembleRelease --no-daemon   # 首次编译会下载插件/依赖，通过阿里云镜像约 2-5 分钟
```

> **为什么不把镜像配置提交到仓库？**
> `init.gradle` 写入用户级 `~/.gradle/`，不进入 git，不影响其他团队成员或 CI 环境。
> 每台机器自行按网络测速决定镜像策略，符合"本地环境自治"原则。

## 本机 Skills 已沉淀规则

- `\\127.0.0.1\skills\ai-git-deploy-workflow` 的核心规则已固化到本文件和 `.github/instructions/git-deploy-workflow.instructions.md`：preflight、worktree 隔离、只 stage 本任务文件、commit/push 后再 deploy、部署后 live 验证。
- `\\127.0.0.1\skills\rust-shared-target-cache` 的关键事故规则：禁止相对路径 `CARGO_TARGET_DIR`；Rust 构建优先使用本仓库脚本，裸跑 cargo 前怀疑环境异常时先检查用户/系统环境变量；多 PC 路径差异用未提交的 `.env.local` / `ELON_BUILD_TARGET_DIR` 解决，不把某台机器盘符写进共享脚本。
- `\\127.0.0.1\skills\p2p-app-distribution` 的可复用规则：APK 分发以公网 `/app/version.json` 为事实来源，P2P mirrors 只能作为加速，必须保留 `downloadUrl` 服务器直链兜底；修改 mirror 优先级、PeerSeeder、relay 或更新链路后属于 Android 可安装端能力变更，必须跑 APK 发布闭环。

## VS Code 快捷入口

- 常规代码任务：运行 `/elon-dev-task`。
- APK 发布任务：运行 `/elon-apk-release`。
- 只做规划：选择 `elon-planner` agent。
- 执行实现：选择 `elon-implementer` agent。
- 提交前审查：选择 `elon-reviewer` agent。
- 可移植技能入口：启用 Agent Skills 后使用 `cloud-apk-dev` skill。
- 跨项目模块化治理：把 `.github/skills/modular-long-term-dev` 复制到其他项目，或在支持 Agent Skills 的工具里启用 `modular-long-term-dev` skill，防止 AI 继续制造巨型文件。

## VS Code Customization 检查

- 用 `Chat: Open Customizations` 打开 Agent Customizations editor。
- 用 diagnostics 视图确认 instructions、prompts、agents、skills 都已加载且没有 frontmatter 错误。
- 如果只打开了 `android/`、`server/` 等子目录，启用 `chat.useCustomizationsInParentRepositories`，让 VS Code 发现仓库根目录 `.github` 配置。
- Agent Skills 需要 `github.copilot.chat.skillTool.enabled`；本项目官方路径是 `.github/skills/cloud-apk-dev/SKILL.md`。
- Hooks 目前是 Preview；没有明确需求前，不在仓库启用会自动执行命令的 hooks。

---

## 工作原则

- 有未提交并发改动时，用临时 worktree 隔离（见 git-deploy-workflow）。
- 隔离 worktree 推送成功后，尽量把原主工作区用 `git pull --ff-only origin main` 同步到最新，同时保留未跟踪/未提交现场不动。
- 避免继续制造巨型文件；新增源文件默认目标 ≤500 行，501-800 行可容忍但必须单一职责，超过 800 行必须拆分；入口文件只做组装和路由。在红区文件（超过角色上限）中追加 ≥30 行新逻辑时，优先顺手抽到独立模块。
- 只 stage 当前任务文件，不夹带其他 AI 的改动。
- 每次任务必须 commit + push，部署必须基于已推送的 SHA。
- 后端运行代码变更只需直接运行 `.\scripts\publish-server.ps1`：脚本会 POST `/api/release/claim` 让服务器原子分配新版本号，通过 `ELON_BUILD_VERSION` 环境变量编译期注入 binary，部署成功后 POST `/api/release/finish`。版本号**不写入 git**，`server/Cargo.toml` 的 version 是冷启动兜底，禁止手动递增并提交。部署脚本会把 git SHA 注入 `/api/server/version`，APK 会动态展示该后端版本。
- Android 新功能必须完成 APK 发布闭环，不能只停在 PR、Debug 包或本地验证。
- 不提交密钥、`.env`、APK 签名材料或任何敏感信息。
- push 被拒绝（non-fast-forward）→ `git pull --rebase` 解决后再 push，禁止 `--force`。
