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

**中止时本机的正确处理**：

1. 已经 `git push` 的版本号 commit / 后端版本号 commit **保留**，不回退（远端历史无损失）。
2. 本次编译的产物（APK / binary）**作废**，不要上传。
3. 直接信任服务器上的新版本：
   - 后端：`curl --noproxy '*' http://43.139.149.158:8080/api/server/version` 查看
   - APK：`curl --noproxy '*' http://43.139.149.158:8080/app/version.json` 或直接下载 `ElonSpeed-latest.apk`
4. 如果**确认**必须用本机版本覆盖（极少数情况，如对方推了错误版本）：重跑脚本加 `-Force`。

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
git add server/Cargo.toml          # 以及 Cargo.toml（如有新依赖）
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

## 🦀 Rust target 与构建缓存安全

本机 skills 中的 `rust-shared-target-cache` 已确认一个高风险事故：相对路径的 `CARGO_TARGET_DIR` 会随 cwd 展开到不同位置，导致多个鬼影 `target/` 目录、磁盘暴涨，以及构建产物找错路径。

执行本项目 Rust 构建时遵守：

- 优先使用仓库脚本：`scripts\publish-server.ps1` / `scripts\publish-server.sh`。脚本会设置明确的构建输出目录，不依赖用户级相对路径。
- 裸跑 `cargo check`、`cargo build`、`cargo zigbuild` 前，如果构建行为异常或机器是新环境，先检查 `CARGO_TARGET_DIR`：

```powershell
[System.Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "User")
[System.Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR", "Machine")
```

- 如果任何输出包含 `..`、`.` 开头或其他相对路径，当前任务不要继续裸跑构建；临时清掉当前进程变量并改用绝对路径，后续再修正用户级配置：

```powershell
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
$env:CARGO_TARGET_DIR = "E:\rust-target\elon-local"
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
# 1. 提交
git add server/Cargo.toml          # 后端运行代码变化必须递增 package.version
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

`publish-server.ps1` / `publish-server.sh` 会自动完成：`git pull --rebase origin main`、基于干净 `HEAD` 创建临时 worktree、在本地开发机用 `cargo zigbuild --target x86_64-unknown-linux-musl` 交叉编译、把编译好的 `elon-server` binary 上传到服务器 staging 路径、通过服务器 `flock` + CAS 替换 binary、重启服务并验证。

> 生产服务器只负责接收 binary、替换、重启和健康检查，不承担编译。**不要**恢复“rsync 源码到服务器后 `cargo build --release`”的旧流程；服务器性能弱，远端编译慢且容易和并发部署互相覆盖。

### 仅本地验证 binary（不部署）

```powershell
cd scripts
.\publish-server.ps1 -SkipUpload
```

```bash
bash scripts/publish-server.sh --skip-upload
```

旧入口 `scripts/deploy.sh` 只保留为兼容包装器，内部会转到 `scripts/publish-server.sh`，不得再实现远端编译。

### 后端版本号规则

**`scripts/publish-server.ps1` 会自动完成 PATCH 递增 → commit → push → 构建 → 部署 → 验证。**  
通常直接运行脚本即可，无需手动修改版本号。

| 情况 | 递增位 | 做法 |
|---|---|---|
| 修复 Bug，无新接口能力 | PATCH（自动） | 直接运行 `.\scripts\publish-server.ps1` |
| 新增后端功能，向后兼容 | MINOR，PATCH 归零 | 手动改 `server/Cargo.toml` 版本后加 `-SkipVersionBump` |
| 不兼容 API / 协议变更 | MAJOR，其余归零 | 手动改 `server/Cargo.toml` 版本后加 `-SkipVersionBump` |

```powershell
# 普通部署（自动递增 PATCH）
.\scripts\publish-server.ps1

# 手动控制 MINOR/MAJOR 时（已在 Cargo.toml 手动改好版本号）
.\scripts\publish-server.ps1 -SkipVersionBump
```

脚本会在构建时注入 git SHA，服务端通过 `/api/server/version` 返回 `versionName` 和 `gitSha`，APK 个人页动态读取该接口展示后端版本。

---

## 📱 Android APK 部署

> Android 新功能不是“代码合并即完成”。只要改了 APK 可安装端能力，必须跑发布脚本并校验服务器版本。

### APK 发布三步式工作流（严格分离，不得合并）

```
[第1步：提交业务代码]   git add 业务文件 → commit → push
        ↓
[第2步：运行发布脚本]   publish-apk.ps1 -Changelog "..."
        （脚本自动：versionCode+1 → 编译 → 上传 → 验证）
        ↓
