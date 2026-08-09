# 用户专属 AI 浏览器接入

`/user-browser` 同时保留两条互不替代的运行路线。Win 客户端优先使用本机
WebView2；普通浏览器/PWA 仍可发现外部托管模块。需要登录时只允许用户操作本人账号，
主项目不接收厂商密码、Cookie、Access Token 或私有 API 数据。

| 路线 | 会话位置 | 当前用途 | 状态 |
|---|---|---|---|
| Win 本地 WebView2 | 用户 Windows 设备 | 官方网页、本地 Profile、可见语义同步 | ChatGPT 语义适配器、Google AI 模式官方网页已接线 |
| 外部托管模块 | 商户模块服务器 | 浏览器/PWA 的隔离远程会话 | 保留既有能力发现 |
| APK 一龙界面 | Android 本地 WebView | 消费去凭证化语义事件 | ChatGPT 语义适配器已接线 |

## 用户入口

客户端入口统一为“官方 AI 网页”，不再要求用户先理解“个人浏览器”或“Win CLI”：

- APK：聊天侧栏设置或个人资料 → ChatGPT 账号与聊天；进入后先打开官方页面登录，
  登录完成可留在官方网页或切换“一龙界面”。
- PWA：我的 → ChatGPT 账号与聊天；PWA 在新标签打开官方 ChatGPT，受同源隔离影响，
  不宣称可以读取登录状态或重渲染官方页面。
- Win：入口同时展示 ChatGPT 与 Google AI 模式。ChatGPT 在本地官方窗口登录后可回到
  一龙原生聊天区；Google AI 模式直接打开 `https://www.google.com/aimode`，本批次不做
  DOM 重渲染。若 Google 要求账号登录，用户改用系统浏览器。

这里的“登录”是设备内的官方网页会话，不是把 ChatGPT 云端账号或 Cookie 绑定到一龙
云端账号。登录状态由官方页面确认。

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

### Win 壳版本握手

`/pc` 页面由服务器即时更新，但 Tauri 命令来自本机 `elon-desktop.exe`，两者可能在用户
完成 Win 客户端更新前短暂错版。前端不能只凭 `window.__TAURI__` 判定本地浏览器可用：

- 必须实际调用 `list_local_ai_web_providers`，成功且列表非空后才显示“Win 本地可用”；
- 旧壳返回的 command not found、ACL/allowlist 拒绝要归一化为“需更新 Win 客户端”；
- 升级提示提供正式 Windows 安装包入口，并说明完全退出旧客户端后重新打开；
- 其他调用错误使用可重试状态，不能吞掉 Tauri 字符串 rejection 后只显示泛化失败。

Windows 上创建子 WebView 的 Tauri command 必须保持为 `async`；同步 command 会在
WebView2 窗口创建期间发生已知死锁，只留下无法导航的白色窗口。子窗口先以
`about:blank` 完成创建，再由宿主导航到登记的官方入口，契约测试固定检查这两个条件。

### IPC 与导航边界

- `build.rs` 只登记主窗口会话命令和子窗口语义事件命令。
- `capabilities/main.json` 只向 `main` 窗口和项目批准的 PC 地址开放该权限。
- 每个 Rust 命令再次检查调用 WebView 标签必须等于 `main`。
- ChatGPT 子窗口只匹配独立 capability，只能上报经过 Rust 白名单清洗的可见语义；它不能
  调用主窗口的会话控制命令。初始化脚本不读取 Cookie、Token、请求头或原始响应。
- ChatGPT 顶层导航仅接受 HTTPS、443、无 URL 凭据的 ChatGPT/OpenAI 域名及精确身份主机。
- Google AI 模式只接受 `google.com` 与 `www.google.com` 的官方搜索顶层页面；
  `accounts.google.com` 在本地窗口被明确拦截，并提示用户使用系统浏览器。
- Cloudflare 或身份提供商验证由用户本人完成；应用不绕过、不自动点击。
- 身份提供商可以拒绝嵌入式浏览器，应用不得伪装 User-Agent 或转移 Cookie 规避。

本地模式当前登记 `chatgpt` 与 `google-ai-mode`。Google AI 模式是 Google 搜索的官方
网页入口，不等于 Gemini 网页版，也不意味着客户端获得 Google OAuth 授权。Google OAuth
官方政策禁止应用把授权请求导向开发者可控制的嵌入式 user-agent，因此本地窗口只提供
访客网页，账号登录使用系统浏览器且两者不共享 Cookie。Gemini 仍不登记，后续应采用
系统浏览器 OAuth、官方 API 或经单独评审的公开接入方案。

## 统一原生渲染协议

`unifiedAiProtocol.ts` 定义 `yilong.ai.ui.v1`，只表达用户可见语义：

- adapter ready 与厂商能力；
- 会话 ID/标题变化；
- 消息快照与流式文字增量；
- idle/thinking/streaming/waiting/error 状态；
- 文本、图片、文件与引用内容块。

协议不定义 Cookie、Authorization、Access Token、原始请求头或网络响应。Win Rust
宿主和 Android 均已通过来源受限的桥接接入 ChatGPT 可见语义适配器；Win 主窗口提供
刷新、返回主页、恢复窗口、系统浏览器回退以及原生消息/文字输入区。后续每个厂商适配器
仍必须独立评审：

```text
官方网页 WebView（网络与登录主体）
        ↓ 仅用户可见语义
Provider Adapter（厂商独立、可降级）
        ↓ yilong.ai.ui.v1
一龙原生聊天 UI
```

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
- Google AI 模式提供商固定指向官方 `google.com/aimode`，以独立 Profile 打开；账号登录
  被定向到系统浏览器，地区、语言、设备或账号灰度未开放时保留 Google 官方提示。
- PC TypeScript/Vite 生产构建、ESLint 和本地浏览器安全契约测试通过。
- 未登录一龙账号时不能创建本地 Profile。
- 同一账号/厂商复用窗口与 Profile，不同一龙账号使用不同指纹目录。
- 用户确认本人账号后才能打开本地会话，清除会话前再次确认。
- 普通浏览器不显示可调用的本地命令，继续使用托管模式。
- 真实账号登录、Cloudflare、下载、音视频和各账号兼容仍需用户本人完成官方验证后验收。
- DOM 变化会让语义适配器降级；降级时保留完整官方窗口、刷新、主页和系统浏览器入口。
