---
version_status: current
reviewed_at: 2026-09-28
implementation_status: in_progress
---

# 视频号短链接接入

## 范围与结论

修复聊天卡片封面、聊天直接跳转，以及 Win 阅读栏和 Android 原文阅读器的兼容入口。不修改全局 UA、不伪造
`WeixinJSBridge`、不导出 Cookie，也不把封面图片当作视频。保持普通链接 HTTPS
导航限制、原页面及二维码。页面代码和接口返回均是不可信输入。

## 真实研究

通过本机 `browser_research` MCP 在 Win WebView2 打开用户给出的短链接：

- `weixin.qq.com/sph/<id>` 重定向到 `channels.weixin.qq.com/finder-preview/pages/sph?id=<id>`。
- 官网同源 POST `/finder-preview/api/feed/get_feed_info` 接收 `baseReq.generalToken`、
  `shortUri`；运行时样本返回封面、说明、统计、`sceneInfo`，没有视频流地址。
- 官方脚本会在页面内读取已有 query/cookie token；无 token 的实测样本传空串。
  页面来源校验不可忽略：无 Origin/Referer 的外部请求返回 401；正确来源的只读
  请求返回 HTTP 201、`errCode=0`。阅读器适配器使用页面同源 fetch；聊天解析器使用
  固定公共端点、空 generalToken 与公开来源头，不搬运登录身份。
- `sceneInfo.dynamicExportId` 有 `expiredTime`。官网移动端构造
  `weixin://biz/finder/openFinderFeed/` 加单次 URI 编码的参数串；桌面端默认生成
  指向 `commonFinderJsApi.html` 的二维码。
- 微信 UA 分支调用 `WeixinJSBridge.invoke(openFinderView)` 或
  `openUrlWithExtraWebview`。UA 字符串本身不能提供这些原生能力。本批未实现
  一龙内直接播放，也未证明所有授权的视频接口都不可用。
- 进一步追踪 `feed.db97a0d4.js`：官方确实内置 HTML video 播放器，输入为
  `h264VideoInfo.videoUrl`、`h265VideoInfo.videoUrl` 或旧 `videoUrl`；有 HEVC 失败后
  回到 H.264 的逻辑。`no_redirect`、`no_autoplay` 只是页面行为选项，不提供播放凭证。
- `sph` 短链接分支的 watcher 跳过 `initVideoSource`，播放处理也排除该模式；点击封面
  走 `onSphClick`，再次显示二维码/微信打开提示。关闭或隐藏弹窗不等于初始化了媒体。
- 同一个无登录样本，以桌面 Chromium、Android Chromium、附加 Windows 微信标识、
  附加 Android 微信标识四种 UA 请求相同接口：均 HTTP 201、`errCode=0`，但都无三种
  视频字段。**只改 UA 在本次样本没有解决播放。** 不能据此推断所有版本/账号相同。
- 使用返回的 `dynamicExportId` 按源码另一种请求形态读取：返回业务 warning(type=2)、
  无 feed 字段。未将 HTTP 成功视为可播放。
- 前端实际引用的 `/web/pages/feed?eid=...` 在普通客户端返回 HTTP 200，但正文为
  `errCode=10012 / Illegal request`；没有尝试伪造身份绕过。
- 二维码中转页 `mobile/commonFinderJsApi.html` 的主要逻辑在内联脚本：等待
  `WeixinJSBridgeReady` 后调用 `openFinderView`，并调用短链接 export key 检查。
  它不是独立播放器，也没有观察到可用于一龙 Windows 容器的外部定位接口。

源码证据（MCP 处理后文本哈希，不是原始字节哈希）：

| 资源 | SHA-256 | 定位 |
|---|---|---|
| `merlin.82141f20.js` | `b298475b6c05a334452862127d44d89ad2ee692f6b9b2b95f4875d8a8976b25d` | `getFeedInfo`、`getFeedDetail` |
| `mmfinderopenwebapisvr.dd823d0e.js` | `8d241217f2957eed398d1aaed277b9f28a742134e896d0efdc69c02a66a2cbf9` | `o_`，约字节 280k 至 284k |
| `feed.db97a0d4.js` | `a74d86e6a5a63dcab42c2eb32c68c23f44fbebd41988ff9ac589efe078986996` | 末尾微信桥分支 |

## 实现边界

- `channels.rs`：旧的通用 HTML Open Graph 只能读到“视频号”标题，实际封面和作者
  在 JSON `feedInfo.coverUrl / description`、`authorInfo.nickname`。新增独立解析，
  仅该平台允许 `finder.video.qq.com` 封面源；复用 DNS 固定、体积限制和缩略图缓存。
- 登录后的 `/api/me/link-preview/wechat-open` 只接收官方短链接，固定公共只读请求，
  超时和并发有界；响应 no-store，临时参数不混入持久封面缓存、不输出日志。
- Android `WechatChannelsCardAction` 和 Win `wechatCardAction.ts`：从聊天卡片直接
  解析和调用微信，不创建阅读器/WebView。会话账号、页面生命周期、重复点击及过期
  独立校验；失败允许重试、复制稳定链接或用户主动打开原网页。
