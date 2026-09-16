---
version_status: current
reviewed_at: 2026-09-16
---

# 六平台聊天卡片完善验收

范围见 [需求](win-social-card-polish.md)。基线已包含上一轮整卡点击和紧凑呈现；本轮在此基础上完善，不重写聊天存储或消息同步。

| 能力 | 实现与证据 | 验收边界 |
|---|---|---|
| 六平台呈现 | 共享 JS/CSS 与 Android `SocialLinkPresentation`；标题最多三行，来源同时保留平台、作者；56 dp/px 真实封面或文字平台标识 | 无标题/封面时保留明确降级，不虚构文章资料 |
| 币安 App 分享 | 三端及服务端精确识别 `app.binance.com`；原查询参数不改写；仿冒域名测试 | 不保证原站免登录，也不放宽逐跳公网 HTTPS 检查 |
| 抖音分享文案 | 从“作者的作品”后的文字提取标题，同时保留作者 | 只影响卡片，不修改发送原文 |
| B 站起播 | 三端显示 `01:20`、`1:01:20`；播放器仍保留 `t`、`p` | 播放器参数合同通过，真实外站播放未验收 |
| Windows 右键 | 生产 `SocialConversation` 菜单加入打开链接、复制链接；悬停/焦点显示更多按钮 | 复制整条消息、引用、草稿仍保留；没有修改消息权限规则 |
| APK 手势 | 标题、来源、封面、占位标识、起播时间均绑定相同点击；长按、多选测试 | Robolectric View 测试不等于真机视觉证明 |

可重放验证入口：

- `node scripts/test-social-link-cards.mjs`：URL、分享文案、缓存/异步回调、降级、原文保留合同。
- `scripts/validate-rust.ps1 -- test --manifest-path server/tests/social-link-preview-harness/Cargo.toml`：真实服务模块的预览与安全边界测试。
- Android `:app:testDebugUnitTest --tests com.elon.app.sociallinks.*`：15 个 native View/策略/绑定测试。
- `npm run build`（`pc-frontend`）：TypeScript 检查和正式构建。
- `pc-frontend/scripts/test-social-cards-browser.mjs`：实际 React 聊天、共享 PWA 卡片；元数据可用/不可用、损坏封面、右键复制、键盘打开/引用、窄窗及 150% 缩放、更新重绘和草稿保留。先用项目 `start-pc-frontend-dev.ps1 -Port 5196 -Foreground`，再运行脚本；依赖可由 `PLAYWRIGHT_MODULE_PATH`、`CARD_BROWSER` 提供，产物目录用 `CARD_EVIDENCE_DIR`。fixture 只在开发服务器可用，不进入正式构建。

所有 fixture 消息为合成数据，浏览器用元数据响应及 IPC 替身，禁止真实群聊写入。IPC 只验证打开/关闭参数，不能声称真实 Windows 宿主已打开。当前 Win 语义桥显示没有在线宿主。APK 更新后反馈按 UI 工作流触发一次干净提交后的真机准备；无真实帧时视觉验收延期，不制造截图或将 PWA 当成 APK。

发布使用项目 Server（包含 PC）与 APK 发布脚本，版本与源码身份以发布回执为准。功能登记停留在 implemented，直到真实设备与跨端视觉证据补齐；源码、组件测试、正式发布分别报告。

## 本轮交付回执

- 源码：`f18e9b309`（共享及 APK 卡片）和 `7a9caffb91a2f1b8ff73c8173750e6b016448076`（桌面动作、组件验收），均已推送主线。
- Server/PWA/PC：`0.3.1766`；健康接口 `OK`，服务端与 `/pc/assets/release.json` 均为上述完整源码 SHA；线上 `social_links.js/css` 与源码归一化换行后逐字一致。
- APK：`1.1.1777` / build `1777`，40,779,778 字节，SHA-256 `82bce2d0d8a597b8f21c49dbdd633a1308f1d8959d7a6188a396750d1e8b1c0f`，线上源码、文件大小、哈希均与本地发布工件一致。
- JS、Rust 预览 harness、Android 15 项测试、PC 类型检查/正式构建、两种元数据场景的实际 React/PWA 浏览器验收通过。PWA MCP 获取 390×844 实际 PNG，SHA-256 `d3f1fc867820df3b0f45218c584874a47b459ece1c841c165298e437fc7bda30`，绑定源码 `7a9caffb91a2f1b8ff73c8173750e6b016448076` 与测试路由。
- 首次服务端发布被日志静默 600 秒门限中止。确认原构建进程退出后，通过原发布入口以 1800 秒静默/3600 秒总时限恢复；成功回执耗时 699 秒，没有改功能代码、跳过构建或手工上传。
- 真机仅准备一次，返回 `RENDERER_CAPACITY_UNAVAILABLE`，没有可用设备；`REAL_DEVICE_STATUS=REQUIRED_FOLLOWUP`，`ANDROID_RENDERER=UNAVAILABLE`。没有尝试修复 ADB、重建会话或以模拟器冒充真机。
- `FIT_RUN_STATUS=NOT_RUN`，`FINAL_VISUAL_LOSS=NOT_MEASURED`，`VISUAL_ACCEPTANCE_THRESHOLD=NOT_PROVIDED`，`CROSS_PLATFORM_VISUAL_PARITY=VERIFICATION_DEFERRED`。UI 门禁 `BUSINESS_DELIVERY_READY=false`（未取得真实 Android 帧），`PLATFORM_EVOLUTION_PENDING=false`，`EVOLUTION_THREAD=none`；这与已完成的业务发布分别报告。
