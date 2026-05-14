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

1. 任务开始 **前后** 各执行一次 `git status --short`，识别主工作区是否有其他 AI 的未提交改动
2. `git add` 只加自己任务相关的文件
3. 发现其他代理未提交改动 → **不回退、不覆盖**，只提交自己的文件
4. **每次 commit 后必须立即 `git push origin main`**

> 分支名是 `main`（不是 `master`）。

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
Set-Location "d:\一龙\一龙参考库"
git cherry-pick <session_commit_sha>
git push origin main

# 4. 清理会话工作树
git worktree remove ..\Elon-session-$id --force
```

---

## 🦀 后端部署（Rust → Linux 服务器）

**服务器信息**：`ubuntu@182.254.168.75`，项目路径 `/home/ubuntu/Elon`

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
  "ubuntu@182.254.168.75:/home/ubuntu/Elon/"

# 4. 在服务器上编译并重启
ssh ubuntu@182.254.168.75 @"
  source ~/.cargo/env
  cd /home/ubuntu/Elon/server
  cargo build --release 2>&1 | tail -5
  pkill -f elon-server 2>/dev/null || true
  sleep 1
  nohup ./target/release/elon-server > /home/ubuntu/elon-server.log 2>&1 &
  echo "已启动 PID: $!"
"@

# 5. 验证
ssh ubuntu@182.254.168.75 'curl -s http://localhost:8080/health'

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
Set-Location "d:\一龙\一龙参考库\android"
.\gradlew.bat assembleRelease

# 4. 上传 APK 到服务器
$apk = Get-ChildItem "app\build\outputs\apk\release\*.apk" | Select-Object -First 1
scp $apk.FullName "ubuntu@182.254.168.75:/home/ubuntu/Elon/app/ElonSpeed-latest.apk"

# 5. 验证
ssh ubuntu@182.254.168.75 'ls -lh /home/ubuntu/Elon/app/ElonSpeed-latest.apk'
```

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
| 服务器 SSH | `ubuntu@182.254.168.75` |
| 服务器项目路径 | `/home/ubuntu/Elon` |
| Rust 二进制 | `/home/ubuntu/Elon/server/target/release/elon-server` |
| 服务日志 | `/home/ubuntu/elon-server.log` |
| 服务端口 | `8080` |
| 健康检查 | `curl http://182.254.168.75:8080/health` |
| APK 下载地址 | `http://182.254.168.75:8080/app/ElonSpeed-latest.apk` |
