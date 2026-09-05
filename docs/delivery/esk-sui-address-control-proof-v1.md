---
title: "ESK Sui 用户地址控制证明 V1 交付证据"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets, protocol]
---

# ESK Sui 用户地址控制证明 V1 交付证据

本记录只说明离线候选证明的实现与验证，不代表平台用户已绑定地址、ESK 已发布、
链上余额已生成或任何资金已移动。范围见[正式需求](../requirements/esk-sui-address-control-proof-v1.md)，
操作见[使用手册](../esk-sui-address-control-proof.md)。

## 状态矩阵

| 能力 | implementation_status | verification_status | delivery_status | acceptance_status |
| --- | --- | --- | --- | --- |
| 版本化挑战与严格 JSON 合同 | implemented | integration_passed | 随本批 main 提交交付 | accepted：仅 testnet 候选挑战 |
| 三种 Sui 单签离线地址控制验证 | implemented | integration_passed | 随本批 main 提交交付 | accepted：不含平台认证/一次消费 |
| CLI 与依赖/无网络静态门禁 | implemented | integration_passed | 随本批 main 提交交付 | accepted：无钱包/RPC/交易执行 |
| 真实平台账户地址绑定 | not_implemented | not_run | 本批不发布服务 | deferred：由服务端 V2 完成 |
| 真实 testnet ESK 与用户余额 | not_implemented | not_run | 本批不发币 | deferred：需参数、钱包、授权和链证据 |

Feature Registry 的 `esk-sui-address-control-proof-v1` 绑定当前需求、合同、源码、测试和
文档证据，状态只推进到 `verified`；不登记为 `released`，也不把代码推送等同部署。

## 已执行验证

- 严格 TDD 首次运行因实现与 schema 尚不存在而失败；实现后定向 Node 回归
  58/58 通过。
- ED25519、Secp256k1、Secp256r1 均使用 `@mysten/sui@2.29.0` 真实
  `signPersonalMessage`/`verifyPersonalMessageSignature` 运行时正例。
- 覆盖错误地址、消息、challenge ID、签名、交易 intent、过期、未来、非规范
  Base64、TTL 120/900 边界、缺字段、未知字段、64 KiB 文件边界及输入脱敏。
- MultiSig、zkLogin、Passkey 与未知标志在 SDK 验证前失败关闭；正式验证入口固定
  使用系统时钟、系统密码学随机源和官方 SDK，不保留测试注入绕过。
- 证据完整性重算和漂移负例通过；账户认证、一次消费、资产身份、链终局、余额资格
  与 manifest 晋级六项始终为 false。复核使用当前系统时间，不信任证据内自报时间
  绕过过期门禁。
- CLI 的 challenge、真实钱包响应 verify、帮助、错误码、文件数量和超限路径通过；
  正向进程在 socket、DNS、HTTP(S)、TLS、fetch 与 WebSocket 失败关闭守卫下通过，
  UNC/设备命名空间在文件访问前拒绝，标准错误不泄漏秘密或路径。网络映射盘仍属于
  操作系统边界，手册要求输入位于本地非共享磁盘。
- 原始 JSON 重复键与 Unicode escape 同名键失败关闭；Evidence schema 只使用内部
  `$ref`，三种真实 serialized signature 均满足其精确 132 字符规范 Base64 合同。
- challenge ID/message SHA-256 与递归 canonical JSON/evidence SHA-256 各有固定
  golden vector，并由 Web Crypto 的独立 SHA-256 路径复核。
- lock 只引用官方 npm registry，全部包含 sha512 integrity；静态门禁未发现 RPC
  client、钱包、签名器、交易、私钥/助记词、环境变量、子进程或网络调用。
- 既有 publication 65/65、Currency 312/312、allocation 98/98 和共享传输回归通过；
  创世 schema/语义、供应守恒、Move 源码绑定及 13 项归属场景声明回归通过。本批没有
  重跑 Move Runtime，也没有执行真实公开网络 ESK 验收。

以上测试使用进程内临时密钥和本地文件，没有真实用户数据、生产钱包、RPC、签名发布、
广播、数据库、APK、服务器或资金操作。真实用户验收为 `not_performed`。

## 兼容与晋级边界

本批只维护 V1 新合同，不接受旧消息、旧证据或旧地址格式的兼容分支。保留历史账本
审计事实不等于继续执行旧运行时。V1 证据本身不具备授权效力；任何消费者都必须重新
验证证据完整性，并继续检查 `platform_subject_authenticated=false` 和
`challenge_single_use_recorded=false`。

后续 `esk-platform-sui-address-binding-v2` 必须绑定真实登录会话、部署实例和一次性挑战，
在同一 SQLite 事务里处理并发消费与地址唯一/换绑历史。真实链上状态仍需三观察器、
源码对应性和委员会 checkpoint 的独立 Evidence/Manifest V2，不能从本证据推导。

## 交付范围

本批为 `CodePushed`：只新增合同、离线 Node 工具、测试与文档，不修改服务器、数据库、
Android、PWA 或量化仓库，不部署服务、不生成 APK、不签名、不查询链、不移动资金。
确切 Git 提交由包含本文件的 main 历史及统一 finish 回执确定。
