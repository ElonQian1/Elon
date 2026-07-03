# AI 代理工作流 — 代码修改·编译·部署完整流程

> 本文档描述 AI 代理在接收到用户需求后，如何安全地修改代码、触发编译、部署，并将结果反馈给用户。
> 它是按需参考文档，不是每轮必读入口；常规任务先读 `AGENTS.md` / `CODEX.md`，只有完整流程、发布异常或任务卡住时再读本文。

> **强制工作流规则**（git 提交、多 AI 并发、临时工作树部署）见：
> [.github/instructions/git-deploy-workflow.instructions.md](../.github/instructions/git-deploy-workflow.instructions.md)
> 该文件保留 Git/部署细则；Codex 按 `AGENTS.md` 路由按需读取。

> **模块化与长期维护规则**（避免巨型文件、按职责拆模块、多 AI 并行边界）见：
> [.github/instructions/modular-architecture.instructions.md](../.github/instructions/modular-architecture.instructions.md)
> 该文件主要应用于源码类文件；Codex 按 `AGENTS.md` 路由按需读取。

---

## 项目进入规则（APK / Web / 服务器 Codex CLI 通用）

1. 每次进入项目先运行任务预检脚本：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`；脚本会先同步本地 `main` 基线，再从最新 `origin/main` 派生独立任务 worktree。如果脚本创建了 worktree，必须切到 `WORKTREE_PATH` 后再观察目录结构和修改文件。脚本输出的 `EDIT_ROOT` 是本轮唯一允许编辑、格式化、测试、提交的目录；如果是 `BLOCKED_CREATE_WORKTREE_FIRST`，当前目录不能继续改。
2. 如果存在 `AGENTS.md`、`CODEX.md` 或 `README.md`，先读轻量入口；`.github/instructions/*.md` 和 `docs/` 只在当前任务需要时读取。
3. `local_path` 和 GitHub 项目按已有 Git 仓库处理。`main` checkout 只作为共享同步基线，不作为业务编辑区；新业务流必须从预检脚本创建的任务 worktree 开始。当前任务自己的提交完成后第一时间 `git push origin HEAD:main`；只有 push 被 non-fast-forward 拒绝时才 rebase。`origin/main` 在编码或构建期间前进是并行常态，不是自动 rebase、重跑验证或重新发布的条件。其他任务或来源不明的未提交改动必须用 `origin/main` 新建 worktree。
4. 一龙项目只是默认登记的 `local_path` 项目，不走特殊执行路径；其他 GitHub 下载或本地挂载项目也应靠自己的项目文档驱动流程。
5. Codex CLI 的长期记忆来自项目文件，不来自服务器进程本身。流程变化必须写回文档并提交。
6. 如果任务在隔离 worktree 完成并推送，收尾时回到原主工作区执行 `git fetch origin` + `git pull --ff-only origin main`，只同步已跟踪文件；不要 stage、stash、删除或移动原主工作区的未跟踪文件，遇到同名路径冲突就报告。
7. Android APK 新功能默认先以“业务提交已进入 `origin/main`”为第一层完成定义；并行任务可以先用 `CodePushed` 收尾。只有用户明确要求安装包交付、下载链接或线上发布时，才以“服务器 APK 指向最新主线”为最终完成定义。
8. 手机触发的项目开发流程中，后端预检错误只作为上下文交给 CLI；CLI 应先自查 Git 现场并尝试安全处理，只有判断无法克服时才向用户说明并暂停。

### 并行 rebase 边界

合理流程固定为：

1. 开工前：从当时最新 `origin/main` 派生独立 worktree。
2. 提交后：立即 `git push origin HEAD:main`。
3. 只有 push 被 non-fast-forward 拒绝：才 `git fetch origin` + `git rebase origin/main`，解决冲突后再 push。
4. 自己的 commit 已经进入 `origin/main` 后：任务代码层面完成；后续 `origin/main` 再前进，不应该为了“保持自己是最新 HEAD”反复 rebase。
5. 发布阶段：如果构建产物被更新的 main 超越，停止上传旧产物，汇报“代码已合并，发布交给最新主线”，而不是重跑。

---

## 项目任务并发规则（APK / 服务器通用）

1. 服务器以 `project_id + conversation_id` 为单位分配会话执行权：同一会话内串行，避免一个会话的连续上下文和分支被两个任务同时修改。
2. 普通服务器 worktree 路线允许同一项目的不同会话并行编码；进入开发流程后，后端为每个 APK 会话创建或复用独立 Git worktree 和 `ai/session/...` 分支。主工作区的 `main` 只负责跟随最新 `origin/main`，不得被业务会话长期占用。
3. PC 节点外部 CLI 路线还要按“PC 节点 + CLI”分配容量槽位：不同 PC 节点可以并行，同一 PC 节点上的 Codex / Claude / Copilot 等外部 CLI 也可以并行，但不能无上限并发。当前试运行默认每个 PC 节点 6 个 CLI 槽位；硬件估算只作为容量参考，不再把默认值压低到 1-4。服务端仍可用 `ELON_PC_NODE_CLI_MAX_PARALLEL` / `ELON_PC_NODE_CLI_HARD_MAX_PARALLEL` 兜底控制。
4. 同一 PC 节点所有 CLI 槽位都被占用时，新任务进入节点队列，而不是继续启动更多 CLI 进程。这样能避免多个会话同时争用同一个本机登录态、sidecar、缓存和项目路径，导致只剩等待状态、没有 CLI 输出、最终超时失败。
5. PC 前端必须明确展示节点容量状态：节点槽位已满、并发槽位数、排队等待时长、已获得节点 CLI 执行权、PC 节点已确认接收、CLI 公开输出、命令/文件/测试/最终回复。云端只写出派发消息不等于本机节点已收到；Route A 必须用节点 ACK 区分“已送达本机”和“等待 CLI 输出”。
6. 推送到远端主线、Android 版本 claim、APK 发布、服务器部署、数据库任务状态落库必须串行；如果项目不能创建 worktree，退回项目级共享工作区串行执行。
7. 一龙自项目与普通 GitHub / `local_path` 项目遵守同一套规则，不允许隐藏特殊流程。

---

## 后端与 Codex CLI 协作边界

1. 后端是流程指挥官，Codex CLI 是代码执行者。
2. 后端调用 Codex CLI 前，必须先确认项目身份、工作区路径、Git/origin、权限、队列/锁状态和用户选择的模型。
3. 后端每次都给 Codex CLI 注入任务单：用户需求、项目路径、必须读取的文档顺序、Git 规则、验证要求、共享发布动作必须串行。
4. 以后即使接入其他 AI 模型，它们也只能作为旁路工具做轻量分类、摘要、图片/特殊分析或检索增强；旁路结论必须由后端整理后回灌到当前 APK 会话绑定的 Codex CLI 原生 session，不能另起长期主会话。
5. Codex CLI 不能依赖跨任务记忆；未知项目先读 `AGENTS.md`、`CODEX.md`、`README.md` 等轻量入口，再按任务读取细则。
6. Codex CLI 完成后，后端负责验收和产品化状态：任务记录、进度展示、下载链接、版本信息、合并/发布/部署锁。
7. 并发安全、版本顺序、APK 发布、服务器部署不能只靠提示词，必须由后端代码和发布脚本强制执行。
8. 后端不能因为 Git 同步的业务性失败直接终止开发任务；应把失败原因注入 CLI 任务单，让 CLI 优先自愈。只有 CLI 启动失败、超时、IO 异常这类平台问题，才由后端直接失败或切换 fallback。

---

## 总体流程

```
用户需求
  │
  ▼
Step 1: 需求分析与分类
  │
  ▼
Step 2: 定位需要修改的代码
  │
  ▼
Step 3: 生成代码修改方案（先规划，后执行）
  │
  ▼
Step 4: 执行代码修改
  │
  ▼
Step 5: 本地验证（语法检查/lint）
  │
  ▼
Step 6: git commit 提交
  │
  ▼
Step 7: 触发编译流水线
  │
  ├── 编译成功 ──► Step 8: 部署 + 打包 APK → 发送下载链接给用户
  │
  └── 编译失败 ──► Step 9: 自动修复 or 回滚 → 反馈用户
```

---

## Step 1：需求分析与分类

AI 代理接收用户消息后，必须先判断需求类型：

| 需求类型 | 示例 | 涉及代码 |
|---|---|---|
| UI 变更 | "首页加个按钮" | Android 布局 XML / 前端 HTML |
| 业务逻辑 | "点击按钮发送消息给服务器" | Android Kotlin + Rust API |
| 服务端逻辑 | "添加一个查询天气的接口" | Rust server |
| 全栈功能 | "做一个用户登录功能" | Android + Rust + 数据库 |
| 配置/文本 | "把应用名改成'我的APP'" | Android res/strings.xml |

**分析输出格式**（内部使用）：
```json
{
  "need_type": "ui_change",
  "affected_modules": ["android"],
  "affected_files": ["android/app/src/main/res/layout/activity_main.xml"],
  "estimated_complexity": "simple",
  "user_friendly_description": "在首页添加一个红色按钮"
}
```

---

## Step 2：定位需要修改的代码

### 2.1 Android 代码定位规则

| 修改内容 | 文件位置 |
|---|---|
| UI 布局 | `android/app/src/main/res/layout/*.xml` |
| 字符串/文本 | `android/app/src/main/res/values/strings.xml` |
| 颜色/样式 | `android/app/src/main/res/values/colors.xml`, `styles.xml` |
| 页面逻辑 | `android/app/src/main/kotlin/**/MainActivity.kt` 等 |
| 网络请求 | `android/app/src/main/kotlin/**/network/` |
| 权限配置 | `android/app/src/main/AndroidManifest.xml` |

### 2.2 Rust 服务端代码定位规则

| 修改内容 | 文件位置 |
|---|---|
| 新增 API 接口 | `server/src/api/` |
| 业务逻辑 | `server/src/services/` 或 `server/src/handlers/` |
| 数据模型 | `server/src/models/` |
| 配置 | `server/src/config.rs` |

### 2.3 前端代码定位规则

| 修改内容 | 文件位置 |
|---|---|
| 页面组件 | `frontend/src/components/` |
| 页面路由 | `frontend/src/pages/` |
| API 调用 | `frontend/src/api/` |
| 样式 | `frontend/src/styles/` |

---

## Step 3：生成代码修改方案

**在修改任何文件之前**，AI 代理必须：

1. **读取原始文件内容**，理解现有结构
2. **规划完整修改方案**，包括：
   - 修改哪些文件（列表）
   - 每个文件改什么（描述）
   - 是否有依赖关系（先改哪个）
3. **评估风险**：该修改是否可能破坏已有功能

```
修改前自检清单：
  □ 是否读取了要修改的文件？
  □ 修改是否局限在用户要求的范围内？
  □ 修改后代码语法是否正确？
  □ 是否需要同步修改多个文件（如接口+调用方）？
