# 游戏主账号授权 V1

状态：授权核心已发布；浏览器入口与游戏 BFF 已实现并持续验收，运营配置默认关闭。此模块为主项目用户授予独立游戏权限，不产生钱包归属、资产、利润、债权或付款授权。双方的公开 HTTPS 地址、证书和独立服务配置齐备后，才能启用实际用户登录。

## 所有权与依赖

源码：`server/src/esk_platform/game_access/`。授权直接查询正式 `users` / `sessions`，不创建另一套主账号。原会话过期、撤销或用户停用后，新的兑换与签名观测立即拒绝。静态 owner、`local-owner` 和量化 `aat_` 均不是游戏授权来源。

V292 迁移新增 `game_access_grants`、`game_access_nonces`、`game_access_audit`。不搬迁 Paper 数据、资产或余额。SQL 约束和不可变触发器保护授权绑定、单次兑换、nonce 消费和审计历史。原用户与会话记录有外键引用，不得物理删除已被授权历史引用的记录。

签名协议沿用 [SDK V1](../sdk/game-access/README.md)，Rust 协议实现位于 `protocol/`。SDK 固定测试向量被 Rust 测试直接读取，避免两个实现对标识符、数字字符串、权限顺序或签名消息有不同解释。

## 配置

`ELON_GAME_ACCESS_CONFIG` 指向运营方私有 JSON 文件，最大 16 KiB。缺失返回 `503 game_access_disabled`；读取或解析失败返回 `503 game_access_unavailable`。不要提交真实配置、服务凭据或签名种子。

必需字段：

| 字段 | 规则 |
| --- | --- |
| `schema` | `esk.game.access.policy.v1` |
| `main_issuer` | 部署后稳定的主项目发行者 ID，1–128 ASCII 字符，允许字母、数字、`_.:-` |
| `client_id` | `esk-game.web` |
| `redirect_uri` | 唯一已登记的规范 HTTPS URL，路径严格为 `/api/account/callback`；无用户信息、查询或 fragment |
| `service_secret_sha256` | 独立 BFF 服务凭据的 SHA-256，小写 64 位十六进制 |
| `key_id` | 固定签名密钥 ID，格式同发行者 ID |
| `signing_seed_hex` | 独立 Ed25519 种子，小写 64 位十六进制 |

游戏 BFF 私下配置原始 `egs_` 加 64 位小写十六进制服务凭据；主项目仅保存其摘要。游戏必须由运营方固定主项目 HTTPS 地址、发行者、key ID 和公钥，不能相信网络响应自行提供的公钥。密钥必须使用安全随机源生成；测试种子不可用于部署。

授权绑定整个配置指纹。更换回调、服务凭据、发行者或签名密钥会使旧授权失效；先部署新的双方配置，再重新登录。当前没有自动密钥发现、静默降级或 refresh token。

## 传输

五个接口都是 POST / JSON，正文上限 16 KiB，拒绝重复或未知 JSON 字段和任何查询参数。接口必须经 Rust 原生 TLS 监听器建立的 `VerifiedAssetTransport` 到达。普通 HTTP 和伪造代理头返回 426。带 `Origin` 的请求只能来自该 TLS 监听器登记的主项目 origin；重复 Origin 拒绝。

响应设 `Cache-Control: no-store`、`Pragma: no-cache`、`Referrer-Policy: no-referrer`。私人接口不启用跨域凭据 CORS。游戏 BFF 服务端通信不发送浏览器 Origin，且总是提供独立服务凭据。

## 授权、兑换与观测

### 浏览器入口

Rust 原生 TLS 监听器提供 `/pc/game-access?request=<编码的授权请求 JSON>`、`/pc/game-login?next=<编码的本地授权页路径>` 及构建后的 `/pc/assets/`。页面来自正常发布的 `pc-next-dist`，不复制旧 PC HTML。TLS 入口仅增加这些页面和专用登录 API，不开放整个旧版主项目路由，也不修改节点所有者的 `/api/auth/login` 协议。

`POST /api/game-access/v1/login` 的严格正文仅为 `{"account":"已有主账号","password":"密码"}`，最大 4 KiB。要求真实 TLS 和唯一同源 Origin，不接受查询参数。它调用主项目原始密码认证与会话服务，返回原始 `users.id`，不创建游戏专用密码或虚拟用户；不开启 remember-device。不存在、停用和密码错误使用同一错误。按进程限制每账号每分钟 5 次、总计 120 次，并限制 4 个并发密码计算；重启清空临时限流窗口。

界面始终向当前主项目 origin 请求，不沿用本机节点 API 地址覆盖。密码不进入 URL；主会话只存于主项目页面既有账号存储。游戏只取得一次性 code，凭据在 Rust BFF 内存中保存。登录页目前支持已有密码账号；第三方账号注册、找回和密码设置由主项目已有账号流程完成。

授权页展示游戏 origin、权限和期限，用户必须单独勾选并确认。账号或授权请求改变时，先前勾选不再有效。回复必须匹配原请求的 state、回调、权限及有效期；页面不会根据任意 `next` 或返回 URL 外跳。HTTP 预览只用于界面检查，登录与授权按钮不会发送凭据。

页面与 API 返回 no-store/no-referrer；页面禁止被嵌入 frame。运营方需要同时启用原生 TLS 监听器，并将 `PUBLIC_URL` 配为该 HTTPS origin；无需启用节点所有者 bootstrap/session 功能。证书必须被使用者浏览器信任，不以忽略证书错误完成验收。

