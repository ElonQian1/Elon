---
applyTo: "**"
---

# 一龙项目 — Git + 部署强制工作流

> AI 代理编辑任何文件前，本规则自动生效。无需主动加载。

---

## ⚡ 核心铁律

**任何代码修改，必须严格按以下顺序执行，一步不得跳过：**

```
修改代码
  → git add（仅自己的文件）
  → git commit -m "type(scope): 描述"
  → git push origin main
  → 临时工作树构建 / 部署
  → 验证
  → 清理工作树
  → 汇报 SHA
```

---

## 🌐 访问服务器必须绕过代理

> **本机可能运行了 VPN/代理客户端，会导致访问服务器的 SSH/curl 超时！**  
> 根本原因：VPN 客户端设置了系统代理，SSH/curl 连接被路由到代理后超时。  
> 服务器内部发起的命令不受影响，只有**从本机发起的 SSH/SCP/curl 到外网 IP** 才需要绕代理。

```powershell
# ✅ SSH / SCP：加 -o ProxyCommand=none
ssh -o ProxyCommand=none root@43.139.149.158 'command'
scp -o ProxyCommand=none <file> root@43.139.149.158:/path/

# ✅ curl.exe（PowerShell）：加 --noproxy '*'
curl.exe --noproxy '*' -s "http://43.139.149.158:8080/health"

# ✅ Invoke-RestMethod（PowerShell）：加 -NoProxy
Invoke-RestMethod "http://43.139.149.158:8080/api/server/version" -NoProxy

# ✅ 服务器内部 SSH 执行 curl：不需要特殊处理
ssh -o ProxyCommand=none root@43.139.149.158 'curl -s http://127.0.0.1:8080/health'
```

---

## ❌ 绝对禁止

| 禁止行为 | 原因 |
|---------|------|
| 修改代码后直接 build/rsync，跳过 commit/push | 半成品代码上线，无法追溯，无备份 |
| 在主工作区执行部署构建（rsync/scp） | 可能把其他 AI 未提交的改动一并同步到服务器 |
| 只 commit 不 push | 本地磁盘故障 = 工作成果全部丢失 |
| 在主工作区执行 `git reset --hard`、`git checkout --` | 会覆盖其他并发 AI 的未提交改动 |
| 夹带无关文件进同一次 commit | 污染提交历史，妨碍定位问题 |
| 编译完成后跳过"服务器是否已有更新版本"的祖先检查，强推上传 | 用旧编译覆盖别人刚发布的新版本，手机端版本倒退 |
| 在共享脚本里写死某台机器的本机盘符（如 `E:\rust-target\...`） | 其他 PC / 远程 Codex 没有这个盘，脚本秒退；本机差异必须走 `.env.local` + `RUST_SERVER_MUSL_TARGET_DIR` / `ELON_BUILD_TARGET_DIR`（详见下方 "🤝 共享脚本不绑死本机路径" 章节） |

---

## 🔁 编译/上传/部署的并发模型（核心理解）

> 多台 PC、多个 AI 同时改这个仓库时，最大风险**不是 git 冲突**（push 拒绝可以 rebase 修），而是**编译耗时窗口内，别人已经发布了更新版本，本机的慢编译再上传会让线上倒退**。

### 我们采用的模型：先 git 同步 → 编译 → 上传前再次祖先检查 → 中止或继续

```
T0  git pull --rebase origin main        ← 起点同步，基于最新 main 编译
T1  本地编译（耗时几分钟到几十分钟）       ← 窗口期，别人可能抢先发布
T2  ── 上传前祖先检查 ──
       读取服务器当前 SHA（后端）或 versionCode（APK）
       ├─ 服务器是本次的祖先 / 服务器版本号 < 本次 → ✅ 继续上传
       └─ 服务器更新 / 服务器版本号 ≥ 本次       → ⏹  中止上传（exit 0）
T3  flock 锁内 CAS 再次校验               ← 最后一道防线（仅后端）
T4  替换产物 + 重启 / 写 version.json
T5  HTTP 健康检查
```

