---
title: "ESK Sui Currency 只读观察器使用手册"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, protocol]
---

# ESK Sui Currency 只读观察器

本工具读取两个公开测试网来源，核对包发布、规范 Currency 注册创建记录、
ESK 类型、6 位精度及精确固定供应。它不读取钱包，不签名，不发币，不写余额。
需求见[规范 Currency 观察器 V1](requirements/esk-sui-currency-observer-v1.md)。
它不会修改主项目现有 HTTP、PWA、APK 或独立量化合同。

## 安装与离线回归

需要 Node.js 22 或更高版本。依赖独立于 PC 前端；只使用 Sui SDK 的离线地址
派生函数。`@mysten/sui` 精确锁定 2.29.0，提交了全部依赖的 integrity；本目录
`.npmrc` 固定公开 npm registry 并禁用安装脚本；工具不需要私有 registry token，
不要为此传入私密 token。
依赖安装后的 `node_modules` 是可重新生成的本目录产物，精确忽略。

在仓库根目录执行（项目内长命令仍走统一有日志执行器）：

```powershell
cd scripts/esk-sui-currency-observer
npm.cmd ci --ignore-scripts --no-audit --no-fund
cd ../..
node scripts/test-esk-sui-currency-observer.js
node scripts/test-esk-sui-publication-observer.js
node scripts/test-esk-sui-observer-transport.js
node scripts/test-esk-sui-genesis-foundation.js
```

前三组测试使用合成 fixture、离线 SDK 及内存网络替身，不访问链；最后一项为
既有创世 schema/语义/源码绑定回归，不冒充 Move Runtime 重跑。

## 真实 ESK 公开参数

```powershell
node scripts/observe-esk-sui-currency.js --help
node scripts/observe-esk-sui-currency.js '<完整genesis摘要>' '<ESK包ID>' '<发布交易摘要>' '<注册交易摘要>' '<注册创建版本>' '<已批准供应基础单位整数>' '<第二公开GraphQL端点>'
```

占位符不可当成实际参数。程序不会猜地址、交易、版本、供应或第二读取源。
操作人先从实际发行/注册回执核对下列公开数据；不可使用测试 fixture 代替。

| 参数 | 要求 |
| --- | --- |
| 链标识 | 完整 32 字节 Base58 genesis digest；不是旧 JSON-RPC 的短十六进制 chain ID |
| 包与发布交易 | 包 ID、该包创建交易的 32 字节 Base58 digest |
| 注册交易与版本 | 规范 Currency 被创建时的交易 digest 与正 UInt53 版本；不是当前版本的上一笔交易 |
| 供应 | 已批准的正 u64 十进制字符串，以 6 位精度基础单位表示；无默认供应、浮点或科学计数法 |
| 第二来源 | 与官方主端点不同主机的公开 HTTPS 根路径或 `/graphql`；不含凭据、查询串、片段、私网地址 |

第一个读取源固定 `https://graphql.testnet.sui.io/graphql`。只接受测试网模式，
不提供钱包、签名、广播、主网、自定义 GraphQL 或 API token 参数。
外部公开链读取使用 HTTPS，不要求更改主项目 HTTP 或申请新证书。

## 如何核对

