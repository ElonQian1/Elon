# 一龙项目 AI 共享契约

先读 `AGENTS.md` 与 `AI_CURRENT.md`。

## 移动设计 V2

`MOBILE_DESIGN_SYSTEM=2.0`
`APP_UI_SYSTEM_REFACTOR_POLICY=runtime-evidence-before-release`

Android/PWA 按 `docs/design/mobile-design-system-v2.md` 与 `docs/Design.md`。旧皮肤/截图标准退役，采用 Material 3，允许渐进 View 迁移。系统级主题/组件/导航/框架及整页导入，必须先内部构建与原生运行证据再发布；缺证据可提交待审，不能发布或称视觉通过。覆盖旧快速通道/模板冲突；业务与工程合同不变。双端语义对齐，平台适配无需空改 HTML。

## 项目边界

本仓库属于一龙自项目；独立用户子项目不得使用本仓库的发布脚本。

Codex、Copilot、Claude 的 Win 无人值守更新/接管统一按 [执行入口](../docs/win-conversation-unattended-acceptance.md)；已有任务授权后直接执行，不逐步重复询问。

- 一龙 Android 发布只走 `scripts/publish-apk.*`，不能用 Debug 包代替可安装端发布。
- 后端发布只走 `scripts/publish-server.*`；Win 节点发布只走 `scripts/publish-node-agent.ps1`。
- 发布版本由服务器 claim/finish 分配，不手改并提交 `server/Cargo.toml` 或 `build.gradle` 版本号。

## 强制任务生命周期

其他 Prompt、Agent、Skill 只引用以下编号，不重复步骤。

| 编号 | 必须满足 |
|---|---|
| `WF-START` | Windows：`powershell -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`；Linux/macOS：`bash scripts/ai-task-preflight.sh --create-worktree`。同步失败必须停止；`LOCAL_MAIN_RECOVERY_REQUIRED=true` 按 Git 手册恢复，不把残留自动当成未提交业务。 |
| `WF-EDIT` | 只在脚本输出的 `EDIT_ROOT` 修改、格式化、验证、提交。`main` checkout 只做同步基线。`EDIT_ROOT=BLOCKED_CREATE_WORKTREE_FIRST` 时禁止编辑。 |
| `WF-DEDUP` | 记 `TASK_BASE_SHA`；写模块前按手册查重。重叠转审查，仅补不相交缺口；同符号或边界不清则停止；不提前 rebase。 |
| `WF-FILES` | 有意创建的源码、测试、fixture 必须提交；一次性产物写入 `.ai-tmp/`；稳定且可重复生成的输出才添加精确 `.gitignore`；来源不明文件不提交、不忽略、不删除。 |
| `WF-VERIFY` | 运行与风险匹配的最小验证。Rust/Cargo、格式化、Android、PC 前端等命令按 `AGENTS.md` 路由读取，不自行绕过项目脚本。 |
| `WF-PUSH` | 只 stage 任务文件；作者名 = 本机 `git user.name` 追加 ` Claude ai助手`，用 `git -c user.name="<user.name> Claude ai助手" commit` 单次指定，不写入 config（Codex CLI/桌面端后缀见 `CODEX.md`）；标题不写用户姓名、ID、需求原文；立即 `git push origin HEAD:main`；提交前查未跟踪文件。 |
| `WF-REBASE` | 仅 non-fast-forward 后重新 fetch 并 rebase/retry；命中上游才补受影响验证，影响不明才全量验证。 |
| `WF-FINISH` | Windows：`powershell -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -File scripts\finish-ai-task.ps1 -Kind <Kind>`；Linux/macOS：`bash scripts/finish-ai-task.sh --kind <Kind>`。 |
| `WF-REPORT` | 只有收尾输出 `FINALIZABLE=true` 才能正常宣告完成。最终回复必须分别报告 `BUSINESS_STATUS`、`LOCAL_MAIN_STATUS`、`TASK_WORKTREE_STATUS` 和未跟踪文件告警。 |

预检输出的 `NEXT=`、`EDIT_ROOT=`、`FINISH_COMMAND_*=`，以及收尾输出的 `FINALIZABLE=`，优先级高于文档示例。

