---
version_status: current
reviewed_at: 2026-09-28
implementation_status: planned
---

# 跨平台视频卡片与播放接入方案

目标：群友贴入视频链接后看到真实封面、标题和作者；点击一次即可播放或打开对应平台，返回时不丢群聊位置和草稿。Win、APK、PWA 共用内容合同，不共用未经验证的跳转假设。

本文是后续实现方案，不是新版本交付声明。继承[外链卡片基线](social-link-cards.md)；视频入口按本文的能力路由设计演进，文章阅读不改。视频号已有实现和限制继续以[视频号要求](wechat-channels-playback-handoff.md)为准。[本轮实际群链接和调查证据](../reports/social-video-provider-research-20260928.md)独立保存。

## 平台策略

卡片展示、嵌入合同、实测播放、打开应用是四种不同能力。HTTP 200、拿到封面、iframe 加载完成都不能单独证明播放成功。

| 平台 | 已确认接口或合同 | 计划默认入口 | 尚须核实 |
| --- | --- | --- | --- |
| B 站 | [官方站外播放器](https://player.bilibili.com/)；支持 BV、分 P、时间位置 | 本应用内的官方播放器；保留“B站打开”与原网页 | 本轮两条群链接已取到标题封面；Win/APK 实际音视频播放、受限内容和精确应用跳转待验收 |
| 抖音 | [VideoID 获取 iframe](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/iframe-player/get-iframe-by-video)，文档说明无需申请权限 | 接口业务成功的内容优先官方播放器；受限时显示“抖音打开” | 群样本接口返回 err_no=0、标题及尺寸；封面、真实首帧/声音及各端跳转待验收 |
| 小红书 | [平台托管的 Deeplink 路由资料](https://pages.xiaohongshu.com/activity/deeplink)，包含笔记/视频路由，但不是当前版本稳定性保证 | APK 验证精确路由后直接打开小红书；Win 未验证路由前打开原网页，可复制链接 | 本轮未找到并验证任意公开笔记的通用官方嵌入播放器；群样本遇登录页。不能把所有笔记当视频，也不能承诺全站站内播放 |
| 微信视频号 | 已有公开落地页元数据与按需跳转适配 | APK 已有直接微信打开；Win 保留明确兼容入口 | 当前样本无可用独立媒体源；Win 已知只能唤起微信而未直达该视频，仍属未完成，不标“跳转成功” |
| YouTube | [IFrame Player API](https://developers.google.com/youtube/iframe_api_reference) | 新增适配后优先官方嵌入 | 未在项目接入；内容嵌入许可、网络可达性及 WebView 身份要求须验收 |
| TikTok | [oEmbed](https://developers.tiktok.com/docs/en/embed-videos)和[Embed Player](https://developers.tiktok.com/docs/en/embed-player) | 新增独立适配后优先官方嵌入 | 与国内抖音分开登记；地区/内容可用性待验收，不沿用抖音 ID 和路由 |
| Vimeo | [oEmbed](https://developer.vimeo.com/api/oembed/videos)和[Player SDK](https://developer.vimeo.com/player/sdk) | 新增适配后优先官方嵌入 | 作者隐私、允许嵌入的域名等限制仍有效；未列出内容须保留必要访问参数 |
| 其他平台 | 先登记官方资料、真实链接和支持范围 | 普通链接卡片及安全原网页 | 查到 SDK 不等于能读任意用户视频；投稿、分享、商业媒体 API 不等于通用播放 API |

“站内播放”首先指在隔离 WebView/iframe 内运行官方播放器，不是取得 MP4 后换成我们自己的播放器。只有内容方提供可用于本产品的媒体接口/授权地址，并完成权限及失效验证，才增加原生媒体播放。不得把上述嵌入 API 包装成下载接口。

## 卡片交互

1. 使用统一媒体卡片组件替代各平台独立大段 UI。展示真实封面、最多两行标题、作者、平台来源；取不到封面时退为紧凑来源卡片，不造封面、不展示破图。
2. 按有效媒体宽高选择横版或竖版框架；缺少尺寸时先稳定占位。限制最大宽高，封面完整可辨，不将所有视频强制裁为 3:4。群里的抖音样本为 2208×936，应为横版。
3. 已支持站内播放的条目使用播放图标。仅支持外跳的条目使用打开应用图标和“抖音打开”等明确动作，不用单独播放三角暗示会站内播放。
4. 主点击只执行一个默认动作。次要菜单保留“在本应用播放”（确有支持时）、“在平台打开”、“查看原网页”、“复制链接”，右键/长按的引用、多选、AI 回复功能保持不变。
5. APK 从群卡片直接调用平台 handoff，不先打开中间 WebView 再等用户点一次。Win 只有实际验证过的协议和处理器才走相应路径；唤起应用但未定位视频只能记 requested。
6. 已有播放器阅读窗口可继续复用。关闭释放播放资源；回到群聊恢复消息锚点、引用和草稿。需保留悬浮/画中画时由明确用户操作进入，同一端同一时间只保留一个有声播放器。
7. 不在聊天列表自动播放、自动开启几十个 WebView 或弹扫码框。已知需要登录/平台应用的条目直接表达下一动作；超时显示重试和外部入口，不无限转圈，也不超时后自动拉起别的应用。
8. 复用现有图标库与主题，Web 卡片圆角不超过 8px；按钮有可访问名称和悬停提示，APK 点击区至少 48dp。检查长标题、200% 字体、横竖屏，卡片加载不推走当前阅读锚点。

## 统一合同

扩展现有 `Preview` 为版本化 `social.media_preview.v2`，保留 v1 字段以兼容旧端。不要新增另一套群消息类型或改写原始消息。平台元数据可缓存，播放与跳转是用户发起的独立动作。

| 字段 | 含义 |
| --- | --- |
| `provider`, `provider_version`, `content_id`, `content_kind` | 平台、适配版本、稳定内容 ID、video/image/article/live/unknown |
| `source_ref`, `canonical_url` | 原消息/附件内的来源引用，以及不带敏感参数的稳定显示身份；执行请求仍使用完整来源，不能拿显示地址替代 |
| `title`, `author`, `cover`, `width`, `height` | 卡片数据；可缺省，尺寸需有界，不能借缺省值宣称视频类型 |
| `metadata_status` | ready/partial/auth_required/unavailable；标注 observed_at、expires_at 和数据来源 |
| `playback` | official_embed/provider_web/native_authorized/none；含审核过的目标、条件、原因和状态，不传任意 HTML |
| `handoff` | Android/Windows/Web 分别登记：supported/candidate/unsupported/unknown；含内容 ID 及目标引用，不长期保存短期 scheme |
| `default_action` | 由可用能力与用户选择计算，不由网址域名直接假定可播 |
| `evidence` | documented/api_confirmed/runtime_verified；按 OS、内容类型、版本、时间分别登记，样本成功不升级成全站保证 |

媒体过程状态：`idle -> resolving -> ready -> opening -> playing / handoff_requested / needs_login / failed`。封面更新不能触发播放或切换当前动作；关闭、换账号、消息撤回使未完成请求失效。

URL 策略必须分开：保留原始分享 URL；身份去重使用内容 ID；执行 URL 保留必要访问参数；日志/报告仅输出安全显示 URL。小红书的访问参数、B站的 p/t、Vimeo 未列出视频的 h 不能被通用“去追踪参数”误删。相同 ID 不代表不同权限链接能共享缓存。

## 模块边界

| 现有模块 | 本轮方案要求的补齐 |
| --- | --- |
| `server/src/router/social_routes/link_previews/` | 沿用公开网络校验、短链解析、封面副本、singleflight 与缓存；将 B站/抖音/小红书适配按职责拆开，新增注册表，不把所有分支塞入 fetch.rs |
| `server/src/assets/social_links.js` / `.css` | 沿用解析、挂载和账号缓存；将 channels 特例抽为通用媒体呈现与动作选择，但保留其独立 handoff |
| `pc-frontend/src/features/friends/SocialLinkCards.tsx` | 消费统一合同，交给通用 action router；调用现有 Win 阅读窗口或审核的原生 handoff |
| `android/app/src/main/kotlin/com/elon/app/sociallinks/` | 原生媒体卡片 + provider action router；复用会话、悬浮/画中画和现有视频号跳转模块，不复制整套 Activity |
| `server/src/assets/social_link_viewer.js` | 分平台选择受信播放器、sandbox、referrer 策略和事件适配；只在点击后创建播放器 |
| Win 群业务 MCP | 新增只读 link inventory / resolve / probe 合同，从原始消息提取完整链接，返回安全元数据。现有 240 字预览不能当完整 URL 数据源 |

Provider 适配职责固定为 `match -> normalize -> resolveMetadata -> resolvePlayback -> resolveHandoff`。公开合同与需登录的私有适配分开，缓存/安全/生命周期由公共层负责。新增平台主要增加适配器、配置、fixture 和一次两端验收，不复制 UI/缓存/调试框架。

先用公开接口，再用公开页面结构化数据；确需登录的只读解析在用户自己的隔离 WebView 身份上下文中进行，不将 Cookie 搬到群服务器。已有成员回读机制能复用则复用，但只分享用户明确允许的预览，禁止把私人正文/登录后内容默认写进共享公共缓存。

## 当前需要修补的缺口

- 抖音 `douyin_preview` 目前提前返回且只填标题，未补封面，也未以 err_no 和审核后的 iframe 合同确认能力；不能只生成一个播放器 URL 就当已可播。先验证业务结果、尺寸和播放器 origin，再从允许的公开元数据补封面；失败保留紧凑卡片。
- 登录页、验证码页和 404 模板的 title/og:image 必须排除。当前通用解析依靠“标题不为空”判断 ready，不足以确认拿到目标内容。
- 客户端三平台尚未套用视频号的大封面呈现，点击策略也没有完成统一分平台路由。不得只改 CSS 后宣称业务完成。
- 短链要记录解析后的身份，但保留可回到原始链接的入口。继续逐跳验证 URL/DNS，不把新的重定向域名静默加入白名单。
- 小红书的 URL 参数按其合同处理。此次去参数访问进入 404，而带已观察参数访问进入登录页；这不是内容已删除的证据。MCP 预览可能截断，后续从原始消息解析后再验证。
- `social_link_viewer.js` 当前统一 no-referrer。接入 YouTube 时不能直接照搬：[官方客户端身份要求](https://developers.google.com/youtube/terms/required-minimum-functionality#embedded-player-api-client-identity)要求 WebView 传递本应用身份。使用真实应用来源，不伪造为官方 App 或附带聊天 URL/访问参数。
- 不用隐藏二维码、伪造 JS bridge 或 UA 冒充官方客户端来替代播放合同。某个页面出现 video 标签或脚本含 videoUrl，只是候选证据；实际播放仍要拿到有效媒体/官方播放器。

## 缓存与安全

沿用当前服务端容量 512、并发 8、总解析超时 12 秒、成功 24 小时/失败 30 秒基线，以及客户端现有有界账号缓存；不新建无界全历史预取。访问敏感预览需独立 owner/scope key，不能直接进入当前公共 URL 缓存。

卡片先呈现缓存再异步更新；只请求可见卡片。metadata、封面与动作能力独立过期，负面能力有期限，不能因为一次登录失败永久关闭平台。短期跳转地址点击时获取并校验到期时间；同一次点击只派发一次，取消后丢弃迟到结果。具体 TTL 由公开合同和响应限制取更短值。

封面只允许受信 HTTPS 地址或有界服务器缩略副本；限制像素、大小、超时，禁止 SSRF/跨域凭据转发。执行层白名单校验 scheme、package、host、path、参数与浏览器回退地址；拒绝任意 intent extras、component 和 shell 命令。

外站播放器不得取得主群聊的 JS bridge 或本机管理能力。oEmbed HTML 不直接注入群 DOM；只解析验证后的 URL/固定模板。postMessage 必须核对 origin、source、会话随机标识与字段结构。需要登录的用户在平台官方上下文完成，不上传会话凭据。

## 分批实现与验收

| 顺序 | 交付内容 | 最小业务证据 |
| --- | --- | --- |
| 1 | 统一数据合同、媒体卡片、B站/抖音元数据补齐；小红书权限状态；完整链接 MCP 读取 | 群内现有四条链接解析，真实封面加载、横竖版布局、p/t 和访问参数保留，旧消息仍正常 |
| 2 | B站/抖音官方播放器路由，APK 优先完成，再 Win | 点击到真实首帧/时间推进/声音；失败明确且可打开原平台，关闭回群保留原位置 |
| 3 | 小红书、视频号精确 handoff 与 Windows 兼容 | APK 到同一条内容；Win 分别记录直达、仅唤起、无处理器；未直达不能标 completed |
| 4 | YouTube/TikTok/Vimeo 和后续平台插件 | 每平台一条真实样本 + 一条不可播放/受限样本，独立登记，不扩大成重复全量研究 |

共享合同、路由、安全和缓存用定向自动测试；各端各平台一次真实成功链路和必要失败恢复验收，回归或协议变化才重测。iframe onload 只记 document_ready，有官方事件时观察播放状态与时间推进；没有可信事件时做限定人工/设备验收，不跨域猜状态。

MCP 只读探测默认不播放、不唤起应用、不发消息；用户明确请求后，另一个受控动作执行播放或 handoff。验收结果使用稳定 capability_id（如 social_video_bilibili_embed_v1），携带代码/运行版本、平台、证据、未完成范围。不是拿到 API 结果就写 completed。

本方案不新增后台下载、转码或永久存储平台视频。此次交付仅方案和调查证据，无应用构建或新平台发布；上表能力必须在实际实现后逐项变更状态。
