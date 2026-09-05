---
title: "ESK 平台认证 Sui 地址绑定 V2"
status: accepted
implementation_status: verified
owner: platform-assets, protocol
priority: p0
reviewed_at: 2026-09-05
decision_refs:
  - "docs/requirements/esk-first-user-delivery-roadmap-v1.md"
  - "docs/requirements/esk-sui-address-control-proof-v1.md"
  - "docs/requirements/esk-platform-recorded-assets-v1.md"
---

# ESK 平台认证 Sui 地址绑定 V2

## 用户结果

已登录的一龙用户可以为本人指定一个规范 Sui testnet 地址，取得短时个人消息挑战，
用支持的单签钱包签名后提交。服务端在本地重新验证地址控制权，并在同一 SQLite
事务内重新确认当前会话、挑战有效期、未消费状态、用户唯一性和地址唯一性，随后
追加一条不可变绑定记录。用户可只读查询本人绑定结果。

成功只表示“当前平台账户曾控制该 testnet 地址并消费了这次挑战”。它不表示 ESK
已经发布、该地址持有 ESK、平台余额已迁移、链上交易已确认，亦不授予 claim、转账、
量化申购、卖回或资金操作权限。

## 依赖与复用

1. 复用 V1 的 challenge 与 wallet response 字节合同，不建立第二套签名消息。
2. 复用正式平台账本的真实用户会话校验；管理员 token、owner 静态 token、
   `local-owner` 和非 active 用户均不得使用本人绑定接口。
3. 依赖 `esk-sui-address-control-proof-v1` 与 `esk-platform-recorded-assets-v1` 已完成。
4. 只支持 `network=testnet`、`purpose=user_asset_migration`、ED25519、Secp256k1、
   Secp256r1。MultiSig、zkLogin、Passkey、未知方案和主网全部失败关闭。

## HTTP 合同

所有接口均要求唯一 Bearer 会话，并返回 `Cache-Control: no-store`、
`Pragma: no-cache` 和 `Referrer-Policy: no-referrer`。

### 创建挑战

`POST /api/me/assets/esk/platform/sui-address-binding/challenges`

请求只允许：

- `schema=yilong.esk.sui.platform_address_binding_request.v2`；
- 规范、非零的 `0x` + 64 位小写十六进制 `address`；
- `ttl_seconds`，范围 120 至 900 秒。

客户端不得提交 user ID、subject commitment、nonce、时间或 challenge ID。服务端使用
操作系统随机源生成 32 字节 subject seed 与 32 字节 challenge nonce；subject seed
只用于生成随机不可逆 commitment，原始 seed 不落库。subject commitment 与私有 user
映射保存在平台数据库，公开挑战继续使用 V1 精确消息格式。

### 完成绑定

`POST /api/me/assets/esk/platform/sui-address-binding/challenges/:challenge_id/complete`

请求为 V1 wallet response 的四个精确字段：`schema`、`challenge_id`、
`message_base64`、`signature`。路径 ID 与响应 ID 必须相同。服务端必须：

1. 从私有账本读取当前用户的不可变挑战并按 V1 重构；
2. 重算消息、challenge ID、SHA-256，检查未到期且尚未消费；
3. 按 Sui personal-message intent 对消息做 BCS byte-vector 封装和 Blake2b-256；
4. 从 `flag || signature || public_key` 恢复允许的单签方案，验证签名并从
   `Blake2b-256(flag || public_key)` 派生地址；
5. 在写入事务内再次验证会话、挑战、时间、用户与地址唯一性；
6. 原子追加绑定，挑战 ID 唯一即代表一次消费已记录。

同一 challenge 与完全相同 wallet response 可幂等重放并返回原绑定；同一 challenge
更换任何响应内容、同一用户绑定另一地址、同一地址绑定另一用户、过期挑战、已撤销
会话及并发第二写入均失败关闭。V2 不提供换绑、解绑或覆盖更新；未来恢复政策必须另立
需求并保留旧绑定审计链。

这里的“并发第二写入”指不得产生第二条账本记录：完全相同响应的竞争请求可以返回同一
绑定并标记为幂等回放；响应不同的竞争请求固定冲突失败。

为限制追加式账本增长，同一用户和地址存在未过期挑战时直接返回原挑战，不追加新行；
每个用户最多同时保留 3 个未过期挑战，并且任意滚动 24 小时最多新建 20 个挑战。
超过任一上限固定返回 `429 ESK_PLATFORM_SUI_BINDING_RATE_LIMITED`，不泄漏当前计数。
这些上限在创建挑战的同一写事务中复核，不能只依赖进程内限流器。

### 查询本人绑定

`GET /api/me/assets/esk/platform/sui-address-binding`

未绑定返回 `status=unbound`。已绑定只返回规范地址、网络、签名方案、绑定时间、
绑定回执摘要及下列真实性标志，不返回 user ID、session、nonce、完整签名、完整钱包
响应或内部 subject commitment：

