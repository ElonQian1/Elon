# 用户专属 AI 浏览器接入

`/user-browser` 同时保留两条互不替代的运行路线。Win 客户端优先使用本机
WebView2；普通浏览器/PWA 仍可发现外部托管模块。两条路线都要求用户登录本人账号，
主项目不接收 ChatGPT 密码、Cookie、Access Token 或私有 API 数据。

| 路线 | 会话位置 | 当前用途 | 状态 |
|---|---|---|---|
| Win 本地 WebView2 | 用户 Windows 设备 | 官方网页、手动登录、本地 Profile | ChatGPT 首版已接线 |
| 外部托管模块 | 商户模块服务器 | 浏览器/PWA 的隔离远程会话 | 保留既有能力发现 |
| APK 一龙界面 | Android 本地 WebView | 消费去凭证化语义事件 | ChatGPT 语义适配器已接线 |

## 用户入口

三个客户端统一使用“ChatGPT 账号与聊天”，不再要求用户先理解“厂商账号”、
“个人浏览器”或“Win CLI”：

- APK：聊天侧栏设置或个人资料 → ChatGPT 账号与聊天；进入后先打开官方页面登录，
  登录完成可留在官方网页或切换“一龙界面”。
- PWA：我的 → ChatGPT 账号与聊天；PWA 在新标签打开官方 ChatGPT，受同源隔离影响，
  不宣称可以读取登录状态或重渲染官方页面。
- Win：左侧 ChatGPT 登录，或账号设置 → ChatGPT；本地 WebView2 保存会话，打开窗口后
  直接完成登录和聊天。

这里的“登录”是设备内的官方网页会话，不是把 ChatGPT 云端账号或 Cookie 绑定到一龙
云端账号。登录状态由官方页面确认。

## Win 本地模式

桌面壳固定登记厂商入口，PC 前端只能传 `providerId` 和当前一龙 `ownerKey`，不能传
任意 URL。Rust 宿主把 `ownerKey` 做稳定不可逆指纹后建立如下本机目录：

```text
app-local-data/
└── ai-web-profiles/
    └── <owner-fingerprint>/
        └── chatgpt/
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

### IPC 与导航边界

- `build.rs` 只把三个本地会话命令登记进 Tauri App Manifest。
- `capabilities/main.json` 只向 `main` 窗口和项目批准的 PC 地址开放该权限。
- 每个 Rust 命令再次检查调用 WebView 标签必须等于 `main`。
- ChatGPT 窗口没有匹配的 capability，也没有初始化脚本或一龙语义桥。
- 顶层导航仅接受 HTTPS、443、无 URL 凭据的 ChatGPT/OpenAI 域名及精确身份主机。
- Cloudflare 或身份提供商验证由用户本人完成；应用不绕过、不自动点击。
- 身份提供商可以拒绝嵌入式浏览器，应用不得伪装 User-Agent 或转移 Cookie 规避。

本地模式当前登记 `chatgpt`。Gemini 暂不登记：Google OAuth 官方政策禁止应用把授权请求
导向开发者可控制的嵌入式 user-agent，必须先设计系统浏览器/官方 API 路线，不能为了
页面可登录而扩大 WebView 权限。

## 统一原生渲染协议

`unifiedAiProtocol.ts` 定义 `yilong.ai.ui.v1`，只表达用户可见语义：

- adapter ready 与厂商能力；
- 会话 ID/标题变化；
- 消息快照与流式文字增量；
- idle/thinking/streaming/waiting/error 状态；
- 文本、图片、文件与引用内容块。

协议不定义 Cookie、Authorization、Access Token、原始请求头或网络响应。Win Rust
宿主当前仍返回 `rendererStatus=reserved`；Android 已通过来源受限的 WebMessage 桥接入
ChatGPT 可见语义适配器。后续每个厂商适配器仍必须独立评审：

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

- Tauri crate 编译与 4 项本地宿主测试通过。
- PC TypeScript/Vite 生产构建、ESLint 和本地浏览器安全契约测试通过。
- 未登录一龙账号时不能创建本地 Profile。
- 同一账号/厂商复用窗口与 Profile，不同一龙账号使用不同指纹目录。
- 用户确认本人账号后才能打开本地会话，清除会话前再次确认。
- 普通浏览器不显示可调用的本地命令，继续使用托管模式。
- 尚未进行真实 ChatGPT 登录、Cloudflare、下载、音视频、更新后兼容和安装包验收。
- Win 尚未启用 DOM 语义适配器或原生 UI 重渲染；APK 已接线但尚未完成真实账号兼容验收。
