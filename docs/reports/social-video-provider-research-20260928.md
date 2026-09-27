---
version_status: current
reviewed_at: 2026-09-28
implementation_status: research_only
---

# 群视频链接接入调查证据

## 范围和方法

用户要求读取“杀蟑螂”群的 B站、小红书、抖音链接，提出与视频号一致的卡片及后续平台接入方案。此次使用本机节点 `win_control` 的 `win_group_ai_action(groups/messages)` 与回执轮询，从已登录 Win 群账号读取；没有截图/点击、群消息发送、AI 提问或应用安装。

查询范围为该群最近 120 条消息，4 页，每页 30。MCP 消息字段仅提供最多 240 字预览；不能据此断言更早记录没有链接，也不能保证长分享 URL 完整。仅筛选目标平台 URL，没有保存整份聊天正文、Cookie、短期 MCP descriptor 或访问参数值。

代码基线 `35dca3dbafb8be401c256ef186e940ec52e2918c`。以下证据为 2026-09-28 的样本，不代表全部内容或所有设备版本。

## 实际样本

| 平台 | 群里找到的安全显示 URL | 无登录公共请求结果 |
| --- | --- | --- |
| B站 | `https://www.bilibili.com/video/BV19eYH6NEsC/` | 本机 HTTP 200，OG 标题/封面/video.other 存在；工具服务另一次抓取返回 412，说明访问环境影响结果 |
| B站 | `https://www.bilibili.com/video/BV1BEY96vEjJ/?t=80` | 本机 HTTP 200，OG 标题/封面/video.other 存在；须保留 80 秒播放位置 |
| 抖音 | `https://v.douyin.com/_XMEsxVKKOY/` | 302 到 iesdouyin share/video，再 302 到 douyin.com/video/7678952575260953882；落地页 200，但本次静态 HTML 未见目标 OG 字段 |
| 小红书 | `https://www.xiaohongshu.com/discovery/item/6a6e8951000000002402c81f` | 去查询参数探测重定向到 404 模板，不能当成目标笔记；带 MCP 预览中已观察到的访问参数则进入 /login，不能断言笔记删除 |

小红书消息预览正好 240 字，包含访问参数但可能截断。完整原始分享链接未通过本次 MCP 取得；报告中的安全 URL 仅用于标识内容，不应用它替换用户分享链接。下一轮需在已有成员权限范围内让链接解析器读取原始消息，而非扩大正文日志。

同一查询也找到了前轮视频号样本 `https://weixin.qq.com/sph/Aur6t4pfk3`，本次不重复已有验证。

## 抖音接口证据

调用官方公开 `GET https://open.douyin.com/api/douyin/v1/video/get_iframe_by_video?video_id=7678952575260953882`，未携带登录 Cookie：

- HTTP 200，业务 err_no=0。
- 返回视频标题、iframe_code；播放器 origin 为 `https://open.douyin.com`。
- video_width=2208，video_height=936，为横版。
- 此合同未返回封面字段；不能因此承诺抖音封面已经可用。
- 未在此次 Win/APK 播放实际音视频。接口确认与完整播放验收分开。

来源：[官方接口文档](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/iframe-player/get-iframe-by-video)。业务错误包括非公开视频，不能只判断 HTTP 200。

## 代码现状

| 已核对源码 | 发现 |
| --- | --- |
| `server/src/router/social_routes/link_previews/fetch.rs` | B站走通用元数据；抖音走独立 early return，只取 video_title，没有封面分支，且没有独立业务 err_no 校验 |
| `server/src/router/social_routes/link_previews/policy.rs` | B站/抖音具备 ID 到官方 embed 的构造；这是候选，不是实际内容播放证明 |
| `server/src/assets/social_links.js` | 平台 URL 识别和受信 embed 已有；大封面卡片目前仅 channels 特例 |
| `android/app/src/main/kotlin/com/elon/app/sociallinks/SocialLinkCardView.kt` | 视频号有独立大封面布局，其他平台仍为通用卡片 |
| `pc-frontend/src/features/friends/SocialLinkCards.tsx` | 视频号已有原生 handoff；其余进入原阅读流程，并非三平台皆直接拉起应用 |
| `server/src/assets/social_link_viewer.js` | 已有 B站/抖音/X 的隔离呈现，统一 no-referrer；接未来 YouTube 需单独身份策略 |
| `SocialLinkBrowserActivity.kt` / `SocialLinkReaderSessions.kt` | APK 已有播放器 URL、独立阅读会话及返回/最小化/PiP 基础，不应重造宿主 |
| `pc-frontend/src/features/friends/group-ai/groupAiControlModel.ts` | MCP messages 明确 slice(0,240)；原消息的链接提取必须有独立完整字段/执行通道 |

## 官方资料核对

- [B站站外播放器](https://player.bilibili.com/)：已核对 bvid、p、t 等参数；不能由 OG 中 video 字段推断可直接取得 MP4。
- [小红书 Deeplink](https://pages.xiaohongshu.com/activity/deeplink)：提供笔记与视频 route，但本轮没有客户端直达验证。没有找到并验证通用站外播放器，不等于已证明官方永远无此能力。
- [YouTube IFrame API](https://developers.google.com/youtube/iframe_api_reference)和[WebView 身份要求](https://developers.google.com/youtube/terms/required-minimum-functionality#embedded-player-api-client-identity)：官方嵌入与原生媒体下载不同，须传递真实应用来源。
- [TikTok 嵌入](https://developers.tiktok.com/docs/en/embed-videos)、[Vimeo oEmbed](https://developer.vimeo.com/api/oembed/videos)：可作为下一批公开合同适配，不代表现已接入一龙。

## 交付边界

已完成：真实群链接读取、四条链接的有限公共探测、抖音公开嵌入 API 核实、相关源码审计及[接入方案](../requirements/social-video-provider-plan.md)。

未完成：统一三平台媒体卡片、抖音封面补齐、小红书完整链接和登录后元数据、各平台 Win/APK 实际播放及精确跳转；未构建/发布应用。原有视频号实现保持不变，Win 微信精确直达仍未证实。

文档查阅限定入口规则、群 MCP、外链规范和命中的领域手册；默认未读取无关群正文、全库 docs 或历史 rollout。报告不将研究结果当成上线能力。