固定币种为预期包下 `::esk::ESK`。规范地址由官方 SDK 根据 Coin Registry
`0xc` 与 `CurrencyKey<T>` 离线推导。fieldless key 的 BCS 为一个零字节，不能
误用空数组，也不能预先重复包装 DerivedObjectKey。
算法依据[官方 SDK 说明](https://sdk.mystenlabs.com/sui/utils/derived_objects)。

一条固定查询同时读取：包与发布成功 checkpoint；指定注册版本的 Currency；
注册成功 checkpoint 与该交易对该对象的创建输出；当前规范元数据。
历史对象、创建输出、交易与当前状态必须自洽，两个来源的归一化证据必须完全一致。
不是任意找一个 symbol 为 ESK 的对象就通过。

注册在[固定框架源码](https://github.com/MystenLabs/sui/blob/46f18562f1f5af2438d35828e8b62d5e0b972db7/crates/sui-framework/packages/sui-framework/sources/registries/coin_registry.move)
中会创建规范派生 Currency，因此检查 `idCreated=true`、`idDeleted=false`、
`inputState=null`，并精确匹配输出地址、版本和 digest。
历史读取依据[精确版本接口](https://docs.sui.io/references/sui-api/sui-graphql/beta/reference/operations/queries/object)
与[ObjectChange](https://docs.sui.io/references/sui-api/sui-graphql/beta/reference/types/objects/object-change)。

当前版本可以高于注册版本，不能将当前 `previousTransaction` 误当初次注册。
两个版本必须都是 Shared、精确 Currency 类型、ESK、6 位、预期供应及 `FIXED`；
`BURN_ONLY` 和 null 不等同于固定供应，字段语义见
[CoinMetadata](https://docs.sui.io/references/sui-api/sui-graphql/beta/reference/types/objects/coin-metadata)。

## 输出与失败恢复

输出 schema 为 `yilong.esk.sui.currency_observation.v1`。只有两个来源都成功且
证据一致才为 `observed` / 退出码 0；其余 `unverified` / 退出码 1。
记录公开参数、查询时间、来源 URL 的 SHA-256、版本/digest 和有界错误码，
不回显完整端点或上游错误文本。供应始终输出十进制字符串。

无论观察结果如何，以下字段始终 false：`publication_certified`、
`asset_identity_verified`、`balance_eligible`、`manifest_transition_allowed`。
`observed` 只表示两份 RPC 报告满足本切片合同，不是委员会签名、源码匹配、
独立运营主体、分配、钱包归属、当前用户持币、收益或交易流动性的证明。

| 错误 | 处理 |
| --- | --- |
| `INVALID_INPUT` / `INVALID_ENDPOINT` | 检查公开参数格式，不传入秘密；错误输入不会发起查询 |
| `SDK_UNAVAILABLE` | 按锁文件安装依赖；不要改成自行猜测地址算法 |
| `CURRENCY_MISMATCH` / `REGISTRATION_MISMATCH` | 核对币种、规范地址和注册创建版本，不使用 pending/legacy 元数据 |
| `SUPPLY_MISMATCH` / `VERSION_MISMATCH` | 核对已批准供应及真实历史，不能为得到通过而改批准值 |
| `SOURCE_DISAGREEMENT` | 保存脱敏回执，检查索引进度后重跑；不能只采信成功的一端 |
| `TIMEOUT` / `NETWORK_ERROR` / `GRAPHQL_ERROR` | 检查公开端点能力/索引状态；不得加密钥或关闭传输保护绕过 |

读取有 12 秒总期限、4 秒 DNS 期限和 128 KiB 响应上限；全部 DNS 结果均检查，
连接绑定到已验证公网地址，拒绝跳转和压缩响应。无需后台服务或额外服务器。

## 可重复公开 schema 检查

```powershell
node scripts/esk-sui-currency-observer/tests/public-schema-smoke.js --run-public-non-esk-smoke
```

此命令明确联网，但只有一次固定官方测试网只读查询：使用公开 SUI Currency 与
一个不相关的非 ESK 包测试字段兼容性，校验离线派生地址并证明完整 ESK 验证拒绝
该混合样例。无参数不联网。公开样例检查通过也不是 ESK 实际发行、双源验收或
用户本人资产验收。测试网重置、历史裁剪或 Beta API 变化时可以失败，应重新取证。

## 下一位开发者

先查 Feature Registry 的 `esk-sui-currency-observer-v1` 与
`esk-sui-publication-observer-v1`，再读[交付记录](delivery/esk-sui-currency-observer-v1.md)。
真实 ESK 双源观察、源码验证、逐桶分配/供应守恒、能力交接、地址所有权及
终局性投影仍独立验收。签名发布须确认网络、钱包、正式参数与授权。
旧 manifest 不自动迁移；旧 Paper/正式平台登记不因本工具观察而增加链余额。
