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
  ├─ password_hash (PBKDF2-SHA256；旧 SHA256 登录成功后升级)
  ├─ account_recovery_codes (只保存哈希，一次性使用)
  ├─ sessions (一龙会话，可逐个撤销)
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

## 账号安全中心 V1

Win、Android 与移动 Web 共用以下一龙账号合同：

- `GET /api/auth/security`：返回密码、离线恢复码数量和当前有效会话。
- `PUT /api/auth/password`：联合登录账号可首次设置密码；已有密码必须验证当前密码。成功后保留当前会话并撤销其他会话。
- `POST /api/auth/recovery-codes/rotate`：明确确认后生成 8 个一次性恢复码；旧码立即撤销，明文只在本次响应显示。
- `POST /api/auth/password/recover`：用账号、离线恢复码和新密码恢复；成功后消费恢复码并撤销全部旧会话，用户必须重新登录。
- `GET /api/auth/sessions`、`DELETE /api/auth/sessions/:session_id`、`POST /api/auth/sessions/revoke-others`：设备会话查看与撤销。
- `POST /api/auth/logout`：撤销当前会话，而不是只清理客户端本地状态。
- `POST /api/auth/password/recovery/start`：邮件/短信恢复预留合同。当前固定返回 `delivery_configured: false`，不查询并泄露账号是否存在。
- `GET /api/auth/security/events`：分页返回当前账号的脱敏安全事件；只暴露 session/request 是否存在，不返回其原值。
- `GET /api/auth/account-export/manifest`：返回可导出的项目/工作区与安全事件清单，不把密码、会话、OAuth、Provider 凭据或保险箱密文放进导出包。
- `POST /api/auth/account-deletion/preflight`：只检查项目所有权、共享成员和 Codex 保险箱槽位等阻塞项；当前固定 `deletion_execution_available: false`，不会删除或停用账号。
- `GET /api/auth/safety/capabilities`：公开认证限流与联合登录完成重放所用后端，以及多副本上线前必须补齐的共享原子 TTL 存储能力。

新密码使用带随机盐、版本和迭代次数的 PBKDF2-HMAC-SHA256；旧版盐化 SHA256 仍可验证，但只在一次成功密码登录后就地升级。恢复码服务端只保存 SHA256 哈希、末四位和审计信息，不提供再次读取。账号安全变更通过 `request_id` 保证存储级幂等，并写入追加式 `auth_security_audit`；恢复入口另有进程内速率限制，生产部署仍必须配置网关/IP/WAF 周边限流。

## Win / Android / Web 统一合同

### 发现公开能力

`GET /api/auth/federation/providers`

服务端仅在配置 `ELON_GOOGLE_OIDC_CLIENT_ID` 或逗号分隔的 `ELON_GOOGLE_OIDC_CLIENT_IDS` 后返回 `configured: true`。第一个 client ID 用于 Web GIS 和 Android Credential Manager 的 server client ID，其余 ID 只用于服务端受众验证。

### 登录或绑定

1. 客户端请求 `POST /api/auth/federation/google/challenges`，提交 `mode`（`login` / `bind`）、`platform`、`request_id` 和稳定的非秘密 `client_instance_id`。
2. 服务端返回十分钟有效的 challenge 与高熵 nonce；数据库仅保存 nonce 哈希。
3. 客户端把 nonce 交给 Google 官方组件，获得 Google ID token。
4. 客户端请求 `POST /api/auth/federation/google/complete`，重试同一次完成请求时复用同一 `request_id`。
5. 服务端从 Google JWKS 验证 RS256 签名、issuer、audience、有效期、已验证邮箱和 nonce。
6. `login` 返回新的一龙 session；`bind` 必须同时携带当前一龙 Bearer session。

身份管理：

- `GET /api/auth/identities`
- `DELETE /api/auth/identities/:identity_id`

Win React、Android APK 与移动 Web 使用相同响应字段。Android 不打开自建 WebView 登录 Google，而使用系统 Credential Manager；Web 使用 Google Identity Services JavaScript。

### APK / PWA 账号绑定入口

- 已登录用户从个人页的“账号与安全”进入，页面必须先展示脱敏后的当前一龙账号，再允许启动 Google 官方流程；`bind` 不得在未登录状态降级为新账号登录。
- 个人页和账号安全页都展示 Google 的“已绑定 / 未绑定 / 暂未配置”状态。服务端返回 `configured: false` 时客户端明确提示管理员配置缺失，不显示无法完成的伪按钮。
- 绑定前文案明确 Google 会成为当前一龙账号的新登录方式；同邮箱不自动合并，身份属于另一账号时按稳定错误码提示先去原账号解绑。
- APK 只把短时 ID token 交给服务端验证，PWA 只使用 Google Identity Services 官方组件；两端均不持久化 Google ID token、access token、refresh token 或密码。

联合登录创建 challenge 和完成操作均有进程内有界限流，超限返回稳定错误码及 `Retry-After` 并写入脱敏审计。完成响应使用 5 分钟、最多 256 项的进程内精确重放缓存，缓存键绑定 challenge、request、客户端指纹；它允许断线后拿回同一结果，但不跨服务进程持久化 Bearer session。生产多副本部署需要把幂等结果迁到共享短时存储，或者由会话粘性和周边重放防护补足。

限流已通过 `AuthRateLimitStore` 抽象与业务入口解耦，但默认实现仍是最多 4096 个键的进程内有界内存。能力接口明确返回 `multi_replica_ready: false`；只有提供原子递增、TTL 和有界 keyspace 的共享实现，并为含 Bearer 的短时重放结果提供加密临时存储或稳定单写者后，才能宣称多副本安全。