**中止时**：git push 的提交保留不回退；本次编译产物作废；直接验证线上版本；如需强制覆盖加 `-Force`。

### 各脚本当前实现

| 阶段 | 后端 `publish-server.ps1` | APK `publish-apk.ps1` |
|---|---|---|
| 起点同步 | ✅ `git pull --rebase` | ✅ `git pull --rebase` |
| 上传前祖先检查 | ✅ `git merge-base --is-ancestor $serverSha $localSha` | ✅ 读 `/app/version.json`，比较 `versionCode` |
| 锁内 CAS 二次校验 | ✅ `flock + .deployed-sha` | ⚠️ 暂无（依赖前面的祖先检查 + 顺序上传） |
| 强制覆盖参数 | `-Force` | `-Force` |
| 中止时的退出码 | `0`（友好退出，不视为失败） | `0`（友好退出，不视为失败） |

> 看到脚本输出 `部署已中止：服务器版本更新` 或 `APK 发布已中止：服务器已有更新版本` 是**正常的并发保护**，不是失败。下一步应该直接验证线上版本，而不是用 `-Force` 覆盖。

---

## ✅ 提交规则

1. **任务开始前**：先运行机器预检脚本，Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`
2. 如果预检脚本输出 `WORKTREE_CREATED=true`，必须切换到 `WORKTREE_PATH` 后再编辑文件；原工作区只保留现场，不继续叠加新改动
3. 任务开始 **前后** 各执行一次 `git status --short`，识别当前工作区是否有其他 AI 的未提交改动
4. `git add` 只加自己任务相关的文件
5. 发现其他代理未提交改动 → **不回退、不覆盖**，回到预检脚本创建的 worktree 继续
6. 提交前再执行 `git fetch origin main` + `git rebase origin/main`，确保本次提交基于最新远端
7. **每次 commit 后必须立即 `git push origin main`**
8. 如果本次是在隔离 worktree 中提交并推送，收尾时回到原主工作区执行 `git fetch origin` + `git pull --ff-only origin main`，让本地已跟踪文件追上远端；不 `git add`、不 stash、不删除/移动未跟踪文件。若本地未跟踪文件与远端新增同名路径冲突，停止并报告路径。

### 本地有未提交改动时如何同步远端

| 场景 | 做法 |
|---|---|
| 工作区干净 | `git pull --rebase origin main` |
| 未提交改动确定属于本任务 | `git stash push -u -m "wip-before-sync"` → `git pull --rebase origin main` → `git stash pop` → 解决冲突 |
| 未提交改动属于其他 AI / 其他任务 / 来源不明 | 不在当前工作区 pull/rebase；从 `origin/main` 创建独立 worktree |

独立 worktree 示例：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree
# 如果输出 WORKTREE_CREATED=true：
Set-Location "<WORKTREE_PATH>"
```

```bash
bash scripts/ai-task-preflight.sh --create-worktree
# 如果输出 WORKTREE_CREATED=true：
cd "<WORKTREE_PATH>"
```

> 未提交改动不会让本地“自动落后”；落后是因为远端后来进了新提交。未提交改动的问题是会让 `pull --rebase` 不安全，所以必须先判断归属。

### 隔离 worktree 推送后的原主工作区同步

隔离 worktree 是为了保护原主工作区现场，不代表原主工作区应该长期落后。任务 worktree 成功 push 到 `main` 后，若能访问原主工作区，应执行：

```powershell
Set-Location "<原主工作区>"
git fetch origin
git pull --ff-only origin main
git status --short --branch
```

这个同步只更新 Git 已跟踪文件到远端最新提交；不要借机 `git add`、stash、清理、删除或移动未跟踪文件。
如果 `ff-only` 因为本地未跟踪文件会被远端同名文件覆盖而失败，说明需要人工判断该文件归属；此时停止并向用户报告冲突路径。

