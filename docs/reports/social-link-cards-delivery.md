---
version_status: report
reviewed_at: 2026-09-16
---

# 六平台聊天外链交付证据

需求：[社交聊天外链卡片与阅读窗口](../requirements/social-link-cards.md)。官方资料与能力边界以该需求内的六平台来源表为入口。

## 实现边界

| 能力 | 实现 | 验证 | 交付 / 验收 |
| --- | --- | --- | --- |
| 六平台链接识别、分享标题、B 站 t/p、原链接保留 | implemented | JS、Android、Rust 合同通过 | 发布回执另记；远端平台限制见下表 |
| 服务端元数据、短链解析、公共 DNS 固定、缓存 | implemented | 生产源码 harness 14 项通过；6 来源只读探测完成 | 正式构建与部署回执另记 |
| PC / PWA 暗色卡片、点击加载官方播放器、重试 | implemented | PC 构建、异步旧回调、失败恢复、隔离 X srcdoc 通过 | 真实 PNG 另记；完整登录用户流程待实际环境验收 |
| APK 卡片、独立 WebView、后退/前进/刷新/返回聊天 | implemented | 4 项模型测试与编译通过 | APK 发布回执另记；真机播放与登录恢复 deferred |
| 与系统分享、图片来源、二维码能力共存 | implemented | Web 合同覆盖六平台不重复生成来源卡、其他链接保留 | 合入 a6ae8bbb7 后补构建；图片来源领域逻辑保留 |

服务端最多 512 条内存缓存：成功 24 小时、失败 30 秒，同一 URL 合并请求、最多 8 个并发抓取、全程 10 秒上限、最多 4 次重定向、HTML/JSON 输入最多 256 KiB。只缓存标题、作者、封面 URL、来源和安全播放器描述，不修改消息数据库或保存远端全文/媒体。网页/PC 内存缓存最多 128 条；APK 元数据 128 条、封面内存 8 MiB、封面磁盘 24 MiB。

外链只使用标准 HTTPS 公网，逐跳重新验证与固定 DNS；共享已有公网地址策略。远端 oEmbed HTML 仅取纯文本摘要，不注入主应用。X 的 widget 在独立、无同源权限的 iframe 或无原生 bridge 的 APK 阅读页执行。封面不携带一龙 token；平台通用 Logo 被过滤，HTTP 图片升级需先验证 HTTPS 可用。

## 外部来源只读探测

生产源码测试 `social_link_preview_live_public_sources` 在本机逐个调用六个公开样例，确认有界返回及原 URL 保留。它不将平台拒绝访问误报成预览成功，也不证明终端能播放视频。

| 平台 | 此次元数据 | 封面 | 安全播放器描述 |
| --- | --- | --- | --- |
| 公众号短地址样例 | unavailable；另一次公开页检查返回环境验证页 | 无 | 原文阅读 |
| B 站用户样例 | unavailable | 无 | 有，保留 t=2 |
| 抖音用户短链样例 | ready | 官方接口未给封面 | 有 |
| 小红书不含访问令牌的样例 | ready | 未取得可用图片 | 原文阅读 |
| 币安广场公开样例 | unavailable | 无 | 原文阅读 |
| X 官方文档样例 | unavailable；本机官方 endpoint 不可达 | 无 | 有 |

“unavailable”仍显示来源/可用分享标题，卡片和原文入口可点击，不阻塞聊天。用户完整链接中的访问参数在实际消息与请求中保留；测试不提交访问令牌。PWA 对不支持嵌入的页面打开浏览器，不承诺通过 iframe 绕过平台限制。

## 验证入口与环境记录

- `node scripts/test-social-link-cards.mjs`：URL、两个 API 响应合同、迟到回调、失败重试、X iframe 隔离及新旧卡片去重。
- `node scripts/test-social-source-links.cjs`：已有来源链接与转发元数据回归。
- `scripts/validate-rust.ps1 -- test --manifest-path server/tests/social-link-preview-harness/Cargo.toml`：直接引用生产服务和公共地址策略，14 项通过；在线只读探测单独显式运行。
- `:app:testDebugUnitTest --tests com.elon.app.sociallinks.SocialLinkPolicyTest`：4 项通过；`npm run build` 通过。
- `check-source-size.ps1` 与 `git diff --check` 通过；原大型入口只接线，新增业务文件最多 223 行。

完整服务端测试二进制曾因 LLVM 内存不足终止，Android 一次构建也受同一内存压力影响；随后分开验证并使用生产源码定向 harness。未以失败的完整测试程序冒充通过，也未修改全局 JVM、Cargo 配置或其他任务进程。

发布版本、提交、PNG 证据和统一收尾状态在发布完成后补记；本报告不把构建或固定元数据样例当成真机播放验收。