预检 lock 活跃 `codex/*` worktree；收尾负责 unlock 和定向清理。
Windows 隔离 worktree 默认放在当前仓库盘符的 `\wt\<短ID>`；机器级覆盖使用绝对路径 `ELON_AI_WORKTREE_ROOT`。

### 平台会话 worktree 例外

位于 `conversation-worktrees/<project>/<conversation>` 或 `ai/session/<project>/<conversation>` 分支时已由平台隔离，不建嵌套 worktree；仍遵守 `WF-FILES` 至 `WF-REPORT`，由 `cleanup-task-worktrees.*` 回收。

## 文件处置契约

| 文件 | 决策 |
|---|---|
| 本次功能需要的测试源码、fixture、脚本 | 作为交付物提交 |
| 构建缓存、日志、临时截图、一次性诊断 | 优先写到仓库外；必须在仓库时写入 `.ai-tmp/` |
| 工具每次都会在固定路径生成的非源码输出 | 添加路径精确、可验证的 `.gitignore` 规则并提交 |
| 任务开始前已存在或归属不明 | 保留并报告，绝不自动 stage、stash、删除或忽略 |

不得仅因文件名含 `test` 而删除或忽略；分类见 `.ai/workspace-policy.txt`。

## 完成类型

| 改动类型 | 发布动作 | `WF-FINISH` Kind |
|---|---|---|
| 文档、配置或只要求代码同步 | 不发布 | `DocsOnly` 或 `CodePushed` |
| 后端运行代码 | 默认运行 `publish-server.*`，除非用户明确只同步代码 | `Server`；只同步时 `CodePushed` |
| PC 前端 | 纯前端用 `publish-pc-frontend.ps1`；含 API 用 `publish-server.*` | `PcFrontend`；只同步用 `CodePushed` |
| Win 节点客户端用户可见改动 | 默认 `publish-node-agent.ps1` | `NodeAgent`；只同步时 `CodePushed` |
| Android 可安装端用户可见改动 | 默认 `publish-apk.*`；系统级 UI 先满足 V2 运行证据 | `AndroidFeature`；只同步时 `CodePushed` |
| Android + 移动 PWA 低风险视觉同步 | `publish-app-ui-fast-lane.ps1`；系统级重构除外 | `AndroidFeature` |

APK 发布后按 `docs/apk-debug-device-delivery.md` 读取主项目手机档案，探测 USB/无线 ADB/mDNS、核对硬件、保留数据 `adb install -r`、不降级并回读版本/摘要，同一硬件只更新一次。离线记延期；在线安装、档案或 ADB 失败必须报失败，不用视觉延期掩盖；本机配置只能覆盖，不能省略检测。

普通微调仅按 `docs/app-ui-fast-lane.md` 使用 `APP_UI_RELEASE_POLICY=publish_before_optional_renderer`；系统级重构不适用。默认无需物理设备视觉检查；用户要求或反馈不正确时才用 `realDeviceRequired=true`，单会话准备最多 30 秒，失败记 `VERIFICATION_DEFERRED`。无真帧不称视觉通过，发布后装机义务不变。

发布期间主线前进：未构建的旧 Android 候选让位；已验证 APK 若仍是主线祖先且线上无更新后代，可先发布。发布类型互不阻塞，失联由短租约回收。业务已入主线的结论不变，不循环 rebase 或重跑旧构建。

## 其他硬边界

- 不泄露或提交 `.env`、签名密钥、token、密码和本机私有路径。
- 不回退、覆盖或夹带其他代理的改动；不能确认归属时停止处置该文件。
- Rust/Cargo 验证必须走 `scripts/validate-rust.ps1`（Git Bash/非 Windows 由 `cargo-dev.sh` 适配）；入口先执行廉价门禁，再按精确指纹复用或运行 `cargo-dev`。发布构建走发布脚本，不能共享裸 Cargo 写入。
- 经仓库脚本确认的全量纯 rustfmt 先独立提交；业务改动另提，不为缩小 diff 反复撤销。
- 新建源文件目标不超过 500 行，超过 800 行必须拆分；入口文件只做组装。
- APP 纯视觉微调读 `docs/app-ui-fast-lane.md`；复杂 UI 先读 V2 及产品规则。
- 带 `#requires -Version 7.0` 的脚本必须用 `pwsh`，不能删要求或降级脚本来绕过。
