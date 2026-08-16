# 用户专属 AI 浏览器接入

`/user-browser` 同时保留两条互不替代的运行路线。Win 客户端优先使用本机
WebView2；普通浏览器/PWA 仍可发现外部托管模块。基础聊天优先检测厂商访客能力；需要登录时只允许用户操作本人账号，
主项目不接收厂商密码、Cookie、Access Token 或私有 API 数据。

| 路线 | 会话位置 | 当前用途 | 状态 |
|---|---|---|---|
| Win 本地 WebView2 | 用户 Windows 设备 | 官方网页、本地 Profile、可见语义同步 | ChatGPT 与 Google AI 模式语义适配器已接线 |
| 外部托管模块 | 商户模块服务器 | 浏览器/PWA 的隔离远程会话 | 保留既有能力发现 |
| APK 一龙界面 | Android 本地 WebView | 消费去凭证化语义事件 | ChatGPT 语义适配器已接线 |

## 用户入口

客户端入口统一为“官方 AI 网页”，不再要求用户先理解“个人浏览器”或“Win CLI”：

- APK：聊天侧栏设置或个人资料 → ChatGPT 账号与聊天；进入后先打开官方页面登录，
  登录完成可留在官方网页或切换“一龙界面”。
- PWA：我的 → ChatGPT 账号与聊天；PWA 在新标签打开官方 ChatGPT，受同源隔离影响，
  不宣称可以读取登录状态或重渲染官方页面。
- Win：首页“一龙 AI”分为 **Chat** 与 **工作**。Chat 默认使用一龙原有消息流，可选择
  ChatGPT 或 Google AI 模式；两种模式始终挂载同一个 `AiChatPage`，共用原消息列表、输入框、
  用户侧栏和交互规范，仅替换消息发送与会话数据源。进入 Chat 或切换厂商时，Win 会静默创建后台
  WebView2 会话；只要官网真的提供可见输入框，未登录用户也能直接在一龙输入框聊天。工作模式保持原 Codex 项目与代理工作流。官方页面负责账号
  状态、推理与搜索，一龙适配器只把可见问题、回答、来源和输入框状态同步到统一聊天 UI。
  Google AI 模式固定打开 `https://www.google.com/aimode`；若 Google 要求账号登录，用户
  改用系统浏览器。

这里的“登录”是设备内可选的官方网页会话，不是把 ChatGPT 云端账号或 Cookie 绑定到一龙
云端账号。基础聊天是否可访客使用以官方页面当前是否出现可用输入框为准；历史、项目、个性化、
更高模型或厂商风控仍可要求登录。登录状态由官方页面确认。

Win 的“账号与本机会话中心”固定把三层状态分开显示：

1. 一龙账号：决定本机 Profile 的 owner 隔离键；
2. Google 作为一龙登录方式：只允许用户用同一 Google 身份进入一龙账号；
3. ChatGPT / Google AI 官方网页：基础能力先尝试访客使用，需要增强能力时再在各自 WebView2 Profile 内单独登录。

绑定 Google 到一龙账号不会把 Google Cookie、OAuth token 或系统浏览器登录态复制给
Google AI 模式，也不会让 ChatGPT 自动登录。云端账号资料短暂不可用时，Win 可从已经登录
且绑定的一龙本机节点恢复同一个稳定 owner；若云端账号和本机节点 owner 不一致则失败关闭，
不会打开或清除任一方的厂商 Profile。

## Win 本地模式

桌面壳固定登记厂商入口，PC 前端只能传 `providerId` 和当前一龙 `ownerKey`，不能传
任意 URL。Rust 宿主以 SHA-256 生成 128 位十六进制稳定指纹后建立如下本机目录；升级时按厂商
原子迁移旧 64 位目录，迁移被正在使用的 WebView2 暂时阻塞时继续使用旧目录并在后续重试，
不会为了升级丢失用户已有 Cookie、会话或快照：

