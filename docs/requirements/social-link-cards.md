---
version_status: current
reviewed_at: 2026-09-16
implementation_status: in_progress
---

# 社交聊天外链卡片与阅读窗口

用户已要求在 Windows、Android APK、PWA 实现公众号、抖音、小红书、哔哩哔哩、币安广场、X 的外链分享体验。本文是本次接受范围，交付状态另记验收报告。

## 用户行为与验收

- 收到包含链接的普通好友或群消息，原文立即显示；可见区域内异步补充标题、来源、可用封面。保留原文、引用、编辑历史及右键菜单。
- 每条消息最多两张卡片，不批量加载播放器。失败显示可点击的来源卡片，用户仍能打开原文；不显示永久加载动画。
- B 站链接识别 BV 号，保留分 P 及 `t=2` 等时间参数。抖音短链、B 站短链在有限重定向内解析，失败仍可打开原始链接。
- X 帖子点击后使用官方嵌入；长文章和币安广场无已确认的通用嵌入合同时打开网页。不把发帖接口误用于读取外链，不要求用户提供发帖密钥。
- Win 和 APK 使用独立阅读窗口；关闭回到原聊天位置及草稿。PWA 对官方支持的播放器/帖子使用隔离嵌入，其余直接打开浏览器页，始终提供原文入口。
- 卡片沿用暗色主题，封面失败不留下大片空白；键盘、触屏和鼠标均可打开。
- 对 HTTPS 公共页面只取有上限的元数据，不保存网页全文、视频或远端 HTML，不向远端附带一龙凭据。缓存限定容量及 TTL，不能随聊天记录无限增长。
- 解析接口鉴权；公共网络 DNS 固定、逐跳验证、禁止私网；数据只通过安全文本/图片元素呈现，远端 HTML 不进入主应用。
- 构建与单测通过后按项目流程发布 Server/PC/PWA/APK；实际设备/平台可用性与代码覆盖分别报告。

## 官方合同及边界

| 平台 | 官方资料与本次接入 |
| --- | --- |
| 公众号 | [移动应用分享 SDK](https://developers.weixin.qq.com/doc/oplatform/Mobile_App/Share_and_Favorites/Android.html)用于向微信分享；收到文章采用公开页面元数据与原文阅读。 |
| 抖音 | [站外播放接口](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/iframe-player/get-iframe-by-video)返回标题及播放器，不把授权用户视频接口当通用抓取接口。 |
| 小红书 | [Android 分享 SDK](https://agora.xiaohongshu.com/doc/android)用于向小红书分享；读取公开元数据，保留访问所需参数，原文阅读兜底。 |
| B 站 | [官方站外播放器](https://player.bilibili.com/)支持 bvid、p、t、autoplay；保留播放位置。 |
| X | [官方 oEmbed](https://docs.x.com/x-for-websites/oembed-api)及[嵌入帖子](https://docs.x.com/x-for-websites/embedded-posts/overview)支持帖子；长文章未确认通用嵌入合同，原文阅读兜底。 |
| 币安广场 | [官方 Square 发布工具](https://www.binance.com/en/skills/detail/binance/square-post)用于发布，不支持读取现有帖子；采用公开元数据和原文入口。 |

本次不复制全文/视频、不自动转发到外部账号、不绕过平台登录/验证码/地域限制。外部平台随时可能限制读取，卡片和播放器可用性不等同于应用自身完成状态。

## 模块与兼容

服务端新增 link_previews 独立路由与策略/抓取/解析模块；Web/PC 共享轻量卡片行为；APK 使用原生卡片与无 JS bridge 的阅读 WebView。已有文章、项目、AI 会话分享卡片优先，不重复展开内部链接。原消息格式和数据库不变；旧客户端仍看到原文链接。预览为可淘汰的短期元数据缓存，重启可重建。