1. 游戏后端生成浏览器绑定的随机 state、PKCE verifier 和流程 Cookie；短期保存 verifier，发送其 S256 challenge。用户在主项目界面明确同意所列权限。
2. 主项目界面携带当前主用户 Bearer 会话、同源 Origin，调用 `POST /api/me/game-access/authorize`。
3. 主项目返回一次性 `egc_` code、原 state、固定 redirect、grant ID 和期限。界面只跳转服务端确认的 redirect，将 code 与 state 交给游戏回调；不得传主会话给游戏。
4. 游戏后端严格验证回调 state 与流程 Cookie，调用 `POST /api/game-access/v1/token`。成功后 code 已消费，令牌仅保留游戏后端，不进入 URL、浏览器存储或日志。
5. 游戏后端为每次授权检查构造新 challenge，调用 `POST /api/game-access/v1/observe`，并使用固定公钥校验返回的 Ed25519 签名。挑战绑定具体令牌摘要、nonce、动作和参数；不得复用旧观测执行新动作。

授权正文：

```json
{
  "schema": "esk.game.access.authorize.v1",
  "client_id": "esk-game.web",
  "redirect_uri": "https://game.example.test/api/account/callback",
  "state": "由游戏后端生成的43至128位随机非保留字符",
  "code_challenge": "S256结果的43位无填充base64url",
  "code_challenge_method": "S256",
  "scopes": ["play", "inventory_read"],
  "expires_in_seconds": 900,
  "explicit_consent": true,
  "confirmation": "授权此游戏使用我的主账号及所选权限"
}
```

示例中的 state 和 challenge 是说明文字，不能直接提交。scope 必须按 `play`、`inventory_read`、`redeem`、`principal_withdraw` 顺序提供且无重复，必含 `play`。界面默认只申请实际需要的权限；`redeem` 不包含提取本金。

兑换正文含 `schema=esk.game.access.exchange.v1`、`grant_type=authorization_code`、`client_id`、`redirect_uri`、`state`、`code` 和 `code_verifier`；请求头 `x-esk-game-service` 提供 BFF 凭据。code 最长 120 秒，grant 最长 15 分钟，均不超过原主会话寿命。

观测请求使用 SDK 的完整 V1 challenge，额外提供 `Authorization: Bearer <egt_…>` 和 `x-esk-game-service`。主项目在同一个 SQLite 写事务内检查原会话、授权和 scope，消耗 nonce 并签名。游戏使用真实当前时间验证观测：最多 10 秒有效，只对观测时间容忍最多 2 秒未来偏差，不放宽授权起止时间。

## 撤销与恢复

| 场景 | 行为 |
| --- | --- |
| 用户撤销 | 主用户会话调用 `/api/me/game-access/grants/:grant_id/revoke`，只能撤销自己的授权 |
| 游戏退出 | 游戏 BFF 立即撤销本地权限，再调用 `/api/game-access/v1/revoke`；主会话已结束也允许关闭这份游戏令牌 |
| 撤销正文 | `{"schema":"esk.game.access.revoke.v1","expected_revision":"1"}`；重复撤销幂等成功 |
| code 兑换响应丢失 | code 保持已消费，不能重发同一 code 取得新 token；重新开始授权 |
| 观测响应丢失 | nonce 保持已消费；以新 nonce 重新核验，不猜测成功 |
| 数据库失败 | 授权、兑换、nonce 与审计所在事务整体回滚；不返回成功授权 |
| 游戏重启/失联 | 游戏端应丢弃内存令牌与许可；重新登录到持久绑定的同一用户角色；不得回退为 Paper 身份 |
| 主项目重启 | 持久化 token 摘要可继续核验，但仍逐次检查原会话和策略 |
| 已返回观测后撤销 | 已签名证明不能从网络中收回；游戏端使用不超过 5 秒的运行租约并持续刷新，经济动作在执行前再次核验 |

nonce 冲突与版本冲突返回 409；权限不足 403；无效凭据与失效授权 401；容量限制 429；存储等不确定错误 503。不要把这些错误转换成空余额、自动成功或 guest 的经济权限。

每用户最多 64 个未过期未撤销授权，每 grant 最多 2048 次观测。它们是本地容量保护，不替代部署前的用户/API 速率限制和历史归档容量规划。

## 验证入口

使用仓库标准有日志执行器包裹以下校验命令：

```powershell
powershell -NoProfile -File scripts/validate-rust.ps1 -- test --manifest-path server/tests/game-access-harness/Cargo.toml
powershell -NoProfile -File scripts/validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server game_access -- --test-threads=1
node --test sdk/game-access/test/session.test.js
node --experimental-strip-types --test pc-frontend/scripts/test-game-access.mjs
```

独立 harness 执行实际生产领域代码、SQL 和签名，覆盖并发单次兑换、回滚、重开数据库、失效和策略隔离，但不证明完整服务端路由已通过。真实 Store / TLS 路由测试另在 `game_access/http_tests.rs` 与 `browser_tests.rs`；后者验证密码登录返回原用户、授权衔接、页面和静态资源、禁止跨站及普通 HTTP。最终验证状态以本次执行回执为准。

PKCE 字符与 S256 规则依据 [RFC 7636](https://www.rfc-editor.org/rfc/rfc7636.html)；固定回调与授权响应处理依据 [OAuth 安全实践 RFC 9700](https://www.rfc-editor.org/rfc/rfc9700.html)。此接口是首方游戏的受限授权协议，未宣称完整 OAuth/OIDC 服务实现。
