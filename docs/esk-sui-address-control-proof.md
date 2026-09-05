---
title: "ESK Sui 用户地址控制证明 V1 使用手册"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets, protocol]
---

# ESK Sui 用户地址控制证明 V1

本工具让一个 Sui testnet 地址签署固定个人消息，并在离线环境中验证三种 Sui
单签。成功只得到“该地址控制者签过这段消息”的候选证据，不会绑定一龙账户、
消费挑战、创建链上余额、发布 ESK、查询 RPC 或执行交易。正式范围见
[需求文档](requirements/esk-sui-address-control-proof-v1.md)。

这是新版唯一地址证明合同，不提供旧消息格式或旧证据兼容入口。后续服务端 V2
必须重新验证签名并在数据库事务内绑定当前登录用户和一次性挑战，不能直接信任
客户端提交的 V1 证据。

## 安装与验证

需要 Node.js 22 或更高版本。依赖只安装在本工具目录，`@mysten/sui` 精确固定为
2.29.0，锁文件只引用官方 npm registry 并保存完整 integrity。不要传入 npm 私密
token；安装过程不需要钱包、Sui 配置或 RPC。

```powershell
cd scripts/esk-sui-address-binding
npm.cmd ci --ignore-scripts --no-audit --no-fund
cd ../..
node scripts/test-esk-sui-address-binding.js
```

测试使用临时生成的三种单签密钥，只存在于测试进程内。运行时源码没有钱包、
签名器、交易构造器、子进程、环境变量或网络访问。

## 生成短时挑战

先由可信平台流程生成不可逆主体承诺；不要把用户 ID、手机号、邮箱、付款信息、
session token 或任何秘密写入请求。请求文件示例：

```json
{
  "schema": "yilong.esk.sui.address_binding_challenge_request.v1",
  "network": "testnet",
  "purpose": "user_asset_migration",
  "subject_commitment": "sha256:<64位小写十六进制摘要>",
  "address": "0x<64位十六进制Sui地址>",
  "ttl_seconds": 300
}
```

生成挑战：

```powershell
node scripts/prepare-esk-sui-address-binding.js challenge request.json > challenge.json
```

工具使用系统密码学随机源生成 32 字节 nonce，并只接受已经规范化的 64 位小写地址。
有效期只能为 120 至 900 秒。输出的 `message_base64` 是钱包必须按 Sui personal
message intent 原样签署的字节；不能签成交易，不能重新排版、改换行或补尾随换行。
challenge ID 是消息 UTF-8 字节 SHA-256 的前 16 字节小写十六进制并加 `eab1_`；
完整摘要写入 `message_sha256`。证据摘要移除根 `evidence_sha256` 后，对每层固定
ASCII 对象键升序、保留数组顺序、无空白 JSON 编码，再计算 UTF-8 SHA-256。
跨语言实现应先通过 `golden.test.js` 的固定向量。

## 验证钱包响应

钱包响应文件只包含版本、挑战 ID、钱包回传消息和 Sui serialized signature：

```json
{
  "schema": "yilong.esk.sui.address_binding_wallet_response.v1",
  "challenge_id": "eab1_<32位小写十六进制>",
  "message_base64": "<钱包实际签署并回传的Base64消息>",
  "signature": "<Sui serialized personal-message signature>"
}
```

验证命令：

```powershell
node scripts/prepare-esk-sui-address-binding.js verify challenge.json wallet-response.json > evidence.json
```

验证器重建完整挑战、消息摘要和 challenge ID，逐字节比较钱包回传消息，检查当前
系统时间仍在有效区间，然后调用官方 SDK 验证声明地址。只接受 ED25519、
Secp256k1、Secp256r1；MultiSig、zkLogin、Passkey 和未知方案在调用 SDK 前拒绝。

成功证据可用相同源码和锁文件再次复核，但必须同时看到这些边界：

| 字段 | 固定值 | 含义 |
| --- | --- | --- |
| `address_control_verified` | `true` | 该单签地址验证了这段个人消息 |
| `platform_subject_authenticated` | `false` | 未证明主体承诺来自当前登录用户 |
| `challenge_single_use_recorded` | `false` | 未在共享数据库中原子消费挑战 |
| `chain_finality_verified` | `false` | 未验证 Sui 委员会终局性 |
| `asset_identity_verified` | `false` | 未证明地址对应正式 ESK 发行 |
| `balance_eligible` | `false` | 不可展示或计入链上余额 |
| `manifest_transition_allowed` | `false` | 不得推动 Evidence/Manifest 状态 |

## 失败与恢复

标准输出只用于机器 JSON。失败时仅在标准错误输出
`ESK_SUI_ADDRESS_BINDING_ERROR=<固定错误码>`，不回显文件路径、输入、签名、
SDK 原始错误或环境内容。单文件上限 64 KiB，拒绝空文件、符号链接、BOM、未知字段、
非规范 Base64、超长值以及 UNC/设备命名空间路径。输入必须先复制到本地非共享、
非网络映射磁盘；操作系统映射盘无法由纯 Node 工具可靠辨认，不在离线保证内。
读取使用单一文件描述符、固定 64 KiB+1 缓冲并在读取前后核对文件身份与快照，
避免并发替换或增长先造成无界内存读取；父目录 junction 仍属于不可信目录边界。
JSON Schema 用于跨代理结构交换，已约束规范 Base64 padding、长度和 RFC 3339
`date-time`；日期真实存在、精确毫秒格式、TTL 关系和摘要一致性仍以运行时验证器
为权威，不能只跑 schema 就接受证据。
原始 JSON 在解析前拒绝同层或嵌套重复键，包括使用 Unicode escape 拼出的同名键，
避免不同语言的 first-wins/last-wins 规则产生协议歧义。

出现 `MESSAGE_MISMATCH`、`CHALLENGE_ID_MISMATCH`、`CHALLENGE_EXPIRED` 或
`CHALLENGE_NOT_YET_VALID` 时，应重新生成挑战并让钱包签署新消息；不要改证据绕过。
出现 `SIGNATURE_INVALID` 时核对地址、personal-message intent 和钱包响应。
`UNSUPPORTED_SIGNATURE_SCHEME` 只能通过使用 V1 支持的单签钱包解决，不得联网降级。
证据再次复核仍使用当前系统时钟，挑战过期后必定失败；文件中的 `verified_at` 只是
未认证的本地观察时间，不能作为新鲜度、认证或一次消费依据。

## 后续开发入口

下一项是 `esk-platform-sui-address-binding-v2`：由已认证会话签发挑战，服务端重新
验证签名，在 SQLite 事务中只消费一次，并以追加记录维护地址绑定/换绑历史。V2
成功后也只能把账户认证、一次消费、地址控制三项置为 true，仍不能自动启用链余额。
真实 testnet 发布、双源观察、源码对应性与终局 checkpoint 齐备后，再单独建立
Evidence/Manifest V2。先查 Feature Registry 并认领，禁止修改本 V1 形成隐式兼容。
