# 一龙项目 AI 共享契约

最后更新：2026-07-18

> 本文件是 Codex、Codex CLI、Copilot、Copilot CLI 等代理的共享权威规则。
> 它只保留每轮都必须知道的不变量；任务专项细节由 `AGENTS.md` 路由按需读取。
> 开始任务时还要读取 `AGENTS.md`，只选择当前任务命中的专项文档。

## 项目边界

一龙是云端 APK 开发平台。本仓库中的 `android/`、`server/`、`pc-frontend/`、`scripts/` 都属于一龙自项目；用户子项目位于独立项目目录，不能套用一龙自身的发布脚本。

- 一龙 Android 发布只走 `scripts/publish-apk.*`，不能用 Debug 包代替可安装端发布。
- 后端发布只走 `scripts/publish-server.*`；Win 节点发布只走 `scripts/publish-node-agent.ps1`。
- 发布版本由服务器 claim/finish 分配，不手改并提交 `server/Cargo.toml` 或 `build.gradle` 版本号。

## 强制任务生命周期

以下规则使用稳定编号。其他 Prompt、Agent、Skill 和长文档只能引用这些编号，不重复整段步骤。

| 编号 | 必须满足 |
|---|---|
| `WF-START` | 任何写任务先运行预检：Windows `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`；Linux/macOS `bash scripts/ai-task-preflight.sh --create-worktree`。 |
| `WF-EDIT` | 只在脚本输出的 `EDIT_ROOT` 修改、格式化、验证、提交。`main` checkout 只做同步基线。`EDIT_ROOT=BLOCKED_CREATE_WORKTREE_FIRST` 时禁止编辑。 |
| `WF-FILES` | 有意创建的源码、测试、fixture 必须提交；一次性产物写入 `.ai-tmp/`；稳定且可重复生成的输出才添加精确 `.gitignore`；来源不明文件不提交、不忽略、不删除。 |
| `WF-VERIFY` | 运行与风险匹配的最小验证。Rust/Cargo、格式化、Android、PC 前端等命令按 `AGENTS.md` 路由读取，不自行绕过项目脚本。 |
| `WF-PUSH` | 只 stage 当前任务文件；提交信息用中文并包含用户标识；commit 后立即 `git push origin HEAD:main`。提交前检查未跟踪文件，防止漏交新源码或测试。 |
| `WF-REBASE` | 只有 push 被 non-fast-forward 拒绝时，才 `git fetch origin`、`git rebase origin/main`、解决冲突后重推。远端前进本身不触发追车。变基不自动触发重新编译或全量测试：先审查上游差异与冲突；无冲突且未命中本任务代码、构建输入或测试基础设施时复用变基前验证，直接重推；命中时只重跑受影响的最小验证。只有发布脚本、明确项目门禁或无法限定影响面的共享底层变更要求全量验证。 |
| `WF-FINISH` | 修改任务只能用统一收尾命令结束：Windows `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\finish-ai-task.ps1 -Kind <Kind>`；Linux/macOS `bash scripts/finish-ai-task.sh --kind <Kind>`。 |
| `WF-REPORT` | 只有收尾输出 `FINALIZABLE=true` 才能正常宣告完成。最终回复必须分别报告 `BUSINESS_STATUS`、`LOCAL_MAIN_STATUS`、`TASK_WORKTREE_STATUS` 和未跟踪文件告警。 |

预检输出的 `NEXT=`、`EDIT_ROOT=`、`FINISH_COMMAND_*=`，以及收尾输出的 `FINALIZABLE=`，优先级高于文档示例。

预检会用 Git worktree lock 标记活跃的 `codex/*` 任务；全局清理不得回收 locked worktree，统一收尾在定向删除前负责 unlock。这样长时间构建/发布不会因超过年龄保护而被并发清理。

### 平台会话 worktree 例外

当前目录位于 `conversation-worktrees/<project>/<conversation>`，或分支为 `ai/session/<project>/<conversation>` 时，平台已经完成隔离；不要创建嵌套 worktree。仍需遵守 `WF-FILES` 至 `WF-REPORT`；平台会话 worktree 由 `cleanup-task-worktrees.*` 在 clean、已合入 `origin/main`、超过年龄保护且非当前 worktree 时回收。

## 文件处置契约

| 文件 | 决策 |
|---|---|
| 本次功能需要的测试源码、fixture、脚本 | 作为交付物提交 |
| 构建缓存、日志、临时截图、一次性诊断 | 优先写到仓库外；必须在仓库时写入 `.ai-tmp/` |
| 工具每次都会在固定路径生成的非源码输出 | 添加路径精确、可验证的 `.gitignore` 规则并提交 |
| 任务开始前已存在或归属不明 | 保留并报告，绝不自动 stage、stash、删除或忽略 |

文件名包含 `test` 不能作为删除或忽略依据。机器分类策略位于 `.ai/workspace-policy.txt`，代理无需把它全文读入上下文。

## 完成类型

| 改动类型 | 发布动作 | `WF-FINISH` Kind |
|---|---|---|
| 文档、配置或只要求代码同步 | 不发布 | `DocsOnly` 或 `CodePushed` |
| 后端运行代码 | 默认运行 `publish-server.*`，除非用户明确只同步代码 | `Server`；只同步时 `CodePushed` |
| `pc-frontend/`、`/pc` 用户可见改动 | 构建后默认 `publish-server.*` 并验证 `/pc` | `PcFrontend`；只同步时 `CodePushed` |
| Win 节点客户端用户可见改动 | 默认 `publish-node-agent.ps1` | `NodeAgent`；只同步时 `CodePushed` |
| Android 可安装端用户可见改动 | 默认 `publish-apk.*` | `AndroidFeature`；只同步时 `CodePushed` |

发布期间若被更新的 `origin/main` 或服务器版本超越，业务代码已进入主线的结论不变；按脚本输出汇报“代码已合并，发布交给最新主线”，不要循环 rebase 或重跑。

## 其他硬边界

- 不泄露或提交 `.env`、签名密钥、token、密码和本机私有路径。
- 不回退、覆盖或夹带其他代理的改动；不能确认归属时停止处置该文件。
- Rust/Cargo 验证必须走 `scripts/validate-rust.ps1`（Git Bash/非 Windows 由 `cargo-dev.sh` 适配）；入口先执行廉价门禁，再按精确指纹复用或运行 `cargo-dev`。发布构建走发布脚本，不能共享裸 Cargo 写入。
- 经仓库脚本确认的全量纯 rustfmt 先独立提交；业务改动另提，不为缩小 diff 反复撤销。
- 新建源文件目标不超过 500 行，超过 800 行必须拆分；入口文件只做组装。
- APP UI 改动先读 `docs/Design.md`、`docs/APP 颜色规范.md`；APK UI 还要按路由检查网页同步规则。
- 带 `#requires -Version 7.0` 的脚本必须用 `pwsh`，不能删要求或降级脚本来绕过。

所有专项文档、验证命令和发布细节从 `AGENTS.md` 路由，不固定全量读取。