- `address_control_verified=true`；
- `platform_subject_authenticated=true`；
- `challenge_single_use_recorded=true`；
- `chain_finality_verified=false`；
- `asset_identity_verified=false`；
- `balance_eligible=false`；
- `manifest_transition_allowed=false`。

创建、完成和查询成功统一返回 HTTP 200。创建响应就是 V1 challenge 的 12 个精确
字段。完成和已绑定查询统一返回下列精确公共合同；未绑定查询仅返回相同 schema 与
`status=unbound`：

- `schema=yilong.esk.sui.platform_address_binding.v2`；
- `status=bound`、`network=testnet`、`address`、`signature_scheme`、`bound_at`；
- `binding_receipt_sha256=sha256:<64 lowercase hex>`；
- 上述七个真实性布尔值。

`binding_id` 只在私有账本使用：对 UTF-8 文本
`YILONG_ESK_SUI_PLATFORM_BINDING_ID_V2\nchallenge_id=<id>\nresponse_digest=<digest>`
执行 SHA-256，取前 32 位小写十六进制并加前缀 `eskpsb_`。绑定回执摘要对下列固定
顺序、LF 分隔、无尾随换行的 UTF-8 文本执行 SHA-256：

```text
YILONG_ESK_SUI_PLATFORM_BINDING_RECEIPT_V2
binding_id=<private binding id>
challenge_id=<V1 challenge id>
subject_commitment=<private subject commitment>
address=<canonical testnet address>
network=testnet
message_sha256=<digest>
signature_scheme=<ed25519|secp256k1|secp256r1>
signature_sha256=<digest>
response_digest=<digest>
verified_at=<UTC RFC3339 milliseconds Z>
bound_at=<UTC RFC3339 milliseconds Z>
```

只有摘要通过 HTTP 返回；原文及其中的私有字段不出服务端。

## 私有追加式账本

新增三类表：

1. 用户与随机 subject commitment 的一对一私有映射；
2. 不可变短时挑战；
3. 同时充当 challenge consumption 的不可变地址绑定。

挑战、绑定和 subject 映射禁止 UPDATE、DELETE 和命中既有唯一键的
`INSERT OR REPLACE`。绑定表对 `challenge_id`、`user_id` 和
`address` 分别唯一，并用外键和插入触发器核对 challenge 的 user、subject、address、
message digest 与有效时间。完整 wallet response 仅保存在私有账本供之后证据复核；
HTTP 和日志不得回显。数据库错误只投影固定错误码。

## 安全与真实性边界

- 不读取钱包配置、私钥、助记词、keystore、剪贴板或环境秘密。
- 不实例化 Sui RPC client，不联网验证，不构建、签名、广播或执行交易。
- 不读取或修改真实付款、ESK 余额、卖回、量化、USDT 或 Binance 数据。
- 不改变 `/api/me/assets/esk/platform` 的 `source=platform_recorded`、
  `chain_status=not_deployed` 和全部链迁移能力关闭状态。
- 当前 Android 私有 API 仍要求安全传输；本功能不得以放宽明文 HTTP Bearer 传输
  作为“可用性修复”。
- 没有真实链发布与终局证据时，任何界面都不得显示“已上链”或链上余额。

## 验收标准

1. 三种单签均用固定跨实现向量通过；错误 flag、长度、public key、地址、消息、签名、
   高 S ECDSA 和非规范 Base64 均拒绝。
2. 服务端生成的挑战与 V1 固定消息、challenge ID 和 digest 完全一致；客户端无法选择
   subject commitment、nonce、时间、用户或 challenge ID。
3. SQLite 合成测试覆盖首次绑定、精确幂等重放、篡改重放、过期、未来时间、会话撤销、
   跨用户、用户/地址唯一性、并发消费、同地址未过期复用、3 个并发上限、滚动 24 小时
   上限及 UPDATE/DELETE/INSERT OR REPLACE 失败。
4. 进程内 HTTP 测试覆盖未登录、静态管理员 token、停用用户、未知字段、超限 body、
   创建/完成/本人读取和 no-store；响应不泄漏私有字段。
5. 完成绑定前后正式 ESK 账户余额和 `platform_recorded/not_deployed` 投影完全不变。
6. Rust 格式、定向 harness、HTTP、迁移、旧 V1 Sui 合同、源码规模、文档与功能登记
   漂移门禁通过；代码推送、后端部署和真实用户验收分别报告。

## 明确不做

- 不接 Android 钱包 UI，不要求用户提供真实钱包或签名做本批验收。
- 不实现主网、项目职责多签、换绑恢复、地址轮换或多人审批。
- 不发布 Sui 包，不分配 ESK，不生成链余额、迁移 manifest 或终局性结论。
- 不导入历史付款，不写真实用户余额，不进行 USDT、币安、量化或任何资金操作。
- 不签名、构建或上传量化 APK。

## 后续交接

真实 testnet 发布得到包、Currency、分配、源码与 checkpoint 终局证据后，另立
Evidence/Manifest V2 把本绑定、三观察器与批准迁移清单组合。只有完成反向结转和
双计防护后，平台资产才可进入“迁移中”；只有链上证据完整时才可进入“已上链”。