### ⚠️ 新建文件必须显式 `git add`（血泪教训）

`git add server/src/main.rs` **不会自动包含同目录下新建的 `.rs` 文件**。  
新建文件是 `untracked` 状态，必须单独加：

```powershell
# 提交前必须执行，检查是否有未跟踪的新文件
git status --short | Select-String "^\?\?"
# 有输出 → 有新建文件，逐一确认是否需要 git add
```

新建 Rust 模块文件时，标准操作：
```powershell
git add server/src/<new-file>.rs   # 新文件必须显式添加
git add server/src/main.rs         # 以及引用它的文件
git add server/Cargo.toml          # 仅当有新增依赖时；禁止用于递增 version 字段
```

**反例（导致构建断裂）**：
```powershell
# ❌ 只 add 了修改的文件，漏掉了新建的 homecli_agent.rs
git add server/src/main.rs   # main.rs 里有 mod homecli_agent;
git commit                   # 提交后，homecli_agent.rs 不在仓库里
# → 其他开发者 pull 后构建：error[E0583]: file not found for module `homecli_agent`
```

> 分支名是 `main`（不是 `master`）。

### local_path / GitHub 项目

- `local_path` 和 GitHub 导入项目必须是真实 Git 仓库，不能在非 Git 目录里临时 `git init` 伪装成项目。
- 一龙项目只是一个默认 `local_path` 项目；执行流程和其他 GitHub/本地项目相同。
- 如果服务器工作区缺少 `.git` 或远端不可用，先修复项目挂载/clone/SSH 权限，再继续修改。

---

## 🔁 push 被拒绝时的处理（non-fast-forward）

> **适用场景**：多台电脑或多个 AI 并行工作，另一方先推送，本次 `git push` 报错。
> AI 代理遇到此错误必须按以下步骤自动处理，不得放弃任务或要求人工介入。

```powershell
# 1. 拉取远程最新，把本地 commit 叠加在对方之上
git fetch origin
git rebase origin/main

# 2a. 无冲突 → 直接推送
git push origin main

# 2b. 有冲突 → 逐文件解决后继续
# 解决冲突（编辑文件，保留正确内容）
git add <冲突文件>
git rebase --continue
git push origin main
```

**rebase 冲突解决原则**：
- 自己改的逻辑 + 对方改的逻辑 **都保留**（除非明确互斥）
- 不允许用 `git rebase --abort` 丢弃自己的修改
- 解决后在汇报中注明："遇到推送冲突，已 rebase 解决，最终 SHA：xxx"

---

## 🔀 多 AI 并发规则

| `git status` 结果 | 做法 |
|---|---|
| 干净（无改动） | ✅ 主工作区直接改代码 |
| 只有本 AI 自己改的文件 | ✅ 主工作区继续，提交时只 add 自己的文件 |
| 有**其他 AI 未提交**的改动 | ⚠️ 必须用独立工作树，不得在主工作区改代码 |

### 有其他 AI 未提交改动时：独立工作树隔离

```powershell
# 1. 基于 origin/main 创建本会话专属工作树
$id = Get-Random -Maximum 9999
git fetch origin main
git worktree add ..\Elon-session-$id -b codex/session-$id origin/main

# 2. 在会话工作树中改代码
Set-Location ..\Elon-session-$id
# ... 修改文件 ...
git add <自己的文件>
git commit -m "feat(scope): 描述"

# 3. 回到主仓库 cherry-pick 集成到 main
  $repoRoot = git rev-parse --show-toplevel
  Set-Location $repoRoot
git cherry-pick <session_commit_sha>
git push origin main

# 4. 清理会话工作树
git worktree remove ..\Elon-session-$id --force
```

---

## 🦀 Rust 代码格式化规则（增量自律）

> **方针**：不重构历史代码、只对新改动增量规范。改 `.rs` 文件后，**只对自己改过的文件**跑 `rustfmt`。

