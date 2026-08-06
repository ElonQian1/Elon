# 一龙统一账号与 AI Provider 架构

## 目标边界

一龙账号负责识别“一龙用户”；Google、Codex、Gemini、Claude、Copilot 等厂商账号仍由厂商官方流程负责认证。两层身份不能混用：

- 一龙联合登录只保存 `provider + issuer + subject` 与展示信息，不保存 Google access token / refresh token。
- AI CLI 登录由本机官方 CLI 进程完成；一龙控制面只保存脱敏任务状态，不解析或导出厂商凭据。
- Codex 的 `auth.json` 仍只通过现有、显式授权的 Codex Vault 备份路径处理，不因登录一龙账号自动上传。
- `chatgpt_web`、`gemini_web` 继续作为禁用适配器保留，不能把 CLI 登录态冒充成官方网页聊天会话。

公开协议依据：Google OIDC 的稳定账号键是 `sub`，服务端必须验证签名、`aud`、`iss`、有效期和 nonce；Android 使用 Credential Manager 的 Sign in with Google。Codex 使用官方 app-server `account/login/start`、`account/read`、`account/logout` 合同。

- [Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect)
- [Google 服务端 ID token 校验](https://developers.google.com/identity/gsi/web/guides/verify-google-id-token)
- [Android Credential Manager Sign in with Google](https://developer.android.com/identity/sign-in/credential-manager-siwg-implementation)
- [Codex app-server 认证接口](https://learn.chatgpt.com/docs/app-server#auth-endpoints)

## 账号模型

```text
users (一龙账号)
  ├─ password_login_enabled
  ├─ sessions (一龙会话，可撤销)
  └─ user_identities (0..N 个联合身份)
       └─ google / canonical issuer / immutable sub
```

规则：

1. 一个一龙账号可以绑定多个登录身份。
2. 一个厂商身份只能属于一个一龙账号。
3. 不按相同邮箱自动合并；已有密码账号必须先登录，再主动绑定 Google。
4. 首次 Google 登录可创建没有密码入口的一龙账号。
5. 解绑后必须仍有密码入口或另一个联合身份，防止用户把自己锁在账号外。
6. 所有创建、绑定、冲突和解绑操作写入身份审计表。

## Win / Android / Web 统一合同

### 发现公开能力

`GET /api/auth/federation/providers`

服务端仅在配置 `ELON_GOOGLE_OIDC_CLIENT_ID` 或逗号分隔的 `ELON_GOOGLE_OIDC_CLIENT_IDS` 后返回 `configured: true`。第一个 client ID 用于 Web GIS 和 Android Credential Manager 的 server client ID，其余 ID 只用于服务端受众验证。

### 登录或绑定

1. 客户端请求 `POST /api/auth/federation/google/challenges`，提交 `mode`（`login` / `bind`）和 `platform`。
2. 服务端返回十分钟有效的 challenge 与高熵 nonce；数据库仅保存 nonce 哈希。
3. 客户端把 nonce 交给 Google 官方组件，获得 Google ID token。
4. 客户端请求 `POST /api/auth/federation/google/complete`。
5. 服务端从 Google JWKS 验证 RS256 签名、issuer、audience、有效期、已验证邮箱和 nonce。
6. `login` 返回新的一龙 session；`bind` 必须同时携带当前一龙 Bearer session。

身份管理：

- `GET /api/auth/identities`
- `DELETE /api/auth/identities/:identity_id`

Win React、Android APK 与移动 Web 使用相同响应字段。Android 不打开自建 WebView 登录 Google，而使用系统 Credential Manager；Web 使用 Google Identity Services JavaScript。

## AI Provider 控制面 V2

`GET /api/ai-provider-accounts` 返回 `elon.ai_provider_accounts.v2`：

- 每个 Provider 明确公布 login、logout、remote login、idempotent start、restart recovery、credential export、web chat 能力。
- 登录启动接受 `request_id`，同一 Provider 的同一请求键重复提交时返回原任务。
- 登录任务脱敏后写入节点数据根目录的 `control-plane/provider-auth-attempts.json`。
- 日志不保存验证码、带 query/fragment 的授权 URL 或厂商 token。
- 节点重启后，原活动任务恢复为 `failed + node_restarted`，客户端可准确提示重新发起。
- 日志保留 24 小时，且最多保存 64 条任务。

公开 Provider：

| Provider | 官方流程 | 远程登录 | 退出 | 凭据归属 |
|---|---|---:|---:|---|
| Codex CLI | app-server JSON-RPC | device code | 是 | Codex CLI |
| Gemini CLI | ACP v1 stdio | 否 | ACP 能力存在时 | Gemini CLI |
| Claude Code | 官方 CLI auth | 否 | 是 | Claude Code |
| GitHub Copilot CLI | 官方 web flow | 否 | 交互式 CLI | 系统凭据存储 |
| ChatGPT Web | 禁用适配器 | 否 | 否 | 厂商 |
| Gemini Web | 禁用适配器 | 否 | 否 | 厂商 |

## 暂缓统一实测的项目

以下代码可先通过编译、静态测试与模拟数据库测试，统一在获得配置后实测：

- Google Cloud OAuth client、Android 包名/SHA 指纹、Web Authorized JavaScript origins。
- 真实 Google 账号登录、绑定、冲突和解绑。
- Codex / Gemini / Claude / Copilot 的真实官方账号登录与退出。
- ChatGPT Web / Gemini Web 内部测试接口。
- 真机、模拟器、Renderer、线上部署、APK 构建上传与发布。
