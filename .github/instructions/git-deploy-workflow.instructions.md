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

## ❌ 绝对禁止

| 禁止行为 | 原因 |
|---------|------|
| 修改代码后直接 build/rsync，跳过 commit/push | 半成品代码上线，无法追溯，无备份 |
| 在主工作区执行部署构建（rsync/scp） | 可能把其他 AI 未提交的改动一并同步到服务器 |
| 只 commit 不 push | 本地磁盘故障 = 工作成果全部丢失 |
| 在主工作区执行 `git reset --hard`、`git checkout --` | 会覆盖其他并发 AI 的未提交改动 |
| 夹带无关文件进同一次 commit | 污染提交历史，妨碍定位问题 |

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

```powershell
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

`scripts\publish-apk.ps1` 会自动完成：`git pull --rebase origin main`、递增 `versionCode/versionName`、构建 release APK、提交 release commit、`git push origin HEAD:main`、上传 `ElonSpeed-latest.apk` 和 `version.json`、验证服务器版本。

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
| 健康检查 | `curl http://43.139.149.158:8080/health` |
| APK 下载地址 | `http://43.139.149.158:8080/app/ElonSpeed-latest.apk` |