## AI Provider 控制面 V2

`GET /api/ai-provider-accounts` 返回 `elon.ai_provider_accounts.v2`：

- 每个 Provider 明确公布 login、logout、remote login、idempotent start、restart recovery、credential export、web chat 能力。
- 登录启动接受 `request_id`，同一 Provider 的同一请求键重复提交时返回原任务。
- 登录任务脱敏后写入节点数据根目录的 `control-plane/provider-auth-attempts.json`。
- 日志不保存验证码、带 query/fragment 的授权 URL 或厂商 token。
- 节点重启后，原活动任务恢复为 `failed + node_restarted`，客户端可准确提示重新发起。
- 日志保留 24 小时，且最多保存 64 条任务。
- 状态机固定为 `starting -> waiting_for_user -> completed/failed/canceled/expired`；终态不可被迟到事件覆盖，失败/取消/过期会显式给出 `retryable` 和 `next_action`。
- `GET /api/ai-provider-accounts/diagnostics` 只返回 CLI 探测摘要、最近任务的状态/错误码和日志合同，不返回验证码、授权地址或 token。
- 诊断同时公布不访问真实账号的 `fake_provider_matrix`，覆盖 Codex/Gemini 成功、拒绝、错误 login id、噪声、进程提前退出、取消、15 分钟过期和节点重启恢复。
- Provider journal 使用同目录临时文件原子替换并保留上一个有效备份；主文件损坏时只从合法备份恢复，活动任务仍转为安全失败而不是假定登录完成。

### 凭据保险箱合同

CLI 登录成功不等于同意上传凭据。Provider V2 为每个 Provider 返回 `credential_vault`：

- 当前只有 Codex 声明可通过现有 AES 密文保险箱备份，而且必须由用户逐次明确同意；默认 `automatic_backup: false`。
- 本机 `/api/codex-vault/backup|restore|sharing/restore|clear|delete-cloud` 均要求 8-128 位 `request_id`、`explicit_consent: true` 和逐操作确认短语；同一请求号只允许完全相同的操作/目的/目标重放。
- `GET /api/codex-vault/operations` 只保留 24 小时、最多 128 条操作元数据和稳定错误码，不保存凭据、账号标识或云端响应；重复提交不会再次写入、恢复或删除。
- Win/APK/Web UI 均不得读取或导出凭据正文。恢复只允许写入受管临时 `CODEX_HOME`，再交给官方 Codex CLI 使用。
- Gemini、Claude 与 Copilot 在没有公开、稳定的凭据导出/恢复合同前固定 `backup_supported: false`。
- 用户登录一龙账号不会自动把任一厂商凭据复制到云端，也不会使另一台设备自动获得厂商登录态。

### 官方网页版聊天预留适配器

`chatgpt_web` 与 `gemini_web` 有类型化 `WebChatAdapterDescriptor`、授权请求以及启动/刷新/撤销错误合同，但固定 `enabled: false`、`actual_state: unavailable`、`cli_login_reusable: false`、`browser_cookie_reusable: false`。预留生命周期为 `authorization_pending -> active -> expired/revoked`，未来必须具备厂商批准授权、逐次同意、最小 scope、服务端会话绑定、刷新轮换、撤销和仅元数据审计。环境变量、CLI 登录状态、浏览器 Cookie 和保险箱内容都不能绕过禁用状态。

### 设备内网页增强不是远程 Web 适配器

Android 的 ChatGPT 本地网页工作台与上述控制面 `chatgpt_web` 描述符属于不同边界：

- 官方页面、Cookie、站点存储和请求执行都留在 APK 内的 WebView Profile。
- 原生层不调用 `CookieManager.getCookie`，不把会话交给 OkHttp、Win 节点或一龙服务器。
- 只有主框架精确位于 `https://chatgpt.com` 时，来源受限的 WebMessage 桥才通过共享 `yilong.ai.ui.v1` envelope 投影用户可见的消息、输入框和生成状态。
- 原生操作只映射为发送、停止、新会话和快照四种固定本地命令；登录、验证码与 Cloudflare 仍由用户在官方网页模式完成。
- 移动 PWA 受浏览器同源策略限制，只提供官方网页入口和能力说明，不宣称可读取或重新呈现跨域页面。
- 页面适配失败时必须退回完整官方网页；该本地能力不改变远程适配器的 `enabled: false` 与 `browser_cookie_reusable: false`。

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

当前已完成的非真实账号验收：

- Rust `elon-server` 与 `elon-pc-node` 编译检查。
- 账号安全临时 SQLite 测试：联合登录账号设密码、改密幂等/会话撤销、恢复码一次性消费。
- 联合登录假数据测试：限流边界、精确重放缓存、nonce/audience 校验、本地伪 RSA 签名 Google JWT；不访问 Google 网络。
- Provider 假协议测试：状态机、重启恢复、损坏 journal 备份恢复、Codex/Gemini 成功/拒绝/错误任务/噪声消息脱敏、保险箱显式同意与精确重放、禁用 Web Chat 生命周期。
- PC Vite/TypeScript 构建与 ESLint、Android `compileDebugKotlin`、移动 Web JavaScript 语法检查。

以下项目统一在获得配置后实测：

- Google Cloud OAuth client、Android 包名/SHA 指纹、Web Authorized JavaScript origins。
- 真实 Google 账号登录、绑定、冲突和解绑。
- Codex / Gemini / Claude / Copilot 的真实官方账号登录与退出。
- ChatGPT Web / Gemini Web 内部测试接口。
- 真机、模拟器、Renderer、线上部署、APK 构建上传与发布。
