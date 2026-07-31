# APP 低风险视觉修改快速通道

适用于 Ripple、颜色、间距、圆角、字号和轻量动画等纯视觉调整。目标是让 Android 与移动 PWA 同步交付，同时避免把真机、Renderer 和 PC 前端引入简单任务。

## 进入条件

必须同时满足：

- 不改业务状态、导航、数据、权限、手势语义或无障碍语义。
- 不依赖 OEM 系统栏、键盘、拖拽手感、相机、麦克风、蓝牙、NFC、生物识别或真实安装权限。
- Android 改动存在移动网页对应效果，并在同一 commit 修改 `server/src/assets/web_page.html`。
- 能从源码审查清楚限定改动范围；不确定时退出快速通道，按 `docs/Design.md` 和完整 UI 流程处理。

## 默认流程

1. 两端一起修改：Android 源码 + `server/src/assets/web_page.html`。
2. 能稳定表达规则时写一个小型契约测试，例如“可点击图标不再使用无边界 Ripple”；只改无法稳定断言的颜色数值时，记录不适用原因，不为测试而复制实现。
3. 运行一次快速验证：

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\invoke-ai-logged-command.ps1 `
     -LogName app-ui-validate -WorkingDirectory . `
     -CommandLine "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-app-ui-fast-lane.ps1 -ContractTest com.elon.app.BorderlessRippleContractTest"
   ```

   没有合适契约时：

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\invoke-ai-logged-command.ps1 `
     -LogName app-ui-validate -WorkingDirectory . `
     -CommandLine 'powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-app-ui-fast-lane.ps1 -NoContractReason "仅调整视觉数值，没有稳定的行为契约"'
   ```

   脚本先执行可选契约测试，再并行运行 Android `:app:assembleDebug` 与移动 PWA 源码语法检查；不构建无关的 `pc-frontend`。
4. 审查 diff、提交并立即 push。默认不启动 ADB、真机、模拟器、Renderer、截图、FitRun，也不重复安装 Debug APK。
   只有用户反馈本轮修改不正确或明确要求真机复核时，才退出快速通道并启用一次性真机流程。
5. 推送后立即运行：

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\invoke-ai-logged-command.ps1 `
     -LogName app-ui-publish -WorkingDirectory . `
     -CommandLine 'powershell -NoProfile -ExecutionPolicy Bypass -File scripts\publish-app-ui-fast-lane.ps1 -Changelog "同步说明"'
   ```

   包装脚本先比较线上 Server SHA 与当前 HEAD 的改动范围，再发布移动 PWA，最后发布 APK：

   - 只有 `server/src/assets/web_page.html` 变化时，原子上传运行时模板，不重新编译 Rust，也不构建 PC 前端。
   - 后端 Rust、其他内嵌资源或 PC 前端变化时，才运行完整 Server 发布；APP UI 路线使用 `-SkipPcFrontend` 避免重复构建无关 PC 页面。
   - 没有移动 PWA 变化时记录 skipped，不启动 Server 发布。
   - APK 发布始终先做线上 Android 构建输入覆盖检查；已覆盖时不 claim 版本、不重复构建。

   每个阶段写入 `.ai-tmp/release-receipts/`，相同源码 SHA 的重试可以复用已经完成的移动 PWA 阶段。两个正式发布仍按顺序执行，不并行抢占。
6. 发布完成后，有空闲 Renderer 时再做针对性视觉补验。任何 start/bootstrap/prepare 前只调用一次 `ui_get_runtime_status` 和 `ui_check_capabilities` 检查明确的 `rendererResourceId`/lease；全部占用时立即记录 `VERIFICATION_DEFERRED=renderer_capacity_unavailable`、`RENDERER_PREPARATION_ATTEMPTS=0`，不得发起准备或重试。存在明确空闲槽时才允许一次最多 30 秒的准备；忙碌、离线或超时不得阻塞 Server/PWA、APK 发布或统一收尾。
7. 用 `AndroidFeature` 执行统一收尾，并分别报告“业务已发布”和“视觉已验收 / 验证延期”，不得把两种状态混为一谈。

## 发布与视觉验收分离

- 固定顺序为 `push → publish Server/PWA → publish APK → optional Renderer verification`。
- 源码、契约、构建或发布失败可以阻塞交付；共享 Renderer 忙碌、无空闲模拟器、真机离线和 Runtime 准备超时不能阻塞普通视觉任务发布。
- Renderer 容量检查必须早于启动或准备；零空闲槽直接延期，禁止以五次超时轮询代替资源检查。
- 只有用户明确要求“验收通过后再发布”，或 OEM、权限、软键盘、Launcher、硬件、传感器、性能等必须设置 `realDeviceRequired=true` 的专项，才把真实设备验收作为发布前置条件。
- 没有真帧证据时允许报告“已发布、验证延期”，但禁止报告“视觉已验收”或伪造视觉损失值。

## 时间与可靠性约束

- Gradle 快速验证和 Release 构建使用 `--no-daemon`，避免 daemon 持有日志句柄导致包装器无法退出。
- `invoke-ai-logged-command.ps1` 支持 `-TimeoutSeconds`；超时返回 124，并终止对应任务的完整子进程树。
- SSH/SCP 固定为非交互模式，具有连接、保活和阶段硬超时；单条远程命令不得无限等待。
- APK 上传前计算 SHA-256 并写入 `version.json`；服务器 staging、原子切换后都在原地校验哈希和文件大小，不重新下载完整 APK。
- APK 已完成原子发布和哈希验证后立即调用 release/finish；广播或 HTTP 后置检查失败只记 warning，不能遗留 in-flight 租约。
- 发布输出必须包含 `RELEASE_STAGE=<name> status=<status> durationSeconds=<n>`，用于定位真实慢阶段。

## 退出快速通道

出现编译/契约失败、两端无法同步、截图问题无法从源码定位，或用户反馈本轮修改不正确时，升级为针对性视觉验证。普通视觉任务仍先发布再补验；只有用户反馈修改不正确或明确要求时才允许占用真机，同一 MCP 会话只准备一次，失败即延期。升级后只验证受影响页面，不为最新主线重复第二次 Debug 真机安装。