```powershell
# ✅ 一条命令格式化所有本次改动的 .rs 文件（修改 + 新增全覆盖）
$rs = @(git diff --name-only) + @(git ls-files --others --exclude-standard) |
  Where-Object { $_ -match '\.rs$' }
if ($rs) { rustfmt $rs }
```

**禁止**：
- `cargo fmt`（无参数）：会扫描整个 crate 数百个历史文件，产生大量无关 diff，污染 PR 历史
- 修改其他 AI 负责的 `.rs` 文件的格式

> `rustfmt <files>` 只格式化指定文件，几百毫秒完成，不触发重编译。

---

## � 共享脚本不绑死本机路径（远程协作铁律）

> 多台 PC / 远程 Codex 容器共用同一个仓库，**任何共享脚本里都不能写死某台机器的本地盘符或用户名**，否则其他 PC 跑同一个脚本会因路径不存在直接失败。

### 强制规则

| 共享脚本默认值 | 是否允许 |
|---|---|
| 可移植路径（`%LOCALAPPDATA%\Elon\...`、`<repo>/.x`、`$XDG_CACHE_HOME/...`） | ✅ |
| 从环境变量 / `.env.local` 读取的值 | ✅ |
| 写死 `E:\`、`D:\rust\shared\...` 等具体盘符 | ❌ |
| 写死 `C:\Users\Alice\...` 等用户名 | ❌ |
| 相对路径（会随 cwd 漂移，回到鬼影 target 事故） | ❌ |

### 本机差异的标准解决方案

**1. 仓库根放未提交的 `.env.local`**（已在 `.gitignore`），里面写本机绝对路径：

```bash
# Windows
RUST_SERVER_MUSL_TARGET_DIR=D:\rust\shared\server-musl-target

# Linux/macOS
RUST_SERVER_MUSL_TARGET_DIR=/var/tmp/server-musl-target

