# ESK Sui 六桶分配观察器

## 当前能力

`scripts/observe-esk-sui-allocation.js` 是 ESK 六桶创世分配的只读 testnet 观察入口。
它使用两个不同公共 Sui GraphQL 主机复核同一份链上历史和指定 checkpoint 快照，输出
`yilong.esk.sui.allocation_observation.v1` JSON。

发布交易和分配交易都读取完整、未分页的 object changes；每个节点必须能完整分类为
Move object 或 Move package。工具会证明发布交易只创建一个指定类型的分配能力，
并用交易 Lamport version 约束分配输出版本，缺字段或不可分类节点不能被忽略。

这个工具不会读取钱包、创建交易、签名、广播或改变余额。`status=observed` 只表示
两个 RPC 来源报告了相同且符合输入预期的对象事实，不等于链上发布、源码认证、地址
控制证明或用户余额已经完成。

正式合同见
[ESK Sui 六桶分配只读观察器 V1](requirements/esk-sui-allocation-observer-v1.md)。

## 运行方式

```powershell
node scripts\observe-esk-sui-allocation.js <public-observation-input.json>
```

查看说明不会访问网络：

```powershell
node scripts\observe-esk-sui-allocation.js --help
```

输入文件上限 64 KiB，必须是普通 UTF-8 JSON 文件；符号链接、未知字段和非对象根值
都会失败关闭。文件只应包含公开链证据，不得放私钥、助记词、交易所 API key、签名
材料或带认证参数的 URL。

## 输入字段

根对象严格包含以下 20 个字段：

| 字段 | 含义 |
| --- | --- |
| `network` | 固定 `testnet` |
| `chain_identifier` | 完整 Base58 genesis digest |
| `currency_package_id` | 定义 `esk::ESK` 的货币包 |
| `participation_package_id` | 定义分配和锁仓对象的参与包 |
| `participation_publication_digest` | 参与包初始发布交易 |
| `allocation_digest` | 六桶分配交易 |
| `allocation_cap_object_id` | 一次性分配能力 ID |
| `allocation_receipt_object_id` | 冻结总回执 ID |
| `team_vesting_object_id` | 团队锁仓对象 ID |
| `initial_supply_coin_object_id` | 完整供应 Coin；分配后成为安全储备 Coin |
| `allocation_checkpoint_sequence` | 分配交易 checkpoint 序号字符串 |
| `allocation_checkpoint_digest` | 分配交易 checkpoint 摘要 |
| `observation_checkpoint_sequence` | 两来源共同读取当前锁仓的 checkpoint |
| `observation_checkpoint_digest` | 该观察 checkpoint 摘要 |
| `manifest_digest` | 非全零 `sha256:<64 lowercase hex>` |
| `expected_supply_base_units` | 固定供应 base units 十进制字符串 |
| `holders` | 五个已审核职责地址 |
| `buckets` | 六个固定桶的 base units |
| `team_vesting` | `start_ms`、`cliff_ms`、`end_ms` |
| `endpoints` | 官方 testnet GraphQL + 不同主机的第二公开 GraphQL |

`holders` 必须精确包含 `allocator`、`distribution`、`team_beneficiary`、`treasury`、
`liquidity_recipient`。后四个地址两两不同；allocator 可以与其中一个角色相同，但必须
同时是 allocation 交易 sender、cap 输入 owner 和完整供应 Coin 输入 owner。

`buckets` 必须精确包含：

- `user_migration_and_ecosystem`
- `team_vesting`
- `project_treasury`
- `liquidity`
- `community_contributors`
- `security_operations_reserve`

六个值都必须是正 `u64` 十进制字符串，合计必须等于 `expected_supply_base_units`。

## 观察结果怎么读

成功结果包含：

- 参与包版本、对象摘要、发布交易及 checkpoint；
- 发布与 allocation 的 effects digest、Lamport version；
- allocation sender、严格 UTC 时间和 checkpoint；
- cap 发布时创建、allocation 时销毁的版本与摘要；
- 冻结回执的版本、摘要、执行时间和 BCS SHA-256；
- 完整供应 Coin 的输入版本/摘要/数量；
- 六桶在 allocation 时的对象、数量、owner、变化类型和 BCS SHA-256；
- observation checkpoint 的时间，以及团队锁仓 total、claimed、remaining 与当前版本。

普通五桶可以在创世分配后合法拆分、合并和转移，所以工具只证明 allocation 历史输出，
不会把回执金额误报成当前地址余额。安全储备使用原始供应 Coin 的余量，因此输出
`change_kind=mutated`，不是第六枚 `created` Coin。

即使 `status=observed`，以下字段仍固定为 `false`：

- `publication_certified`
- `source_verified`
- `allocation_certified`
- `address_control_verified`
- `finality_certified`
- `asset_identity_verified`
- `balance_eligible`
- `manifest_transition_allowed`

平台资产页或迁移程序不得只凭本报告把余额标成“已上链”。

## 常见失败

| 错误码 | 处理方向 |
| --- | --- |
| `INVALID_INPUT` | 复核严格字段、地址、数量、时间与 checkpoint 顺序 |
| `INVALID_ENDPOINT` / `PRIVATE_ADDRESS` | 使用两个无认证参数的公共 HTTPS GraphQL 主机 |
| `PACKAGE_MISMATCH` | 复核参与包 ID 和初始发布交易 |
| `ALLOCATION_MISMATCH` | 复核 allocation 交易、sender、effects 和时间 |
| `OUTPUT_SET_MISMATCH` | 复核分页以及额外/缺失 ESK 目标对象 |
| `CAP_MISMATCH` | 复核一次性能力的发布创建和分配销毁 |
| `RECEIPT_MISMATCH` | 复核冻结回执 ID、owner、版本、digest 和内容 |
| `SUPPLY_MISMATCH` / `COIN_MISMATCH` | 复核完整供应输入及六桶历史输出 |
| `VESTING_MISMATCH` | 复核 beneficiary、时间表和 claimed/remaining 守恒 |
| `VERSION_MISMATCH` | 复核发布/分配 Lamport version 与对象版本单调关系 |
| `BCS_MISMATCH` | 对象内容不是规范 Base64 或不是当前 V1 固定布局 |
| `SOURCE_DISAGREEMENT` | 两个来源未对同一 checkpoint 返回完全相同证据 |

失败结果不会回显 endpoint URL、输入文件原文或底层网络错误。

## 验证

本地合成矩阵：

```powershell
node scripts\test-esk-sui-allocation-observer.js
```

显式的官方 testnet 非 ESK schema 探测：

```powershell
node scripts\esk-sui-allocation-observer\tests\public-schema-smoke.js --run-public-non-esk-smoke
```

第二条命令只证明固定 GraphQL query 被当前公开 schema 接受，并要求无关样本被完整
领域校验拒绝；拒绝可能先发生在 owner 等前置边界，不单独证明 ESK 类型门禁。它永远
不是实际 ESK 正向验收。

## 真实 ESK 验收

当前仓库没有真实 ESK package、allocation 交易、holder、对象和 checkpoint 参数，
所以不能运行真实正向观察。取得并审核这些公开参数后，再把它们写入独立的非秘密
输入文件运行 CLI，并把原始 JSON 报告作为候选证据交给后续 Evidence/Manifest V2
门禁复核。

旧 genesis manifest V1 的链标识、六份回执和 `vesting_policy_ref` 结构与新版冻结总
回执不一致。本工具不会读取、写回或推进旧 V1；产品只交付实际新版，后续直接建立
新版终局投影合同，不增加双轨兼容层。