```

---

## Step 4：执行代码修改

- 使用 `replace_string_in_file` 或 `multi_replace_string_in_file` 精确修改
- **不允许**整个文件重写，除非是新建文件
- **不允许**删除用户未明确要求删除的功能
- **不允许**继续向巨型文件堆叠大段新逻辑；新建源文件默认目标 ≤500 行，501-800 行可容忍但必须单一职责，超过 800 行必须拆分；触碰 1500 行以上文件时，除小修外优先抽出本次职责模块
- 保持代码缩进、风格与原文件一致

---

## Step 5：本地验证

### Rust 代码验证
```powershell
cd server
cargo check   # 只检查语法，不完整编译，速度快
```

### Android 代码验证
```powershell
cd android
./gradlew lint   # 静态检查
```

### 前端代码验证
```powershell
cd frontend
npm run lint
```

> 如果验证失败，**立即修复，不允许带错误提交**。

---

## Step 6：git commit 提交

```powershell
git add <修改的文件列表>
git commit -m "feat(用户需求): <用中文简洁描述本次修改内容>

用户ID: {user_id}
需求原文: {original_request}
修改文件: {file_list}"
```

**commit message 规范**：
- 前缀：`feat` 新功能 / `fix` 修复 / `style` 样式 / `refactor` 重构
- 主体：中文，一句话描述用户看到的变化
- 必须包含：用户ID、需求原文

如果本次提交是在隔离 worktree 中完成并已推送到 `main`，回到原主工作区做安全同步：

```powershell
cd "<原主工作区>"
git fetch origin
git pull --ff-only origin main
```

这一步只让本地已跟踪文件追上远端；不得顺手 `git add`、stash、清理、删除或移动未跟踪文件。

---

## Step 7：触发编译流水线

### 7.1 Rust 服务端本地交叉编译

服务端发布必须使用仓库脚本在本地开发机交叉编译 Linux binary，再上传到服务器。桌面版 Codex 就是在本机编译；生产服务器只负责接收 binary、替换、重启和健康检查，不承担编译。

多台 PC 的本地盘符不同，不能把某台机器的构建目录写死在共享脚本里。需要让多个 Rust 后端项目共享同一份 musl 编译缓存时，在仓库根创建未提交的 `.env.local`，优先设置精确 target 目录 `RUST_SERVER_MUSL_TARGET_DIR`：

```powershell
RUST_SERVER_MUSL_TARGET_DIR=D:\rust\shared\server-musl-target
```

Windows / Linux / macOS 发布脚本都会优先读取这个变量，并直接把它作为 `CARGO_TARGET_DIR`。旧名 `RUST_MUSL_TARGET_DIR` 仍兼容。这样 bb64a、一龙等技术栈相近的 Rust 服务端项目可以复用相同 target triple/profile/features 下的依赖编译产物。

旧的 `ELON_BUILD_TARGET_DIR` 仍然兼容：脚本会在其下创建**固定名子目录 `elon-server-musl/`**（不含 SHA）。如果两个变量都未配置，默认路径为：

| 系统 | 默认缓存路径 |
|---|---|
| Windows | `%LOCALAPPDATA%\Elon\build-target\elon-server-musl\` |
| Linux / macOS (Ubuntu Codex) | `~/.cache/elon/build/elon-server-musl/` (XDG 标准) |

两者都跨 session 持久化。**同一台机器首次全量编译约 10 分钟，后续只改业务代码约 30 秒**。CI 或远程 Codex 也可以通过进程环境变量传入 `RUST_SERVER_MUSL_TARGET_DIR` 或 `ELON_BUILD_TARGET_DIR` 指向持久卷。**禁止**把 `CARGO_TARGET_DIR` 手动设为含 SHA 的路径，那样会让每次都全量重编。

发布脚本会在编译服务端产物时强制覆盖 release rustflags 为 `-C target-cpu=x86-64`，并检查/提示全局 Cargo config 或环境变量中的 `target-cpu=native`。原因是 Windows/Linux 开发机可能有 AVX-512 等本机专有指令，不能把 `native` 优化带进要上传到服务器的二进制。

```powershell
cd scripts
.\publish-server.ps1 -SkipUpload
```

```bash
bash scripts/publish-server.sh --skip-upload
```

后端运行代码变更必须先 commit + push，再使用发布脚本部署。版本号由服务器 release API 分配，脚本会把版本号和 git SHA 注入二进制；禁止为了发布手动递增并提交 `server/Cargo.toml`。服务端通过 `/api/server/version` 暴露 `versionName` 和 `gitSha`，APK 个人页会动态显示该后端版本。

### 7.2 Rust 日常验证缓存

发布构建缓存只服务于 `publish-server.*` 的 Linux musl release 产物。日常开发验证使用单独的开发 target：

```powershell
# .env.local（不提交）
ELON_DEV_CARGO_TARGET_DIR=D:\rust\shared\elon-dev-cargo-target

