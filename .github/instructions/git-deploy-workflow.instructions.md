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
  → git push origin HEAD:main
  → check-task-complete -Kind CodePushed
  → 临时工作树构建 / 部署（需要发布时；被更新 main 超越则停止追车）
  → 验证（明确负责发布时）
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
| 代码已 push 后为了让本代理发布成功而反复 rebase / 重跑构建 | 并行任务会互相追车；发布应由运行时最新 main 或发布协调者统一完成 |
| 在同一个 `target` 上并行裸跑多个 Cargo 验证命令 | `cargo check` / `cargo test` 会同时写 dep-info / fingerprint 临时文件，Windows 上尤其容易互踩；日常验证必须走 `scripts\cargo-dev.ps1` / `scripts/cargo-dev.sh` 的共享 target 锁 |
| 在共享脚本里写死某台机器的本机盘符（如 `E:\rust-target\...`） | 其他 PC / 远程 Codex 没有这个盘，脚本秒退；本机差异必须走 `.env.local` + `RUST_SERVER_MUSL_TARGET_DIR` / `ELON_BUILD_TARGET_DIR`（详见 `scripts/publish-server.ps1` 的 `.NOTES` 注释和 `.env.local.example`） |

---

> 脚本内置并发安全（祖先检查 + flock+CAS），详见 `scripts/publish-server.ps1` 注释。中止提示是正常保护，不是失败；强制覆盖用 `-Force`。

---

## ✅ 提交规则

1. **任务开始前**：先运行机器预检脚本，Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`
2. 脚本会把本地 `main` 当作只同步基线：先直连 fetch，再在 `main` 干净时快进到最新 `origin/main`，最后创建带时间、PID 和短随机 ID 的独立任务 worktree
3. 如果预检脚本输出 `WORKTREE_CREATED=true`，必须切换到 `WORKTREE_PATH` 后再编辑文件；脚本输出的 `EDIT_ROOT` 是本轮唯一允许编辑、格式化、测试、提交的目录；原工作区只保留 `main` 基线，不继续叠加新改动
4. 修改预检脚本、worktree 清理脚本或本工作流说明后，必须运行 `powershell -ExecutionPolicy Bypass -File scripts\test-ai-task-preflight-workflow.ps1`；该门禁会验证 `main` 必须派生任务 worktree，同时干净、未落后、已隔离的非 `main` worktree 显式传 `-CreateWorktree` 时不会重复创建嵌套 worktree（nested worktree）
5. 任务开始 **前后** 各执行一次 `git status --short`，识别当前工作区是否有其他 AI 的未提交改动
6. `git add` 只加自己任务相关的文件
7. 发现其他代理未提交改动 → **不回退、不覆盖**，回到预检脚本创建的 worktree 继续
8. 提交前执行 `git fetch origin main` 了解远端是否前进；不要为了“永远基于最新远端”在提交前反复 rebase。自己的提交完成后第一时间 push。
9. **每次 commit 后必须立即 `git push origin HEAD:main`**；只有 push 被 non-fast-forward 拒绝时，才 `git fetch origin` + `git rebase origin/main` 一次，把自己的提交接到远端最新提交之后再推。
10. push 成功后运行 `powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed`（Linux/macOS 可用同等 git 祖先检查），确认本次 HEAD 已包含在 `origin/main`。
11. 如果本次是在隔离 worktree 中提交并推送，收尾时回到原主工作区执行 `git fetch origin` + `git pull --ff-only origin main`，让本地已跟踪文件追上远端；不 `git add`、不 stash、不删除/移动未跟踪文件。若本地未跟踪文件与远端新增同名路径冲突，停止并报告路径。

### Rebase 触发条件

| 事件 | 是否 rebase | 正确动作 |
|---|---|---|
| 开工前 `origin/main` 已前进 | 否 | 由预检脚本从当时最新 `origin/main` 派生隔离 worktree |
| 编码中发现 `origin/main` 又前进 | 否 | 继续完成本任务提交；不要为了“保持最新”中断重跑 |
| 提交前 `git fetch origin main` 发现远端前进 | 否 | 只作为态势信息；提交后立即 push |
| `git push origin HEAD:main` 被 non-fast-forward 拒绝 | 是 | `git fetch origin` → `git rebase origin/main` → 解决冲突 → 重推 |
| 本任务 HEAD 已包含在 `origin/main` | 否 | 代码同步完成；后续 main 前进不影响本任务完成状态 |
| 发布构建期间被更新 main / 服务器版本超越 | 否 | 停止上传旧产物，汇报“代码已合并，发布交给最新主线” |

**禁止把 `origin/main` 前进本身当作 rebase / 重跑构建 / 重新发布条件。** 这种追最新 HEAD 的行为会让多个代理互相追车，形成活锁。

### 本地 main 基线和任务 worktree

| 场景 | 做法 |
|---|---|
| 当前在 `main` | 运行预检脚本并进入 `WORKTREE_PATH`，不要直接编辑 `main` |
| 当前任务 worktree 干净且不落后 | 可以继续当前任务 |
| 当前任务 worktree 落后远端 | 继续完成本任务提交并第一时间 push；只有 push 被拒绝时才 `git fetch origin` → `git rebase origin/main` → 解决冲突并重推 |
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
git push origin HEAD:main

# 2b. 有冲突 → 逐文件解决后继续
# 解决冲突（编辑文件，保留正确内容）
git add <冲突文件>
git rebase --continue
git push origin HEAD:main
```

