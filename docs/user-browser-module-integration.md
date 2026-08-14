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
任意 URL。Rust 宿主把 `ownerKey` 做稳定不可逆指纹后建立如下本机目录：

```text
app-local-data/
└── ai-web-profiles/
    └── <owner-fingerprint>/
        ├── chatgpt/
        └── google-ai-mode/
```

WebView2 自己在 Profile 中保存 Cookie、DOM storage、缓存和权限。应用不枚举、不导出、
不上传这些数据；“清除会话”只调用 WebView2 的整 Profile 浏览数据清理。OpenAI 官方
文档也明确区分浏览器中的 ChatGPT 网页会话与 Codex 客户端的浏览器回调登录：
[OpenAI authentication](https://learn.chatgpt.com/docs/auth)。

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
AI 模式的可见 DOM 适配器。主窗口提供刷新、返回主页、恢复窗口、系统浏览器回退以及统一
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

- Tauri crate 编译与本地宿主安全测试通过。
- Win 实机已验证未登录状态的 ChatGPT 官方页面可完整加载、关闭后状态可恢复并可再次打开。
- 正式安装版 `0.3.69+8eacc54c5c6b356dbce0c50838e875edfc03cdfb` 已验证一龙原生聊天窗
  独立创建、取得焦点并完成 React 根节点渲染；本机诊断收到 `created`、`page_started`、
  `page_finished` 和 `page_health/settled`，窗口未再白屏或闪退。
- Google AI 模式提供商固定指向官方 `google.com/aimode`，以独立 Profile 打开；Win 代码已
  接通问题、回答、引用、草稿、发送、停止和新对话的可见语义路径。账号登录仍定向到系统
  浏览器；地区、语言、设备或账号灰度未开放时保留完整 Google 官方窗口。
- PC TypeScript/Vite 生产构建、ESLint 和本地浏览器安全契约测试通过。
- 没有可验证的一龙云端 owner 或已登录本机节点 owner 时不能创建本地 Profile；两者同时存在
  但不一致时同样失败关闭。
- 同一账号/厂商复用窗口与 Profile，不同一龙账号使用不同指纹目录。
- 进入 Chat 模式即可在后台打开本地访客会话，不需要先勾选账号确认；用户主动登录时仍只操作本人账号，清除会话前再次确认。
- ChatGPT 与 Google AI 的输入权限只绑定经过清洗的 `composerReady` 可见语义，不把 `authenticated=false`
  当作拒绝基础聊天的理由；真实官网没有输入框时不会伪造访客可用。
- 普通浏览器不显示可调用的本地命令，继续使用托管模式。
- Google AI 模式真实页面 DOM、账号开放状态、流式回答与引用选择器仍需在已开放账号环境验收；
  真实账号登录、Cloudflare、下载、音视频和各账号兼容也仍需用户本人完成官方验证。
- DOM 变化会让语义适配器降级；降级时保留完整官方窗口、刷新、主页和系统浏览器入口。
- 一龙原生子窗口把创建、导航、页面完成、React 根节点健康、焦点、关闭请求和销毁事件写入本机有界脱敏
  快照；节点 Codex 控制诊断与项目绑定 MCP 可直接读取。快照不记录页面正文、Cookie、
  token、请求/响应正文、prompt 或 URL query；快照最多保留 4 条高频心跳，避免窗口事件被
  心跳覆盖，子窗口导航失败时也继续保留以便排障。