```text
app-local-data/
└── ai-web-profiles/
    └── <owner-fingerprint>/
        ├── chatgpt/
        │   └── yilong-semantic-snapshot.v1.dpapi
        └── google-ai-mode/
            └── yilong-semantic-snapshot.v1.dpapi
```

WebView2 自己在 Profile 中保存 Cookie、DOM storage、缓存和权限。应用不枚举、不导出、
不上传这些数据；“清除会话”只调用 WebView2 的整 Profile 浏览数据清理。OpenAI 官方
文档也明确区分浏览器中的 ChatGPT 网页会话与 Codex 客户端的浏览器回调登录：
[OpenAI authentication](https://learn.chatgpt.com/docs/auth)。
宿主导航日志只记录 `scheme + host + path`；搜索问题、登录参数、fragment 和 userinfo 不进入日志。

一龙原生 UI 另使用三层 stale-while-revalidate 快照，避免厂商切换时先清空再等待官网：

1. React 进程内有界热缓存按当前 owner 与 provider 隔离，切换厂商的首帧直接回显；
2. Rust 宿主进程内快照跨 `/pc` 页面刷新保留，并继续由 1.5 秒有界状态轮询更新；
3. Windows 客户端重启时，从同一 owner 指纹与 provider 目录读取 Windows 当前用户 DPAPI
   加密的最后完整快照，随后后台 WebView2 与官方页面重新同步。

缓存状态明确区分 `empty`、`cached` 与 `live`。缓存只负责显示消息、公开引用、会话和项目目录；
发送、停止、新建会话和历史同步仍必须等待当前官方页面的实时 `composerReady` 与适配器能力。
持久快照不保存输入草稿、流式半成品、命令结果、Cookie、token、请求头或原始响应；完整会话超过
2 MiB 时先删除较旧的本机会话副本，再从当前聊天最旧消息开始裁剪，同时保留最近消息和
`messageWindowStart/observedMessageCount` 边界。单条异常数据仍超限、缓存损坏、版本未知或解密失败时
静默忽略并回退官方页。“清除会话”同时清除 WebView2 浏览数据、内存快照和对应 DPAPI 文件。

后台访客会话不会主动弹出官方窗口；官网没有提供输入框、首次登录、Cloudflare 或厂商真人验证期间，
用户可主动显示官方 WebView2，并由本人操作。会话就绪后用户可将官方页收起到本机后台，在一龙 Chat UI 中继续发送、停止、新建会话和
读取可见回复。刷新后台页面不会强制把官方窗口重新弹出，“显示官方页”始终可以一键恢复。

### Win 壳版本握手

`/pc` 页面由服务器即时更新，但 Tauri 命令来自本机 `elon-desktop.exe`，两者可能在用户
完成 Win 客户端更新前短暂错版。前端不能只凭 `window.__TAURI__` 判定本地浏览器可用：

- 必须实际调用 `list_local_ai_web_providers`，成功且列表非空后才显示“Win 本地可用”；
- 旧壳返回的 command not found、ACL/allowlist 拒绝要归一化为“需更新 Win 客户端”；
- 升级提示提供正式 Windows 安装包入口，并说明完全退出旧客户端后重新打开；
- 其他调用错误使用可重试状态，不能吞掉 Tauri 字符串 rejection 后只显示泛化失败。

Windows 上创建 WebView 的 Tauri command 必须保持为 `async`；同步 command 会在
WebView2 窗口创建期间发生已知死锁，只留下无法导航的白色窗口。一龙原生聊天窗在 Windows
作为独立顶层窗口直接加载登记的 `/pc/user-browser/native` 地址；主窗口与原生聊天窗必须
使用完全相同的 WebView2 browser arguments，否则共享默认用户数据目录的 WebView2 环境会
拒绝初始化，而 Rust build 返回后用户只看到窗口一闪而过。非 Windows 平台继续使用 parent
关系。宿主导航失败时不得销毁窗口：保留窗口并显示稳定诊断码，防止用户只看到“一闪而过”。

### IPC 与导航边界

- `build.rs` 只登记主窗口会话命令和子窗口语义事件命令。
- `capabilities/main.json` 只向 `main` 窗口和项目批准的 PC 地址开放该权限。
- 每个 Rust 命令再次检查调用 WebView 标签必须等于 `main`。
- ChatGPT 与 Google AI 模式子窗口分别匹配独立 capability，只能上报经过 Rust 白名单
  清洗的可见语义；它们不能调用主窗口的会话控制命令。初始化脚本不读取 Cookie、Token、
  请求头或原始响应，也不发起厂商私有网络请求。
- ChatGPT 顶层导航仅接受 HTTPS、443、无 URL 凭据的 ChatGPT/OpenAI 域名及精确身份主机。
- Google AI 模式只接受 `google.com` 与 `www.google.com` 的官方搜索顶层页面；
  `accounts.google.com` 在本地窗口被明确拦截，并提示用户使用系统浏览器。
- Cloudflare 或身份提供商验证由用户本人完成；应用不绕过、不自动点击。
- 身份提供商可以拒绝嵌入式浏览器，应用不得伪装 User-Agent 或转移 Cookie 规避。

本地模式当前登记 `chatgpt` 与 `google-ai-mode`。Google AI 模式适配器只替换呈现层，
搜索、回答、来源和开放策略仍由 Google 官方页面决定；它不等于 Gemini 网页版，也不意味
客户端获得 Google OAuth 授权。Google OAuth 官方政策禁止应用把授权请求导向开发者可控制
的嵌入式 user-agent，因此本地窗口优先使用访客网页，账号登录使用系统浏览器且两者不共享
Cookie。Gemini 仍不登记，后续应采用系统浏览器 OAuth、官方 API 或经单独评审的公开方案。

## 统一原生渲染协议

`unifiedAiProtocol.ts` 定义 `yilong.ai.ui.v1`，只表达用户可见语义：

- adapter ready 与厂商能力；
- 会话 ID/标题变化；
- 消息快照与流式文字增量；
- idle/thinking/streaming/waiting/error 状态；
- 文本、图片、文件与引用内容块。

协议不定义 Cookie、Authorization、Access Token、原始请求头或网络响应。Win Rust
宿主和 Android 均已通过来源受限的桥接接入 ChatGPT 可见语义适配器；Win 另接入 Google
AI 模式的可见 DOM 适配器。Win 不再维护缩水或分叉的厂商脚本：ChatGPT 按与 APK 相同的固定
31 个语义模块顺序加载，并用适配器版本与每页随机文档令牌约束事件和命令；Rust 回归会直接对照
Android 的资产清单和版本，任一端新增、遗漏或乱序都失败关闭；Google 直接复用 APK 的
`google_web_adapter.js`。页面完成加载后宿主会幂等重注入并请求快照，覆盖首次加载、官方登录跳转和
SPA 文档切换。主窗口提供刷新、返回主页、恢复窗口、系统浏览器回退以及统一
消息/文字输入区。每个厂商适配器仍必须独立评审：

```text
官方网页 WebView（网络与登录主体）
        ↓ 仅用户可见语义
Provider Adapter（厂商独立、可降级）
        ↓ yilong.ai.ui.v1
一龙原生聊天 UI
```

桌面壳在厂商清单中同时返回 `adapterActions`，作为该厂商允许执行的动作白名单；前端再把
白名单与当前页面快照的动态 `capabilities` 取交集。因此“发送、停止、新建对话、历史与项目、
Google 登录入口”等按钮只会在宿主允许且页面当前确实具备能力时启用。旧桌面壳没有返回动作
清单时，前端只使用与当前内置适配器一致的兼容清单，不把 Google AI 模式误当成支持 ChatGPT
历史、项目或登录动作。

每次 Win 原生命令还携带有界随机 `requestId`，React 只接受“动作 + requestId”同时匹配的
回执，避免快速切换厂商、连续发送或菜单操作时把旧结果当成当前成功。ChatGPT 消息快照保留
Markdown 和图片、文件、代码、表格、公式、音视频、图表等安全结构化描述；模型、工具、附件、
听写与功能导航统一显示在原聊天输入区上方，不另建第二套聊天页面。相关菜单仍由官网当前可见
控件动态产生，过期选项失败关闭并提示显示官方页。

ChatGPT 侧栏对缓存采用“先显示、后刷新”：已有项目与会话目录不会阻止原生 UI 首帧回显，
但每次重新激活 ChatGPT 厂商仍会在实时页面就绪后后台执行一次 `list_conversations`，以官网
当前目录原子替换旧快照。Google AI 目前只缓存当前 AI 搜索会话，不伪造官网未提供的项目列表。

原生界面必须把技术状态归一化为用户可理解的阶段：客户端检查或需更新、官方窗口未打开或加载中、
导航拦截或错误、适配器等待、ChatGPT 需要登录、Google 地区/语言/账号不可用的只读降级、
Google 访客可用、ChatGPT 已登录以及回答生成中。状态卡同时显示当前已激活的原生能力；不能发送
时直接给出官方窗口回退入口和原因，不再用一个笼统的“官方页已打开”掩盖账号或页面能力差异。
快照只增加经过 Rust 白名单清洗的 `pageKind` 与布尔 `loginRequired`，不增加 Cookie、凭证、
页面全文或网络请求访问。

适配器失效时必须退回完整官方网页，不得自动切换 Cookie 私有接口重放路线。

## 外部托管模块

既有路线通过开放商业公共目录发现 `browser.chatgpt.session.launch`，执行服务器端 action
confirmation，再调用商户模块运行时：

1. 为可信模块服务器所属项目创建商户，并配置 HTTPS `merchant_runtime` Binding。
2. 使用 `contracts/open-commerce/user-browser-capability-v1.json` 的输入和输出 Schema。
3. 保持 `kind=action`、`access_level=public`、`handler_type=merchant_runtime`、价格为 0。
4. 验证 Runtime Manifest 后发布商户目录。
5. 公共目录中只能保留一个活动的同名 Runtime；多个来源时失败关闭。

`public` 只表示所有已登录一龙用户都能请求动作。档案所有权由主项目写入签名运行信封的
`requester_user_id` 决定，能力输入不能指定用户。每个用户仍需在模块页面自行登录本人账号。

## 当前验收边界

- Tauri crate 定向 Rust 测试、本地宿主安全测试、28 个共享适配器 JavaScript 语法检查、
  PC 用户浏览器契约与 TypeScript/Vite 生产构建通过。
- Win 实机已验证未登录状态的 ChatGPT 官方页面可完整加载、关闭后状态可恢复并可再次打开。
- 正式安装版 `0.3.69+8eacc54c5c6b356dbce0c50838e875edfc03cdfb` 已验证一龙原生聊天窗
  独立创建、取得焦点并完成 React 根节点渲染；本机诊断收到 `created`、`page_started`、
  `page_finished` 和 `page_health/settled`，窗口未再白屏或闪退。
- Google AI 模式提供商固定指向官方 `google.com/aimode`，以独立 Profile 打开；Win 代码已
  接通问题、回答、引用、草稿、发送、停止和新对话的可见语义路径。账号登录仍定向到系统
  浏览器；地区、语言、设备或账号灰度未开放时保留完整 Google 官方窗口。
- ChatGPT Win 桥已补齐 APK 使用的完整 31 模块、版本 125 与文档令牌绑定；启动错误额外记录稳定的
  模块阶段名，但不记录页面正文、Cookie、Token 或异常消息。Google Win 桥复用
  APK 版本 1 的消息提取器和 `google_web` 适配器，并在每页生成独立文档令牌。WebView2 的
  initialization script 只先安装本机消息出口，等待 DOM 根节点和 `DOMContentLoaded` 后再安装
  Google 语义桥；因此“窗口/Profile 已连接”不会再早于适配器首份快照被误当成可发送。重复重连
  保持幂等，桥缺失只产生不含正文、Cookie、令牌或 URL query 的脱敏诊断。旧的 Windows Google
  重复脚本已经删除，新增厂商时必须通过 `ProviderAdapter` 显式登记初始化、动作白名单、事件清洗
  和页面命令绑定。
- PC TypeScript/Vite 生产构建、ESLint 和本地浏览器安全契约测试通过。
- Win 已实现按 owner/provider 隔离的前端热缓存、Rust 进程缓存和 DPAPI 持久快照；owner 目录使用
  SHA-256 截断指纹并兼容迁移旧目录，超长完成态聊天会保留最近上下文而不是整份放弃持久化。定向测试覆盖
  LRU 淘汰、账号/厂商隔离、旧 Profile 迁移、缓存状态转换、超长消息裁剪、草稿/流式过滤、当前 Windows
  用户加密回读与清除。
  缓存快照只读，官方页面重新加载完成前不会解锁发送或历史动作。
- 生产首页不再用统一的 2.4 秒轮询窗口判断所有网页动作：发送、会话目录、附件和延迟菜单分别使用有界
  动作期限。ChatGPT 会话目录先回传当前可见项和 `complete=false`，随后在后台滚动同步完整历史；Rust
  合并层会保留部分采集没有看到的缓存会话、项目和置顶项，完整采集才可移除官网已不存在的普通会话。
  `observedCount` 表示本轮官网实际观察数量，`availableCount` 表示合并缓存后的可展示数量。
- ChatGPT 消息快照按 `messageWindowStart` 与 `observedMessageCount` 合并同一会话的虚拟化窗口，暂时缩短的
  DOM 快照不会清空上一轮可见上下文；新建、打开会话或项目仍建立明确边界，绝不继承上一会话消息。
- 没有可验证的一龙云端 owner 或已登录本机节点 owner 时不能创建本地 Profile；两者同时存在
  但不一致时同样失败关闭。
- 同一账号/厂商复用窗口与 Profile，不同一龙账号使用不同指纹目录。
- 进入 Chat 模式即可在后台打开本地访客会话，不需要先勾选账号确认；用户主动登录时仍只操作本人账号，清除会话前再次确认。
- ChatGPT 与 Google AI 的输入权限只绑定经过清洗的 `composerReady` 可见语义，不把 `authenticated=false`
  当作拒绝基础聊天的理由；真实官网没有输入框时不会伪造访客可用。
- 普通浏览器不显示可调用的本地命令，继续使用托管模式。
- Google AI 模式真实页面 DOM、账号开放状态、流式回答与引用选择器仍需在已开放账号环境验收；
  ChatGPT 与 Google 从一龙原生输入框真实发送、接收完整回答、停止生成和跨跳转恢复也尚未在用户
  账号环境完成本轮验收。真实账号登录、Cloudflare、下载、音视频和各账号兼容仍需用户本人完成官方验证。
- Win 模型/工具选择、官方附件选择、听写授权、产品功能导航和结构化内容卡片已完成代码路径，
  但仍需在 ChatGPT 当前真实 DOM 与 WebView2 权限环境统一现场验收。
- 真实账号下 ChatGPT/Google AI 厂商切换首帧时延、重启后 DPAPI 快照回显和官网目录替换仍需
  安装版现场验收；代码、加密与离线合同通过不能替代真实网页性能验收。
- DOM 变化会让语义适配器降级；降级时保留完整官方窗口、刷新、主页和系统浏览器入口。
- 一龙原生子窗口把创建、导航、页面完成、React 根节点健康、焦点、关闭请求和销毁事件写入本机有界脱敏
  快照；节点 Codex 控制诊断与项目绑定 MCP 可直接读取。快照不记录页面正文、Cookie、
  token、请求/响应正文、prompt 或 URL query；快照最多保留 4 条高频心跳，避免窗口事件被
  心跳覆盖，子窗口导航失败时也继续保留以便排障。