# Legacy per-project root is still supported:
# ELON_BUILD_TARGET_DIR=D:\rust\shared\target
```

**2. 或临时通过进程环境变量覆盖**（CI / 一次性测试最方便）：

```powershell
$env:RUST_SERVER_MUSL_TARGET_DIR = "D:\rust\shared\server-musl-target"
.\scripts\publish-server.ps1
```

**3. 仓库根提供 `.env.local.example`**（可提交，纯文档），告诉新人有哪些可配置项。

### 脚本侧实现要点

- 优先级：进程环境变量 > `.env.local` > 可移植默认值
- 启动期校验：路径必须是绝对路径；Windows 上盘符必须存在；否则给出明确报错引导改 `.env.local`
- 参考实现：`scripts/publish-server.ps1` 的 `Import-LocalEnvFile` + `Resolve-ServerMuslTargetDir` / `Resolve-BuildTargetRoot` 函数

### 适用范围

不只是 Cargo target 缓存。**任何"每台机器值不同但脚本需要用"的配置**都走这套模式：

- APK 签名 keystore 路径
- 本机 IDE/编译器缓存目录
- 本地调试用的 mock 服务器地址
- SSH 别名 / 跳板机配置

通用准则：
> **共享脚本里只允许两种值：可移植的默认值，或从 `.env.local` / 环境变量读取的值。绝不允许某台机器的本地绝对路径作为默认值。**

---

## �🦀 Rust target 与构建缓存安全

本机 skills 中的 `rust-shared-target-cache` 已确认一个高风险事故：相对路径的 `CARGO_TARGET_DIR` 会随 cwd 展开到不同位置，导致多个鬼影 `target/` 目录、磁盘暴涨，以及构建产物找错路径。

执行本项目 Rust 构建时遵守：

- 优先使用仓库脚本：`scripts\publish-server.ps1` / `scripts\publish-server.sh`。脚本会设置明确的构建输出目录，不依赖用户级相对路径。
- 多台 PC 的盘符、缓存盘、权限策略不同，不能把某台机器的绝对路径写进共享脚本。需要本机固定构建缓存时，在仓库根创建未提交的 `.env.local` 或设置进程环境变量：

```powershell
RUST_SERVER_MUSL_TARGET_DIR=D:\rust\shared\server-musl-target
```

`scripts\publish-server.ps1` / `scripts\publish-server.sh` 优先读取 `RUST_SERVER_MUSL_TARGET_DIR`，并把它作为精确 `CARGO_TARGET_DIR`，便于一台机器上的多个 Rust 后端共享 `x86_64-unknown-linux-musl` 编译产物；旧名 `RUST_MUSL_TARGET_DIR` 仍兼容。旧的 `ELON_BUILD_TARGET_DIR` 也仍兼容：脚本会在其下创建 **固定名子目录 `elon-server-musl/`**（不含 SHA）；这些变量都未设置时 Windows 使用 `%LOCALAPPDATA%\Elon\build-target\elon-server-musl\`，Linux/macOS 使用 `~/.cache/elon/build/elon-server-musl/`（XDG 标准缓存路径）。这些缓存都**跨 session 持久化**，支持增量编译。禁止手动把 `CARGO_TARGET_DIR` 设为含 SHA 的路径，那样会让每次构建都变成全量重编。

- 裸跑 `cargo check`、`cargo build`、`cargo zigbuild` 前，如果构建行为异常或机器是新环境，先检查 `CARGO_TARGET_DIR`：

```powershell
[System.Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "User")
[System.Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "Machine")
```

- 如果任何输出包含 `..`、`.` 开头或其他相对路径，当前任务不要继续裸跑构建；临时清掉当前进程变量并改用绝对路径，后续再修正用户级配置：

```powershell
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
$env:CARGO_TARGET_DIR = "D:\rust\shared\target\manual-check"
```

- 新 PC 的长期配置应写到 `%USERPROFILE%\.cargo\config.toml`，并使用绝对路径：

```toml
[build]
target-dir = "E:/rust-target"
```

- 不要把相对路径 `CARGO_TARGET_DIR` 写入用户环境变量、系统环境变量、脚本或文档示例。

---

## 🦀 后端部署（Rust → Linux 服务器）

**服务器信息**：`root@43.139.149.158`，项目路径 `/root/Elon`

### 标准流程：本地交叉编译，只上传 binary

```powershell
# 1. 提交业务代码（不再需要手动改 server/Cargo.toml 的 version）
git add server/src/<file>.rs          # 只加自己改的文件
git commit -m "fix(server): 描述"
git push origin main

# 2. 在当前执行机本地交叉编译并上传产物
cd scripts
.\publish-server.ps1

