---
title: "ESK Sui 用户地址控制证明 V1"
status: accepted
implementation_status: verified
owner: platform-assets, protocol
priority: p0
reviewed_at: 2026-09-05
decision_refs:
  - "docs/decisions/esk-sui-economic-foundation-v1.md"
  - "docs/requirements/esk-first-user-delivery-roadmap-v1.md"
---

# ESK Sui 用户地址控制证明 V1

## 用户结果

首批 ESK 用户可以收到一份只绑定本人平台账户承诺、目标 Sui testnet 地址和用途的
短时挑战，用任意受支持的 Sui 单签钱包签署个人消息。项目方可在完全离线、不读取
私钥、不创建交易的环境中验证：该签名确实来自所声明地址的控制者，并生成可复算的
地址控制候选证据，供之后的用户迁移审批与 Evidence/Manifest V2 使用。

本功能只证明“某个 Sui 地址对这段短时个人消息完成了有效签名”。它不证明平台账户
已经完成认证、不防止挑战在多个独立工具实例中重放、不证明地址当前余额、ESK 资产
身份、链上终局性或用户迁移资格，也不会把平台余额升级成链上余额。

## 依赖与权威边界

1. 复用已接受的 ESK Sui 创世与首批用户路线图；不修改 Currency、六桶、锁仓、
   Paper、正式平台登记、卖回或 QSHARE 合同。
2. 只接受 `network=testnet` 和 `purpose=user_asset_migration`。主网地址绑定、项目职责
   多签与恢复演练必须另立需求并取得对应批准。
3. 使用官方 `@mysten/sui` TypeScript SDK 的个人消息验证，并精确锁定依赖版本和
   npm 完整性。运行时不实例化 RPC client、钱包或交易构造器，不联网。
4. V1 只接受 `ED25519`、`Secp256k1` 和 `Secp256r1` 单签。MultiSig、zkLogin、Passkey
   及未知签名方案失败关闭；不得为了“兼容钱包”静默联网验证。
5. 平台账户只以服务端产生的不可逆 `sha256:<64 lowercase hex>` subject commitment
   进入挑战，不写入用户 ID、手机号、邮箱、付款资料或认证 token。离线工具不负责
   判断该 commitment 是否来自当前已认证会话。

## 短时挑战合同

挑战请求使用 `yilong.esk.sui.address_binding_challenge_request.v1`，根对象拒绝未知字段，
精确包含：

- `network=testnet`；
- `purpose=user_asset_migration`；
- 非全零 `subject_commitment=sha256:<64 lowercase hex>`；
- 完整、非零、规范化为 64 位小写十六进制的 Sui 地址；
- `ttl_seconds`，范围 120 至 900 秒。

工具使用系统密码学随机源生成 32 字节 nonce。挑战输出
`yilong.esk.sui.address_binding_challenge.v1`，固定包含 challenge ID、上述绑定、严格 UTC
毫秒时间、规范 Base64 nonce、精确待签 UTF-8 消息及其 SHA-256。challenge ID 从完整
待签消息摘要派生，不能由调用方选择。精确算法为：对下述消息的 UTF-8 字节执行
SHA-256，`message_sha256` 写成 `sha256:<64 lowercase hex>`，`challenge_id` 写成
`eab1_` 加同一摘要的前 32 个小写十六进制字符（前 16 字节）。

待签消息为固定顺序、LF 分隔、无尾随换行的 ASCII/UTF-8 文本：

```text
YILONG_ESK_SUI_ADDRESS_BINDING_V1
network=testnet
purpose=user_asset_migration
subject_commitment=sha256:<digest>
address=0x<64 lowercase hex>
nonce_base64=<canonical base64>
issued_at=<UTC RFC3339 milliseconds>
expires_at=<UTC RFC3339 milliseconds>
```

任何字段、顺序、大小写、换行或字节变化都会改变 message digest 并使验证失败。

## 钱包响应与离线验证

钱包响应使用 `yilong.esk.sui.address_binding_wallet_response.v1`，只接受：

- `challenge_id`；
- 钱包实际签署并回传的 `message_base64`；
- Sui serialized personal-message `signature`。

验证时必须重新构造挑战、重算 challenge ID/message digest、逐字节比较钱包消息，并在
当前时间仍位于挑战有效区间内时调用官方 SDK。签名必须能以声明地址验证，且解析出的
方案属于 V1 白名单。错误地址、错误 intent、交易签名、篡改消息、过期/未来挑战、
非规范 Base64、超长输入、未知字段或 SDK 异常全部失败关闭。