# Windows
powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 check --manifest-path server\Cargo.toml
powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml pc_lightweight
```

```bash
# .env.local（不提交）
ELON_DEV_CARGO_TARGET_DIR=/var/tmp/elon-dev-cargo-target

# Linux/macOS
bash scripts/cargo-dev.sh check --manifest-path server/Cargo.toml
bash scripts/cargo-dev.sh test --manifest-path server/Cargo.toml pc_lightweight
```

`scripts/cargo-dev.*` 会读取 `.env.local`，设置 `CARGO_TARGET_DIR`，并在目标目录上加锁。这样多个 worktree / 多个 AI 可以复用同一份日常编译缓存，但不会同时写入同一个 target。版本化的 pre-push hook 也走同一个 `cargo-dev` 入口。不要并行裸跑 `cargo check` 和 `cargo test` 到同一个 `server/target` 或同一个共享 target；如确实要并行验证，必须显式指定不同 `ELON_DEV_CARGO_TARGET_DIR` / `-TargetDir` / `--target-dir`。

### 7.3 Android APK 编译打包
```powershell
cd android
./gradlew assembleRelease
# 编译产物: android/app/build/outputs/apk/release/app-release-unsigned.apk
```

### 7.4 APK 签名
```powershell
# 签名密钥从环境变量注入，不要硬编码
apksigner sign --ks $env:APK_KEYSTORE --ks-pass pass:$env:APK_KEYSTORE_PASS `
  --out app-release-signed.apk `
  android/app/build/outputs/apk/release/app-release-unsigned.apk
