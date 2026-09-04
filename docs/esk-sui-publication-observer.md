---
title: "ESK Sui 发布只读观察器使用手册"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, protocol]
---

# ESK Sui 发布只读观察器使用手册

## 适用范围

这是测试网公开交易的观察工具，不是钱包、发币脚本或余额接口。
它读取给定包对象、创建交易及成功交易的 checkpoint，在两个来源一致时输出
`observed`；不认证 ESK 源码/身份、固定供应、资产分配或用户持有量。
查询不存在、索引尚未追上、数据被剪枝或端点失败时均是 `unverified`，
不能据此断言交易从未发布。

目标路线图见 [首批用户交付阶段目标](requirements/esk-first-user-delivery-roadmap-v1.md)，
验收合同见 [观察器 V1](requirements/esk-sui-publication-observer-v1.md)。

## 准备与运行

需要 Node.js 18+ 和能够直连公共 DNS/HTTPS 的开发机，不需要 Sui CLI、钱包、
助记词、API key 或主项目服务器证书。工具的 HTTPS 用于验证外部 Sui 服务身份，
不改变主项目的部署技术栈。

从已审核的测试网发布记录取得以下公开参数：

1. 完整 Base58 genesis checkpoint digest，即 GraphQL 的 `chainIdentifier`；
   不能使用旧 JSON-RPC 的 8 位十六进制短值，也不自动从当前端点猜测预期链。
2. 要核对的精确 package ID 和对应发布交易 digest。
3. 第二个经人工审阅、无认证的公开 GraphQL URL。仅支持 HTTPS 443，路径 `/`
   或 `/graphql`；拒绝凭据、查询串、片段、路径 token、重定向和私网 DNS 结果。
   不支持收费服务的密钥 URL；不要把密钥放进命令行。

在仓库根运行（尖括号内容须换成真实公开参数）：

```text
node scripts/observe-esk-sui-publication.js <完整链标识> <包ID> <发布交易摘要> <第二公共GraphQL地址>
```

首个端点固定为 `https://graphql.testnet.sui.io/graphql`，第二端点必须不同主机名。
这只排除明显重复；同供应商别名或同后端仍可能相关，运营独立性需要人工审阅。
DNS 地址核验后直接用于本次 TLS 连接，显式保持证书检查，禁止重定向；
DNS 查询最多等待 4 秒并取消，单个请求整体最多等待 12 秒，响应上限 128 KiB。
不自动重试、切链、替换参数或转入签名流程。

退出码 `0` 表示两个 RPC 返回的指定发布事实一致；`1` 表示未核实或输入不合法。
结果写到标准输出，不自动持久化或改写清单。操作人按正常证据保管流程保存
公开输入和输出；如索引延迟，稍后对相同输入重新运行，不覆盖旧观察记录。

## 结果合同

`schema = yilong.esk.sui.publication_observation.v1`，包含：

- `expected`：已校验的预期测试网、链标识、包 ID、发布交易。
- `sources`：两端结果及各端点规范化 URL 的 UTF-8 SHA-256；不回显端点 URL。
  操作人保留公开输入以便以后重新计算端点指纹。
- `evidence`：仅当两端完全一致时，包含包对象摘要/版本、发布交易、checkpoint
  序号和摘要；序号与版本先按 GraphQL UInt53 校验，再以十进制字符串输出。
- `error_code`：有界错误代码，不保存服务端错误正文、响应中的其他字段或凭据。
- `trust_basis`：RPC 报告，不是本工具已校验委员会签名的密码学证明。

无论是否 `observed`，以下字段始终为 `false`：

```text
publication_certified
asset_identity_verified
balance_eligible
manifest_transition_allowed
```

错误大类：`INVALID_INPUT/INVALID_ENDPOINT` 要修正公开参数；
`NETWORK_ERROR/TIMEOUT/HTTP_ERROR` 检查网络或来源；`GRAPHQL_ERROR/INVALID_RESPONSE`
应核对上游接口而非降级接受部分结果；`CHAIN_MISMATCH/PACKAGE_MISMATCH/
TRANSACTION_MISMATCH` 要核对指定记录；`CHECKPOINT_MISSING` 表示确认尚不能核实；
`TRANSACTION_NOT_SUCCESSFUL` 不能当作发布成功；`SOURCE_DISAGREEMENT` 要调查分歧。
`PRIVATE_ADDRESS/RESPONSE_TOO_LARGE` 不可通过关闭安全检查解决。

## 已验证与未验证

本批次离线测试入口：

```text
node scripts/test-esk-sui-publication-observer.js
node scripts/test-esk-sui-genesis-foundation.js
node scripts/test-esk-asset-contract.js
node scripts/test-esk-profile-asset-visibility.js
```

2026-09-04 使用本工具的实际传输和校验模块完成官方测试网单源 smoke。
公开样例包 `0x8f9df445446cb4568136e6a0f6ef69c36d15ce869fca1185660bcd16a616a0e3`、
交易 `52uc677bkdkD858wn6gtYkpmHWf8NQQE8nbHVjbL7Zdn` 被观测为成功，
checkpoint `379597347`。这不是 ESK，不是双源线上验收，更不是发币记录。
未确认可用的独立第二端点，不配置假地址或用同服务别名凑成功。

| 能力 | 实现 | 验证 | 部署/验收边界 |
| --- | --- | --- | --- |
| 输入→双源查询→一致性回执 CLI | implemented | offline_passed | 工具代码交付；无生产部署 |
| 当前官方 GraphQL 适配 | implemented | environment_passed | 公开非 ESK 单端点 smoke |
| 实际 ESK 双源观察 | implemented | deferred | 待真实公开发布参数及审阅后的第二端点 |
| 完整发行认证与用户链余额 | not_started（本切片） | not_run | 独立后续功能，不由此回执解锁 |

下一步需逐项补齐：源码 verify-source、Currency Registry 注册及 fixed supply、
六桶余额复算与真实执行回执、团队 Move 归属、权限交接、清单前序绑定、
地址所有权与用户 claim、反向结转和终局性索引。旧创世验证器继续拒绝
`testnet_published` 和全部主网清单；旧 APK/Paper API 不消费本观察结果。

## 上游依据

- [chainIdentifier](https://docs.sui.io/references/sui-api/sui-graphql/beta/reference/operations/queries/chain-identifier)：完整 Base58 genesis digest。
- [package 查询语义](https://docs.sui.io/references/sui-api/sui-graphql/beta/reference/operations/queries/package)：顶层 package 有升级版本选择语义，本工具使用精确 object + asMovePackage。
- [transaction](https://docs.sui.io/references/sui-api/sui-graphql/beta/reference/operations/queries/transaction)：查询缺失也可能是索引保留范围限制。
