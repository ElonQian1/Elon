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

1. **任务开始前**：先执行 `git pull --rebase origin main` 同步最新代码，再开始修改
2. 任务开始 **前后** 各执行一次 `git status --short`，识别主工作区是否有其他 AI 的未提交改动
3. `git add` 只加自己任务相关的文件
4. 发现其他代理未提交改动 → **不回退、不覆盖**，只提交自己的文件
5. **每次 commit 后必须立即 `git push origin main`**

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
# 1. 基于 main 创建本会话专属工作树
$id = Get-Random -Maximum 9999
git worktree add ..\Elon-session-$id -b ai/session-$id main

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

### 完整流程

```powershell
# 1. 提交
git add server/src/<file>.rs          # 只加自己改的文件
git commit -m "fix(server): 描述"
git push origin main

# 2. 创建临时工作树（保证基于干净 commit 构建，不含其他 AI 未提交改动）
$sha = git rev-parse --short HEAD
$tmp = "..\Elon-deploy-temp-$sha"
git worktree add --detach $tmp HEAD

# 3. 从临时工作树 rsync 到服务器（排除 target/ 和 .env）
rsync -avz --exclude='target/' --exclude='.env' `
  "$tmp/" `
  "root@43.139.149.158:/root/Elon/"

# 4. 在服务器上编译并重启
ssh root@43.139.149.158 @"
  source ~/.cargo/env
  cd /root/Elon/server
  cargo build --release 2>&1 | tail -5
  pkill -f elon-server 2>/dev/null || true
  sleep 1
  nohup ./target/release/elon-server > /root/elon-server.log 2>&1 &
  echo "已启动 PID: $!"
"@

# 5. 验证
ssh root@43.139.149.158 'curl -s http://localhost:8080/health'

# 6. 清理临时工作树（必须！）
git worktree remove $tmp --force
```

### 快速部署（仅修改服务端单个文件时）

```powershell
# 提交
git add server/src/<file>.rs
git commit -m "fix(server): 描述"
git push origin main

# 用现有 deploy.sh（等价于上方完整流程，但不使用工作树隔离）
# ⚠️ 仅在确认主工作区无其他 AI 未提交改动时才可用此快捷方式
bash scripts/deploy.sh
```

> 如果主工作区有其他 AI 的未提交改动，**必须**用上方临时工作树流程。

---

## 📱 Android APK 部署

> Android 是**例外**：允许在主工作区构建（Gradle 需要提交版本号）。

```powershell
# 1. 更新版本号（如需要）
# android/app/build.gradle → versionCode + versionName

# 2. 提交
git add android/app/build.gradle android/app/src/  # 只加 android 相关文件
git commit -m "feat(android): 描述 - vX.X.X"
git push origin main

# 3. 本地编译 APK
$repoRoot = git rev-parse --show-toplevel
Set-Location "$repoRoot\android"
.\gradlew.bat assembleRelease

# 4. 上传 APK 到服务器
$apk = Get-ChildItem "app\build\outputs\apk\release\*.apk" | Select-Object -First 1
scp $apk.FullName "root@43.139.149.158:/root/Elon/app/ElonSpeed-latest.apk"

# 5. 验证
ssh root@43.139.149.158 'ls -lh /root/Elon/app/ElonSpeed-latest.apk'
```

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