- 双端视频号卡片采用固定 3:4 封面、作者行和独立“查看原网页”；不是内嵌播放器。
  普通网页浏览器保持原文路径，不伪装拥有原生应用调度能力。
- 共享 `wechat_channels_handoff.js`：仅精确官方预览路径；新鲜只读请求、6 秒超时、
  nonce 单飞、页面代次检查、链接有效期检查、固定错误码。跳转状态只保存在当前页面内。
- Android `WechatChannelsPolicy/Handoff`：官网主框架跳转进入原生确认；工具栏
  “微信打开”走同源解析。只发 `ACTION_VIEW` 给 `com.tencent.mm`，不解析任意
  `intent://`、不透传 component/extras、不授予 URI 权限。后台和页面切换取消待执行操作。
- Win `internal_browser_wechat.rs`：只有主窗口显式 `wechat` 命令执行跳转；仍保持
  普通导航 HTTPS 白名单。不会让后台网页自动唤起任意应用。
- Win 阅读栏有“尝试在微信打开”和“复制视频号链接”按钮。复制的是稳定短链接，
  不复制临时跳转参数。系统接受请求只能报告“已请求”，不能报告视频播放成功。

## 验证记录

| 层级 | 当前证据 |
|---|---|
| 共享解析器 | 6 项 Node 定向测试通过，覆盖来源、参数、到期、幂等、导航竞态和错误投影 |
| 后端源码 | 生产源码 harness 22 项通过；新增真实公共样本测试通过，包含图片下载、缩略图和新鲜跳转 |
| 卡片渲染 | shared JS 契约通过；Playwright 使用真实公共封面验证 360/1280 视口、3:4、主/次入口独立点击，非安装包实机证据 |
| Android | 5 项定向 JUnit/Robolectric 测试通过；真实生产 Kotlin 源码编译通过 |
| Win 卡片控制 | 生产 TS 传输代码定向测试通过，覆盖固定解析接口、IPC、内容 ID、到期、取消和非原生浏览器；ESLint 通过 |
| 真实同源合同 | 用共享解析器读取真实预览接口，获得有效、未来到期的 feed scheme |
| 小米微信 | 已核对登记硬件；相同 scheme 进入 `FinderShareFeedRelUI`，截图看到指定视频播放 |
| Windows 微信 | 注册了 `weixin` 协议；真实调用后只打开聊天主窗口，未进入指定视频，也无视频子窗口 |
| 新 APK 按钮全链路 | 1822 已安装；MCP 可达但 `activity_bound=false`，新卡片点按验收延期，不能用直接 Intent 验收替代 |
| Win Rust | `validate-rust.ps1 ... test ... internal_browser` 6 项通过，含隐藏/关闭/导航取消；运行身份已核对，见下方发布记录 |
| PC 前端 | TypeScript 检查与 Vite 生产构建通过；线上 `/pc` 和前端发布标记检查通过 |

## 发布记录

- 功能提交 `14940f7ee5abcc645a3f26c79942dee6effbc297` 已推送主线。
- 服务端与 PC 前端正式发布 `0.3.1783`，来源为功能提交；健康检查、`/pc` HTTP 200
  和 `PcFrontend` 完成门禁通过。
- APK `1.1.1822 (1822)` 已正式发布。SHA-256：
  `296a5d08a67a3f99412c3e9adf21ea8c92797602be959eedaba6b9f7d9d48d01`。
  主项目登记小米通过 `adb install -r` 更新并回读版本一致；荣耀离线，未清数据。
- Win 本次单独发布被已有发布锁拒绝，未抢锁、未重复构建。确认正在发布的主线后代
  `e2b17ce145c91603c568eaa6722f38742f5741a6` 包含全部功能提交，新增 6 个路径不改动
  视频号模块，因此复用该正式包。远端 outbox 首次执行为 `synced`，`NodeAgent`
  完成门禁通过。
- Win 无人值守更新回执 `1b828046-ada2-4e9c-b5d5-86349379823f` 为 `passed`：节点与
  实际桌面进程均为 `0.3.69+e2b17ce145c91603c568eaa6722f38742f5741a6`，并非仅校验
  磁盘候选。该回执只证明安装和运行版本，不证明微信直达指定视频或站内播放。
- 正式发布全部完成；新卡片的安装端交互验收仍延期，Windows 精确视频定位与站内
  播放能力仍按下方边界登记未完成。

## 剩余边界

电脑微信目前不能据此宣称直达视频。对方暂没有标准 SDK 文档，后续按实际前端与
客户端能力持续分析，不把“没有 SDK”当作终止研究的理由，也不保证未验证的入口可用。
两条独立能力的分工、状态与验收见[播放与跳转方案](../requirements/wechat-channels-playback-handoff.md)。
仍需获得并验证 Windows 定位入口和有效嵌入播放数据；现有兼容入口提供扫码/复制链接。
不尝试注入微信进程、读取私有数据库、绕过登录或解密受保护视频。