```

### 7.5 前端构建
```powershell
cd frontend
npm run build
# 构建产物: frontend/dist/
```

---

## Step 8：部署 + 发送结果

### 8.1 部署服务端
```powershell
cd scripts
.\publish-server.ps1
curl --noproxy '*' http://43.139.149.158:8080/health
curl --noproxy '*' http://43.139.149.158:8080/api/server/version
```

Linux/macOS 开发机使用 `bash scripts/publish-server.sh`。不得使用旧式“同步源码到服务器后远端 `cargo build --release`”流程。

### 8.2 分发 APK

Android 可安装端能力变更要先区分**代码同步**和**APK 发布**：

```powershell
# 只确认代码已经合并到远端主线
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed

# 需要交付可安装 APK 时，再单独跑发布
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

- **代码同步完成**：业务代码已 commit 并 push 到 `origin/main`。如果用户明确说“先同步代码”“发布不一定成功”或“这次不用出 APK”，到这里即可结束任务。
- **APK 发布完成**：只有当用户明确要求下载链接、线上 APK、安装包交付时，才要求 `AndroidFeature` 校验通过。

发布脚本会完成：快进到当前 `origin/main`、向服务器申请版本号、临时注入 `build.gradle`、构建 release APK、还原版本字段、上传 APK 和 `version.json`、写入 `.apk-deployed-sha`、验证服务器版本。版本号不写入 git，也不会生成 release-only commit。