**rebase 冲突解决原则**：
- 自己改的逻辑 + 对方改的逻辑 **都保留**（除非明确互斥）
- 不允许用 `git rebase --abort` 丢弃自己的修改
- 解决后在汇报中注明："遇到推送冲突，已 rebase 解决，最终 SHA：xxx"

---

## 🔀 多 AI 并发规则

| 当前位置 / 状态 | 做法 |
|---|---|
| 当前在 `main`，即使干净 | 先运行预检脚本，进入新建的 `WORKTREE_PATH`，不要直接编辑 `main` |
| 当前在本任务 worktree，且干净不落后 | 可以继续当前任务 |
| 当前在本任务 worktree，但落后远端 | 完成本任务提交后立即 push；只有 push 被拒绝时再 rebase 到 `origin/main` |
| 当前 worktree 有其他 AI / 其他任务 / 来源不明改动 | 不回退、不覆盖；重新从 `origin/main` 创建独立 worktree |

### 独立工作树隔离

```powershell
# 1. 让脚本同步 main 基线，并从最新 origin/main 创建本会话专属工作树
powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree

# 2. 按脚本输出进入 WORKTREE_PATH 后再改代码
Set-Location "<WORKTREE_PATH>"
# ... 修改文件 ...
git add <自己的文件>
git commit -m "feat(scope): 描述"

# 3. 第一时间把本会话提交推入远端 main
git push origin HEAD:main
# 若 push 被拒绝：git fetch origin; git rebase origin/main; 解决冲突后重推
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed

# 4. 清理已合并且干净的历史 AI worktree
powershell -ExecutionPolicy Bypass -File scripts\cleanup-task-worktrees.ps1 -Apply
```

---

## 🦀 Rust 代码格式化规则（增量优先，纯格式化拆提交）

> **方针**：日常任务不重构历史代码，优先只对新改动增量规范。改 `.rs` 文件后，**只对自己改过的文件**跑仓库格式化脚本；脚本会按文件所属 crate 的 `Cargo.toml` 读取 edition，再调用 `rustfmt --edition <crate edition>`。

### Cargo 验证共享缓存与锁

日常 `cargo check` / `cargo test` / `cargo build` / `cargo clippy` 使用开发验证脚本，不直接裸跑 Cargo：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 check --manifest-path server\Cargo.toml
powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml pc_lightweight
```

```bash
bash scripts/cargo-dev.sh check --manifest-path server/Cargo.toml
bash scripts/cargo-dev.sh test --manifest-path server/Cargo.toml pc_lightweight
```

脚本会读取 `.env.local` / `ELON_DEV_CARGO_TARGET_DIR`，设置开发用 `CARGO_TARGET_DIR` 并在 target 目录上加锁。同一台机器多个 AI 可以复用同一份开发编译缓存，但同一时间只允许一个 Cargo 进程写这个 target，避免 dep-info / fingerprint 临时文件互踩。版本化的 pre-push hook 也必须走同一个 `cargo-dev` 入口。不要把开发验证 target 和发布构建 target 混用；服务端发布继续由 `RUST_SERVER_MUSL_TARGET_DIR` + `publish-server.*` 管理。

```powershell
# ✅ 一条命令格式化所有本次改动的 .rs 文件（修改 + 新增全覆盖）
$rs = @(git diff --name-only) + @(git ls-files --others --exclude-standard) |
  Where-Object { $_ -match '\.rs$' }
if ($rs) { powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1 -Apply -Files $rs }
```

如果用户明确要求、任务确实需要，或已经产生一次全量 Rust 格式化，必须走仓库脚本逐个指定 crate manifest，让 `cargo fmt` 从 `Cargo.toml` 读取 edition：

```powershell
# 只检查
powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1

# 全量写入格式化
powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1 -Apply
```

全量 Rust 格式化本身不是错误；错误的是把它和业务/文案/逻辑改动混在同一个 commit。正确处理：

1. 最好在业务修改前，从干净 worktree 运行全量格式化，然后单独提交 `style(rust): 全量格式化 Rust 代码`。
2. 如果已经在任务中产生全量格式化，先用 `git diff --stat` / `git diff --check` 确认它是纯 rustfmt 机械变化；不要为了缩小 diff 回退纯格式化。
3. 将纯格式化文件或格式化 hunks 单独提交为 `style(rust): ...`；业务、文案、测试语义改动另起提交。
4. 只有格式化命令用错 edition、生成非 rustfmt 变化、或碰到来源不明的未提交改动/其他代理正在负责的文件时，才停止并报告；不要自动回退别人的改动。

Linux/macOS/服务器 CLI 使用：

```bash
bash scripts/format-rust.sh
bash scripts/format-rust.sh --apply