成功输出 `yilong.esk.sui.address_control_evidence.v1`，保留可独立复算所需的挑战、钱包
消息和 serialized signature，并记录验证时间与签名方案。输出必须精确标记：

`evidence_sha256` 的精确算法为：移除根对象唯一的 `evidence_sha256` 字段；对每层
对象的固定 ASCII 字段名按升序排列，数组保持原顺序，使用标准 JSON 字符串转义且不加
空白，得到 UTF-8 字节；执行 SHA-256 后写成 `sha256:<64 lowercase hex>`。任何实现
不得依赖输入 JSON 的原始字段顺序或空白。固定 challenge 与 canonical JSON golden
vector 位于 `scripts/esk-sui-address-binding/tests/golden.test.js`。

- `address_control_verified=true`；
- `platform_subject_authenticated=false`；
- `challenge_single_use_recorded=false`；
- `chain_finality_verified=false`；
- `asset_identity_verified=false`；
- `balance_eligible=false`；
- `manifest_transition_allowed=false`。

因此该文件只能成为后续服务端事务和人工审批的输入，不能直接作为入账、迁移或产品
展示依据。生产接入必须在同一数据库事务中验证当前用户、未过期挑战、一次消费、地址
唯一/换绑政策、审批和幂等迁移，并把用户身份映射保存在私有账本中。

## CLI 与安全边界

提供两个离线命令：

```text
node scripts/prepare-esk-sui-address-binding.js challenge <request.json>
node scripts/prepare-esk-sui-address-binding.js verify <challenge.json> <wallet-response.json>
```

- `challenge` 只输出公开挑战，不读取钱包、助记词、私钥、签名或网络配置。
- `verify` 只读取两个显式文件，不扫描环境变量、Sui 配置、keystore 或剪贴板。
- 除显式 `--help` 的说明文本外，标准输出只包含机器可读挑战/证据；错误只返回固定错误码，
  不回显输入、签名、路径、
  SDK 原始错误或环境内容。
- 单个输入文件最大 64 KiB；符号串、Base64 和时间均有严格上限。
- 工具不写数据库、不消费挑战、不签名、不广播、不查询 RPC、不移动 ESK/SUI/USDT。

## 验收标准

1. 三种允许的单签方案各有运行时正例，地址从验证公钥恢复并与声明地址一致；错误地址、
   签名和消息逐项拒绝。
2. challenge/message/challenge ID 可确定性复算，nonce 由密码学随机源生成；缺字段、未知
   字段、非规范地址/Base64/时间、TTL 边界和 64 KiB 上限均有负例。
3. 过期、尚未生效、challenge ID 漂移、钱包回传字节不一致和交易 intent 签名失败关闭；
   MultiSig、zkLogin、Passkey 与未知方案在零网络请求下拒绝。
4. 成功证据自包含且可再次验证，但所有账户认证、一次消费、资产、终局性、余额与
   manifest 晋级标志保持 false；不得把候选证据称为已完成用户迁移。
5. SDK 版本、lock 完整性和依赖来源固定；源码静态门禁确认没有 RPC client、交易执行、
   私钥/助记词读取或网络调用。
6. CLI 正常及失败退出码、固定错误投影、无输入泄漏和无网络执行均通过；旧 publication、
   currency、allocation 观察器与创世/Move 回归不受影响。

## 明确不做

- 不接入真实用户账号、Android 钱包连接或生产数据库，不保存/消费挑战。
- 不验证项目职责多签阈值、恢复能力、UpgradeCap/MetadataCap 托管或真实参数批准。
- 不验证 Sui 委员会签名终局性、`verify-source`、包身份、六桶或链上余额。
- 不发布 testnet/mainnet，不签名、不广播、不执行交易或资金操作。
- 不启用链上余额、claim、反向结转、USDT 兑换、量化申购、收益或卖回。

## 后续交接

服务端地址绑定 V2 必须把本挑战嵌入已认证会话并原子记录一次消费；真实 testnet 发布
完成后，Evidence/Manifest V2 才能同时绑定三观察器、`verify-source`、地址控制、终局
checkpoint 和批准清单。任何缺口都保持 `manifest_transition_allowed=false`。