[第3步：提交脚本产生的版本号变动]   git add build.gradle → commit → push
```

**第1步必须先单独提交**，不能把业务代码和版本号文件混在同一个 commit。

```powershell
# 第1步：先提交业务代码
git add android/app/src/main/...   # 只加业务文件，不加 build.gradle
git commit -m "feat(android): 描述"
git push origin main               # 脚本基于远端 HEAD，必须先 push

# 第2步：运行发布脚本（自动完成版本号递增、编译、上传）
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

### ⚠️ 并发发布冲突：push 被拒后必须重跑脚本

当第2步脚本的最终 `git push` 被拒绝（non-fast-forward），**绝对不能只 rebase 再推**：

```
❌ 错误做法：
  git pull --rebase → git push   ← 产物里嵌入的是旧 versionCode！
  → version.json 的版本号 ≠ APK 实际内嵌版本号 → 手机死循环弹更新提示

✅ 正确做法：
  git pull --rebase origin main   # 先同步对方的版本号 commit
  重新运行 scripts\publish-apk.ps1 -Changelog "..."   # 基于最新版本号重新编译上传
```

**原因**：APK 产物在编译时已经把旧的 `versionCode` 打包进去了，不重新编译就上传，会导致 `version.json` 中的版本号与 APK `BuildConfig.VERSION_CODE` 不一致。

### ⚠️ APK 慢构建防覆盖规则

`scripts\publish-apk.ps1` 必须把 APK 发布视为基于明确 git SHA 的原子动作：

- 构建开始前记录基础 SHA。
- 构建期间定期检查 `origin/main` 和服务器 `.apk-deployed-sha`；如果线上 APK 已部署了包含基础 SHA 的更新提交，立即中止本地旧编译，改为测试线上新版。
- 构建完成后、提交 release commit 前再次检查 `origin/main`；如果远端已前进但线上还未确认包含基础 SHA，停止发布并要求基于最新 `main` 重跑。
- `git push HEAD:main` 被拒绝时，脚本不得自动 rebase 后继续上传旧 APK；必须停止并要求重新运行发布脚本。
- 上传 APK 和 `version.json` 必须先传 staging 文件，再由服务器 `flock` + `.apk-deployed-sha` CAS 原子替换；慢构建不得覆盖已经上线的后代 SHA。
- `version.json` 必须包含发布 commit 的 `gitSha`，`check-task-complete.ps1 -Kind AndroidFeature` 必须校验线上 `gitSha` 等于当前 HEAD。

如果用户明确要求“只改代码，不发布 APK”，最终汇报必须写明 `APK 发布状态：未发布（用户明确要求）`。

---

## 🏷️ 版本号管理规则

> 版本号由两个字段组成，均在 `android/app/build.gradle` 维护。

| 字段 | 格式 | 规则 |
|------|------|------|
| `versionCode` | 整数，只增不减 | **每次发布 APK 必须 +1** |
| `versionName` | `MAJOR.MINOR.PATCH` | 按语义版本递增 |

**语义版本递增规则**：

| 情况 | 递增位 | 示例 |
|------|--------|------|
| 修复 Bug，无新功能 | PATCH | `1.2.3` → `1.2.4` |
| 新增功能，向后兼容 | MINOR，PATCH 归零 | `1.2.3` → `1.3.0` |
| 破坏性变更（API 不兼容） | MAJOR，其余归零 | `1.2.3` → `2.0.0` |

**AI 代理执行 APK 发布时必须**：

```powershell
# 1. 自动读取并递增 versionCode（脚本化，不依赖 AI 手动计算）
$repoRoot = git rev-parse --show-toplevel
$gradlePath = "$repoRoot\android\app\build.gradle"
$content = Get-Content $gradlePath -Raw

$oldCode = [int]([regex]::Match($content, 'versionCode\s+(\d+)').Groups[1].Value)
$newCode = $oldCode + 1
$content = $content -replace "versionCode\s+$oldCode", "versionCode $newCode"
Set-Content $gradlePath $content -NoNewline
Write-Host "versionCode: $oldCode → $newCode"

# 2. 根据本次改动类型手动决定 versionName 递增哪位（见上方规则表）
#    bug fix → PATCH；新功能 → MINOR；破坏性变更 → MAJOR
$oldName = [regex]::Match($content, 'versionName\s+"([\d.]+)"').Groups[1].Value
# 手动替换 $content 里的 versionName 为新版本号，再次 Set-Content

# 3. 将版本号写入 commit message（格式固定）
git commit -m "release(android): vNEW_VERSION (build $newCode) - <改动描述>"
```

**禁止**：
- 发布 APK 时不更新版本号
- `versionCode` 减小或重复（设备会拒绝安装）
- 多人同时发布时使用相同 `versionCode`（后推送的一方 push 被拒绝后必须再次 +1 再推）

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