# 只格式化指定文件
bash scripts/format-rust.sh --apply --files server/src/main.rs
```

**禁止**：
- 把全量 `cargo fmt` / rustfmt 结果混入业务 commit
- 在仓库根裸跑 `cargo fmt` 或 `rustfmt` 后继续提交，而不经过仓库脚本/edition 复核
- 修改来源不明的未提交改动，或覆盖其他 AI 正在负责的 `.rs` 文件

> `scripts/format-rust.* --files <files>` 只格式化指定文件，几百毫秒完成，不触发重编译；脚本会从所属 crate manifest 读取 edition。仓库根目录也有 `rustfmt.toml` 固化 edition，显式参数和 `scripts/format-rust.*` 的 manifest-path 用于避免 AI 或脚本在其他工作目录直接调用 rustfmt 时回退到旧默认 edition。

---

> 共享脚本不允许写死路径；Rust 构建缓存配置详见 `scripts/publish-server.ps1` 的 `.NOTES` 注释和 `.env.local.example`。
> 后端发布脚本会强制使用通用 `-C target-cpu=x86-64`，并报告全局 Cargo config / 环境变量中的 `target-cpu=native`；不得让机器级 native rustflags 参与服务器发布构建。

---

## 🦀 后端部署（Rust → Linux 服务器）

```powershell
git add server/src/<file>.rs; git commit -m "fix(server): 描述"; git push origin HEAD:main
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed
.\scripts\publish-server.ps1   # claim → 编译 → 部署 → finish（内部原理见脚本注释）
curl --noproxy '*' http://43.139.149.158:8080/api/server/version
```

> Linux/macOS: `bash scripts/publish-server.sh`。仅本地验证不部署：`.\publish-server.ps1 -SkipUpload`。版本号由服务器 `/api/release/claim` 原子分配，不写 git。并行任务中，发布脚本发现 `origin/main` 或服务器已进入更新版本时应停止上传并汇报，不要求本代理继续 rebase 重跑。

---

## 🪟 Windows PC 节点客户端部署

> 影响 Win 端节点客户端、启动器、安装/自更新、节点托盘、`elon-pc-node` 二进制或 `scripts/publish-node-agent.ps1` 的用户可见修复，代码进入 `origin/main` 后默认继续发布 Win 节点客户端，除非用户明确说只同步代码或暂不发布。

```powershell
git add server/src/node_agent_... scripts/publish-node-agent.ps1
git commit -m "fix(node): 描述"
git push origin HEAD:main
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed

scripts\publish-node-agent.ps1
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind NodeAgent
```

`publish-node-agent.ps1` 会：
- 构建 Linux `elon-pc-node`、Windows `elon-pc-node.exe` 和完整 Windows 客户端 zip。
- 上传到 `/opt/elon/data/downloads/`，并更新 `/api/node-agent/version` 对应的 `node-agent-version.json`。
- 默认调用 `/api/admin/nodes/push-update` 广播 `UpdateClient`，在线 Win 节点收到后自动更新、自动重连；离线节点下次启动会读取版本接口补更新。

运行发布脚本需要本机环境变量 `ADMIN_TOKEN` 或 `ELON_ADMIN_TOKEN`。如果只允许上传产物、不允许广播，必须显式传 `-SkipBroadcast`，最终汇报也必须说明“未推送在线节点更新”。

---

## 📱 Android APK 部署

> Android 新功能的代码完成标准是“业务提交已进入 `origin/main`”；APK 发布标准是“服务器 APK 指向最新主线”。并行任务先确保代码 push；只有明确负责发布的任务才必须等到 `AndroidFeature` 校验通过。

### APK 发布两步式工作流（v0.3.69+ 新流程：版本号不进 git）

```
[第1步：提交业务代码]   git add 业务文件 → commit → git push origin HEAD:main
        ↓
[第1.5步：代码完成检查] check-task-complete.ps1 -Kind CodePushed
        ↓
[第2步：运行发布脚本]   publish-apk.ps1 -Changelog "..."（需要发布时）
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
git push origin HEAD:main          # 脚本基于远端 HEAD，必须先 push
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed

# 第2步：运行发布脚本（明确负责 APK 发布时）
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

> APK 发布并发保护（慢构建防覆盖、finish 槽位释放）详见 `scripts/publish-apk.ps1` 注释。并行任务中，若发布脚本提示已被更新的 `origin/main` 或服务器 APK 超越，视为“代码已合并，发布交给最新主线”，不要为了当前代理发布成功继续 rebase 重跑。
---

## 🏷️ 版本号管理规则（v0.3.69+ 服务器分配）

> 版本号由服务器 `/api/release/claim` 原子分配，不写 git。`server/Cargo.toml` 和 `build.gradle` 里的版本字段是冷启动兜底，不得手动递增并 commit。

**禁止**：
- ❌ 手动递增 `versionCode`、`versionName`、`Cargo.toml` version 并 commit
- ❌ 跳过 `/api/release/finish`（会让 in-flight 槽位永久残留，需运维清理）

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
✅ Win 节点发布状态：已发布并推送 / 仅代码同步 / 未尝试发布
✅ Win 节点版本与下载地址：<version/gitSha/windows-client-url>
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
