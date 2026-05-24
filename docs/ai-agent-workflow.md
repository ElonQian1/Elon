# AI 代理工作流 — 代码修改·编译·部署完整流程

> 本文档描述 AI 代理在接收到用户需求后，如何安全地修改代码、触发编译、部署，并将结果反馈给用户。
> AI 代理在执行任何代码操作前，必须先阅读本文档。

> **强制工作流规则**（git 提交、多 AI 并发、临时工作树部署）见：
> [.github/instructions/git-deploy-workflow.instructions.md](../.github/instructions/git-deploy-workflow.instructions.md)
> 该文件通过 `applyTo: "**"` 自动注入，AI 代理编辑任何文件时均自动生效。

---

## 项目进入规则（APK / Web / 服务器 Codex CLI 通用）

1. 每次进入项目先运行任务预检脚本：Windows 用 `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`，Linux/macOS/服务器 CLI 用 `bash scripts/ai-task-preflight.sh --create-worktree`；如果脚本创建了 worktree，必须切到 `WORKTREE_PATH` 后再观察目录结构和修改文件。
2. 如果存在 `AGENTS.md`、`CODEX.md`、`.github/copilot-instructions.md`、`.github/instructions/*.md`、`README.md` 或任务相关 `docs/`，必须先阅读，再决定修改方案。
3. `local_path` 和 GitHub 项目按已有 Git 仓库处理。修改前先 `git fetch origin main`；工作区干净才直接 `git pull --rebase origin main`，当前任务自己的未提交改动可 stash/rebase/pop，其他任务或来源不明的未提交改动必须用 `origin/main` 新建 worktree。
4. 一龙项目只是默认登记的 `local_path` 项目，不走特殊执行路径；其他 GitHub 下载或本地挂载项目也应靠自己的项目文档驱动流程。
5. Codex CLI 的长期记忆来自项目文件，不来自服务器进程本身。流程变化必须写回文档并提交。
6. Android APK 新功能的完成定义是“手机可安装到最新 APK”，不是 PR、分支、Debug 包或代码已提交。
7. 手机触发的项目开发流程中，后端预检错误只作为上下文交给 CLI；CLI 应先自查 Git 现场并尝试安全处理，只有判断无法克服时才向用户说明并暂停。

---

## 项目任务并发规则（APK / 服务器通用）

1. 服务器以 `project_id` 为单位分配项目执行权：不同项目可以并行，同一项目当前必须排队执行。
2. 这不是取消 `worktree` 并行，而是保护还在共享工作区中的 Git 同步、文件修改、commit、push。
3. 同项目多任务要做到真正并行时，应为每个任务创建独立分支和 worktree；任务完成后再通过受控 merge 进入主分支。
4. 无论是否使用 task worktree，merge 到 `main`、Android 版本号递增、APK 发布、服务器部署、数据库任务状态落库都必须串行。
5. 一龙自项目与普通 GitHub / `local_path` 项目遵守同一套规则，不允许隐藏特殊流程。

---

## 后端与 Codex CLI 协作边界

1. 后端是流程指挥官，Codex CLI 是代码执行者。
2. 后端调用 Codex CLI 前，必须先确认项目身份、工作区路径、Git/origin、权限、队列/锁状态和用户选择的模型。
3. 后端每次都给 Codex CLI 注入任务单：用户需求、项目路径、必须读取的文档顺序、Git 规则、验证要求、共享发布动作必须串行。
4. 以后即使接入其他 AI 模型，它们也只能作为旁路工具做轻量分类、摘要、图片/特殊分析或检索增强；旁路结论必须由后端整理后回灌到当前 APK 会话绑定的 Codex CLI 原生 session，不能另起长期主会话。
5. Codex CLI 不能依赖跨任务记忆；未知项目必须先读 `AGENTS.md`、`CODEX.md`、`README.md`、`.github/instructions` 和相关 `docs/`。
6. Codex CLI 完成后，后端负责验收和产品化状态：任务记录、进度展示、下载链接、版本信息、合并/发布/部署锁。
7. 并发安全、版本顺序、APK 发布、服务器部署不能只靠提示词，必须由后端代码和发布脚本强制执行。
8. 后端不能因为 `git pull --rebase` 的业务性失败直接终止开发任务；应把失败原因注入 CLI 任务单，让 CLI 优先自愈。只有 CLI 启动失败、超时、IO 异常这类平台问题，才由后端直接失败或切换 fallback。

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

---

## Step 7：触发编译流水线

### 7.1 Rust 服务端本地交叉编译

服务端发布必须使用仓库脚本在本地开发机交叉编译 Linux binary，再上传到服务器。桌面版 Codex 就是在本机编译；生产服务器只负责接收 binary、替换、重启和健康检查，不承担编译。

```powershell
cd scripts
.\publish-server.ps1 -SkipUpload
# 本地交叉编译产物: server/target/x86_64-unknown-linux-musl/release/elon-server
```

```bash
bash scripts/publish-server.sh --skip-upload
```

后端运行代码变更必须先递增 `server/Cargo.toml` 的 `package.version`。PATCH 用于修复，MINOR 用于向后兼容的新功能，MAJOR 用于不兼容 API / 协议变更。部署脚本会把 git SHA 注入二进制，服务端通过 `/api/server/version` 暴露 `versionName` 和 `gitSha`，APK 个人页会动态显示该后端版本。

### 7.2 Android APK 编译打包
```powershell
cd android
./gradlew assembleRelease
# 编译产物: android/app/build/outputs/apk/release/app-release-unsigned.apk
```

### 7.3 APK 签名
```powershell
# 签名密钥从环境变量注入，不要硬编码
apksigner sign --ks $env:APK_KEYSTORE --ks-pass pass:$env:APK_KEYSTORE_PASS `
  --out app-release-signed.apk `
  android/app/build/outputs/apk/release/app-release-unsigned.apk
```

### 7.4 前端构建
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

Android 可安装端能力变更必须使用仓库发布脚本，不得手工拼接版本号、签名、上传步骤：

```powershell
scripts\publish-apk.ps1 -Changelog "<本次用户可见改动>"
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind AndroidFeature
```

发布脚本会完成：同步 `main`、递增 `versionCode/versionName`、构建 release APK、提交 release commit、推送 `HEAD:main`、上传 APK 和 `version.json`、验证服务器版本。

### 8.3 推送结果给用户

通过 WebSocket 发送：
```json
{
  "type": "task_complete",
  "message": "已完成！你要的功能做好了。",
  "apk_download_url": "https://download.example.com/apk/v1.2.3/app.apk",
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
3. **必须**在每次代码修改后更新 `copilot-instructions.md` 中的"当前开发状态"
4. **每个用户任务**必须有完整的 git 提交记录，可溯源
5. **不允许**一次修改范围过大（超过5个文件应拆分为多次任务）
6. **Android 新功能禁止只交 PR 或 Debug 包**；默认必须完成 APK 发布闭环并校验服务器 `version.json`
7. **后端运行代码变更必须递增服务端版本号**；默认必须部署后校验服务器 `/api/server/version`