# 3. 验证
curl --noproxy '*' http://43.139.149.158:8080/health
curl --noproxy '*' http://43.139.149.158:8080/api/server/version
```

Linux/macOS 开发机使用等价入口：

```bash
bash scripts/publish-server.sh
curl --noproxy '*' http://43.139.149.158:8080/health
curl --noproxy '*' http://43.139.149.158:8080/api/server/version
```


### 仅本地验证 binary（不部署）

```powershell
cd scripts
.\publish-server.ps1 -SkipUpload
```

```bash
bash scripts/publish-server.sh --skip-upload
```

旧入口 `scripts/deploy.sh` 只保留为兼容包装器，内部会转到 `scripts/publish-server.sh`，不得再实现远端编译。

### 后端版本号规则（v0.3.69+ 新流程：服务器分配，不再进 git）

**版本号由服务器在 `/api/release/claim` 原子分配，`server/Cargo.toml` 里的 `version` 字段只是冷启动兜底，不再被任何脚本或 git 提交修改。**

`scripts/publish-server.ps1` / `publish-server.sh` 自动执行：

1. `git pull --rebase origin main`
2. POST `/api/release/claim` → 服务器返回 `assignedVersionName` 和 `token`
3. 设置环境变量 `ELON_BUILD_VERSION=<assignedVersionName>`，`cargo zigbuild` 通过 `option_env!("ELON_BUILD_VERSION")` 把版本号编进 binary
4. 上传 binary → flock + CAS 替换 → 重启
5. POST `/api/release/finish`（success=true/false）释放或确认 in-flight 槽位

并发多 builder 时，服务器维护 in-flight 列表，每个 builder 拿到不同 patch，互不冲突。

| 情况 | 做法 |
|---|---|
| 修复 Bug / 新增后端功能 | 直接运行 `.\scripts\publish-server.ps1`（默认 bump=patch） |
| MINOR / MAJOR 版本切换 | 由人工修改服务器侧 `LaneState.last_published_version_name`（运维操作） |
| 旧参数 `-SkipVersionBump` | 已废弃，留作向后兼容；新流程下没有 git 版本号变动，本参数无效果 |

```powershell
# 一条命令完成：claim → 编译 → 部署 → finish
.\scripts\publish-server.ps1
```

脚本会在构建时注入 git SHA + 服务器分配的版本号，`/api/server/version` 返回 `versionName` 和 `gitSha`。`server/Cargo.toml` 的 version 字段被 `option_env!` 覆盖，permanent 不变。

---

## 📱 Android APK 部署

> Android 新功能不是“代码合并即完成”。只要改了 APK 可安装端能力，必须跑发布脚本并校验服务器版本。

### APK 发布两步式工作流（v0.3.69+ 新流程：版本号不进 git）

```
[第1步：提交业务代码]   git add 业务文件 → commit → push origin main
        ↓
[第2步：运行发布脚本]   publish-apk.ps1 -Changelog "..."
        （脚本自动：claim 拿版本号 → 临时注入 build.gradle → 编译 → 上传
         → finish 释放槽位 → 还原 build.gradle 到 git 兜底版本）
```

**与旧流程的核心区别**：
- 不再有"第3步：提交版本号变动"。`android/app/build.gradle` 里的 `versionCode/versionName` 永远是冷启动兜底，不会被脚本提交。
- 多 PC 并发发布时不会撞 versionCode：服务器 `/api/release/claim` 原子分配不同的 patch 号给不同 builder。

```powershell
# 第1步：先提交业务代码
git add android/app/src/main/...   # 只加业务文件，不加 build.gradle
git commit -m "feat(android): 描述"
git push origin main               # 脚本基于远端 HEAD，必须先 push

# 第2步：运行发布脚本（脚本会自动 claim 版本号、编译、上传、finish）
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

### ⚠️ APK 慢构建防覆盖规则

`scripts\publish-apk.ps1` 必须把 APK 发布视为基于明确 git SHA + 服务器 token 的原子动作：

- 构建开始前 claim 拿到 token + version。
- 构建期间定期检查 `origin/main` 和服务器 `.apk-deployed-sha`；如果线上 APK 已部署了包含基础 SHA 的更新提交，立即中止本地旧编译并 finish(success=false)，改为测试线上新版。
- 构建完成后再次检查 `origin/main` 与 `version.json`；服务器已有同等或更高 versionCode 时，停止上传并 finish(success=false)。
- 上传 APK 和 `version.json` 必须先传 staging 文件，再由服务器 `flock` + `.apk-deployed-sha` CAS 原子替换；慢构建不得覆盖已经上线的后代 SHA。
- `version.json` 必须包含本次源码 commit 的 `gitSha`（即 `BuildBaseSha`，没有新增 release commit），`check-task-complete.ps1 -Kind AndroidFeature` 校验线上 `gitSha` 等于当前 main HEAD。
- 任何失败路径（claim 后 abort、编译失败、CAS 冲突、scp 失败）必须调 `/api/release/finish` 的 success=false 以释放 in-flight 槽位。