APK 发布脚本必须防止慢构建覆盖新版本：构建期间如果发现服务器已部署包含本次基础提交的更新 APK，就中止本地旧编译并测试线上新版；构建完成后如果 `origin/main` 已前进且包含 Android 改动，就停止上传并汇报“代码已合并，发布交给最新主线”。脚本不得在 APK 编译完成后自动 rebase 或要求本代理重新跑旧产物。**这类发布并发中止不会否定“代码已经同步到远端主线”这一完成状态。**

### 8.3 推送结果给用户

通过 WebSocket 发送：
```json
{
  "type": "task_complete",
  "message": "已完成！你要的功能做好了。",
  "apk_url": "https://download.example.com/apk/v1.2.3/app.apk",
  "changes_summary": "在首页添加了一个红色按钮，点击后显示'你好'",
  "version": "1.2.3"
}
```

---

## Step 9：编译失败处理

```
编译失败
  │
  ▼
分析错误信息
  │
  ├── 是代码逻辑错误 ──► 修复代码 → 重新提交 → 重新编译
  │                     （最多尝试 3 次）
  │
  ├── 是依赖/配置问题 ──► 修复配置 → 重试一次
  │
  └── 无法自动修复 ──► git revert 回滚到修改前的状态
                       → 告知用户: "这个需求遇到了技术问题，正在人工处理"
```

---

## 重要约束

1. **禁止**将编译失败的代码推送到主分支
2. **禁止**在 commit 中包含 APK 签名密钥、数据库密码等敏感信息
3. **禁止**把机器本地路径、密钥、签名材料或临时构建状态写入共享说明文件
4. **每个用户任务**必须有完整的 git 提交记录，可溯源
5. **不允许**一次修改范围过大（超过5个文件应拆分为多次任务）
6. **Android 新功能禁止只交 PR 或 Debug 包**；默认先 push 到 `origin/main` 并用 `CodePushed` 校验，明确负责 APK 发布时再完成 `AndroidFeature` 闭环
7. **后端运行代码变更必须先 push 到 `origin/main`**；版本号由服务器分配，发布脚本被后续 main 超越时停止追车并汇报，明确负责发布时再校验服务器 `/api/server/version`
8. **不允许继续制造巨型文件**；新建源文件默认目标 ≤500 行，501-800 行可容忍但必须单一职责，超过 800 行必须拆分；新功能默认按职责模块化，入口文件只做组装和路由
