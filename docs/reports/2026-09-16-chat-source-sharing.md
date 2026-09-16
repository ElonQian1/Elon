# 外部分享与图片原文链接交付证据

本文件仅记录本次交付结果，不定义新需求。产品协议见 [外部分享与图片原文链接](../chat-source-sharing.md)。

## 范围与归属

本任务负责 Android `sharing/`、PC `source-links/`、移动网页 `social_source_*` 和附件来源协议；复用现有账号、好友/群聊、附件上传和聊天 JSON 存储。通知声音、文章发布平台、币安接入和 HTTPS 配置不在本次改动中。

用户截图作为交互参考，不作为像素拟合目标。验收使用合成二维码和本地模拟会话，没有给真实好友或群发送测试消息，没有操作微信账号会话或安装到物理手机。

## 能力矩阵

| 能力与模块 owner | implementation_status | verification_status | delivery_status | acceptance_status |
| --- | --- | --- | --- | --- |
| Android 系统分享、登录恢复、预览后选会话发送；本任务 `sharing/` | implemented | offline_verified：导入/草稿/HTML 合同；正式 APK 编译和 lintVital 通过 | published | deferred：具体手机和微信菜单 |
| 发送端本地识别、多二维码选择；本任务 Android/PC/PWA | implemented | offline_verified：Android 真解码及浏览器实际 Worker | published | synthetic_flow_verified |
| 图片来源链接存储、旧消息兼容；本任务 Rust 协议 | implemented | offline_verified：真实存储 JSON 往返及旧字段缺省 | published | contract_verified |
| 图片下方链接、手动扫码和 URL 消息入口；本任务三端 UI | implemented | offline_verified：PC/PWA 实际组件；Android 编译与单测 | published | Android_visual_deferred |
| APK 独立阅读页与浏览器回退；本任务 `ArticleLinkActivity` | implemented | offline_verified：构建及源码检查 | published | deferred：真实公众号页面、WebView/OEM |

来源字段是客户端提供的 URL 信息，不是可信认证。服务器不抓取二维码目标或保存文章全文。普通 URL 仅增加少量 JSON，接收端不会自动扫码或自动打开链接。暂未实现服务器消息幂等键，发送结果不确定时禁止自动重发，要求到聊天核对。

## 验证证据

- `scripts/test-chat-source-android.ps1`：3 个 suite、7 个测试，failures/errors/skipped 均为 0；最终日志 `share-android-atomic`。检查实际 JUnit XML，不依赖可能遗漏失败退出码的 Gradle 启动脚本。
- Rust `attachment` 筛选：12 passed、0 failed；最终日志 `share-rust-final`。包括来源 URL 校验、查询参数保留、真实聊天附件 JSON 往返以及已有附件回归。
- `scripts/test-social-source-links.cjs`：PC/PWA 来源合同通过。
- `scripts/test-social-source-browser.cjs`：实际 PC 组件和 PWA 模块、Worker 双二维码、链接选择、确认后单次模拟发送通过；最终日志 `share-source-browser-final`。截图人工检查无遮挡。
- 既有 `test-mobile-social-recovery.cjs`：11 项通过；`test-mobile-social-browser.cjs`、`test-pc-social-chat-parity.cjs` 均通过。
- PC `npm run build` 通过，日志 `share-pc-final`。
- 源码大小、文档模块化、Rust 格式和 Git diff 检查通过。新增第三方 jsQR 1.4.0 保留 Apache 2.0 许可证；二维码 fixture 不包含用户数据。

## 发布与线上核验

- 功能提交：`a6ae8bbb7ef7022754ebfc759ab0671bfb09e60d`，已推送 `origin/main`。
- 本任务服务端发布脚本完成并返回成功：`0.3.1762`，日志 `share-publish-server`。四个新增静态脚本经 HTTP 下载核对与发布提交一致。
- 随后主线 `c9748fe3903ad124fe76b9f544b03384357fd2e1` 合入独立任务的跨端聊天链接卡片，包含本次提交，且仅在来源文本卡片入口增加去重条件。本任务补跑 `test-social-link-cards.mjs` 通过，未覆盖该任务实现。
- 2026-09-16 14:55 重新核验：服务端 `0.3.1763`、PC 前端 `server_bundle` 均为 `c9748fe3`；`check-task-complete.ps1 -Kind PcFrontend` 通过，`/health=OK`、`/pc=HTTP 200`。
- APK `1.1.1773`（build 1773），发布提交 `c9748fe3`；`check-task-complete.ps1 -Kind AndroidFeature` 通过，`APK_PROVENANCE_STATUS=exact_task_commit`。
- APK 文件 40,774,714 字节；服务器实物 SHA-256 与 `/app/version.json` 一致：`6db21db077f2303a70e8f89a010e0b35ee050c173ce3ed9dc1a6762fb2802d7e`。
- 本任务 APK 编译与 lintVital 通过后，发布请求遇到 `token-not-active`；独立并发发布已交付同一后代提交，因线上覆盖检查通过而没有重复发布。此处区分本次命令失败与最终工件已经上线。
- 下载入口：`http://43.139.149.158:8080/app/download`。本次没有发布 Windows 节点程序；Windows 聊天网页通过 PC 前端发布交付。

## UI 验收边界

- UI 工作台任务：`desktop_882627f7153f4b289e7beed6d816c358`。
- 三张参考截图已导入。`TARGET_DESIGN` 未提供，因此没有像素损失目标；本地浏览器组件截图不冒充真实 Android 截图或跨端视觉一致性证明。
- 工作台浏览器捕获超时。发布后一次模拟器验证（`preferEmulator=true`、`isolatedEmulatorPackage=true`）也在工具预算内超时，未重试物理设备。
- `ui_check_workflow_completion` 返回 `BLOCKED`、`PATCH_FREE_BUILD_VERIFY=PREPARATION_REQUIRED`、`DEBUG_RUNTIME_NOT_CONNECTED`；无缺失平台能力、无进化任务。此为视觉验收延期，不否定已经核验的仓库发布。
- `FIT_RUN_STATUS=VERIFICATION_DEFERRED`
- `FINAL_VISUAL_LOSS=not_measured`
- `VISUAL_ACCEPTANCE_THRESHOLD=not_applicable_without_target_design`
- `CROSS_PLATFORM_VISUAL_PARITY=not_verified`
- `BUSINESS_DELIVERY_READY=false`（UI 工作台门禁，原因仅为 Runtime 未连接；仓库发布另行核验）
- `PLATFORM_EVOLUTION_PENDING=false`
- `EVOLUTION_THREAD=none`

下一次手机验收入口：在浏览器分享公众号 URL 到“发送到一龙”，预览并选择测试会话；再分享一张含网页二维码的图片，核对图片下方链接和阅读页。微信仅在调用 Android 系统分享时能列出本应用，其私有菜单不受本应用控制。

## 仓库与检索记录

所有业务改动在预检建立的隔离任务树完成。功能登记 MCP 在当前会话不可调用，未手改注册表。主 checkout 有既存已跟踪修改，未改动、暂存或清除该现场；未知文件保持原样。最终仓库收尾状态以 `finish-ai-task.ps1` 回执为准。

只按入口、Git/发布、模块化、PC/PWA、Android UI/WebView、构建缓存和交付主题读取命中文档；未全读 docs、Prompt、Agent、Skill 或历史讨论。本证据报告不会改变正式规范的权威性。