如果用户明确要求"只改代码，不发布 APK"，最终汇报必须写明 `APK 发布状态：未发布（用户明确要求）`。

---

## 🏷️ 版本号管理规则（v0.3.69+ 服务器分配）

> **版本号不再由 git 管理**。`android/app/build.gradle` 和 `server/Cargo.toml` 里的版本号是冷启动兜底值，所有 AI 代理都**不得手动递增并提交**这些字段。

| 项目 | 字段位置 | 实际来源 |
|------|---------|---------|
| 后端 `versionName` | `server/Cargo.toml` (兜底) | `option_env!("ELON_BUILD_VERSION")` ← `/api/release/claim` |
| 后端 `gitSha` | 编译时注入 | 本机 `git rev-parse HEAD` |
| APK `versionCode` | `build.gradle` (兜底) | publish-apk.ps1 临时写入 ← `/api/release/claim` |
| APK `versionName` | `build.gradle` (兜底) | publish-apk.ps1 临时写入 ← `/api/release/claim` |
| APK `gitSha` (version.json) | 上传 json | 本机 `BuildBaseSha` |

**AI 代理执行规则**：

```powershell
# ✅ 正确：直接运行发布脚本，版本号由服务器分配
.\scripts\publish-apk.ps1 -Changelog "<改动描述>"
.\scripts\publish-server.ps1

# ❌ 禁止：手动递增 build.gradle 的 versionCode 并 commit
# ❌ 禁止：手动递增 server/Cargo.toml 的 version 并 commit
```

**禁止**：
- 手动递增 `versionCode` / `versionName` / `Cargo.toml` 的 version 并 commit
- `versionCode` 减小或重复（设备会拒绝安装；服务器 claim 已天然单调递增防重复）
- 跳过 `/api/release/finish` 调用（会让 in-flight 槽位永久残留，需运维清理 `release-state.json`）

---

## 🌐 Android 编译环境首次配置（每台新机器必做）

> 详见 `docs/android-setup.md`。

---

## ⚠️ `.gitignore` 陷阱

修改 `.gitignore` 后必须验证，防止误忽略 Rust/Kotlin 源码：

```powershell
git status --ignored --short | Select-String "^!! " | Where-Object {
    $_ -match "\.(rs|kt|toml|java|xml)$" -or $_ -match "src/.+/$"
}
# 有输出 → .gitignore 规则写错了，务必修正
```

根路径规则必须加 `/` 前缀锚定：

```gitignore
# ❌ 危险：匹配仓库任意层级的 tls/ 目录
tls/

# ✅ 正确：只匹配根目录下的 tls/
/tls/
```

---

## 📊 任务交付汇报（任务结束时必须包含）

```
✅ 本次提交 SHA：$(git rev-parse --short HEAD)
✅ 是否已推送到 origin/main：是
✅ 是否已部署到服务器：是 / 否（原因：xxx）
✅ 基于哪个 SHA 部署：<sha>
✅ 临时工作树已清理：是 / 不适用
✅ 服务健康验证结果：<curl 输出>
✅ 后端版本验证结果：<curl /api/server/version 输出>
✅ APK 发布状态：已发布 / 未发布（非 Android 任务或用户明确要求）
✅ APK 版本与下载地址：<versionName/build/downloadUrl>
```

---

## 🔗 项目关键信息速查

| 项目 | 值 |
|------|----|
| Git 远端 | `git@github.com:ElonQian1/Elon.git` |
| 主分支 | `main` |
| 服务器 SSH | `root@43.139.149.158` |
| 服务器项目路径 | `/root/Elon` |
| Rust 二进制 | `/root/Elon/server/target/release/elon-server` |
| 服务日志 | `/root/elon-server.log` |
| 服务端口 | `8080` |
| 健康检查 | `curl --noproxy '*' http://43.139.149.158:8080/health` |
| APK 下载地址 | `http://43.139.149.158:8080/app/ElonSpeed-latest.apk` |
