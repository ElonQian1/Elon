---
title: "ESK Sui 六桶分配只读观察器 V1"
status: accepted
implementation_status: verified
owner: platform-assets, protocol
priority: p0
reviewed_at: 2026-09-05
decision_refs:
  - "docs/requirements/esk-sui-publication-observer-v1.md"
  - "docs/requirements/esk-sui-currency-observer-v1.md"
  - "docs/requirements/esk-sui-allocation-vesting-v1.md"
---

# ESK Sui 六桶分配只读观察器 V1

## 用户结果

项目方和独立复核者可以只凭公开 Sui testnet GraphQL 数据，核对一笔 ESK 创世分配
交易是否按已批准参数消费一次性能力和完整供应 Coin，生成唯一冻结回执、四枚新建
用途 Coin、一个变更后的安全储备 Coin 与一个团队锁仓对象，并在指定 checkpoint
复核团队锁仓仍满足守恒。

观察成功只表示两个不同公共 GraphQL 主机报告了相同的链上对象事实。它不签名、不
广播、不移动资金，也不证明地址私钥控制、源码匹配、委员会签名终局性、用户余额、
经济参数批准或发布清单可以自动晋级。

## 依赖与边界

1. 复用 `esk-sui-publication-observer` 的 HTTPS、DNS 公网地址校验、TLS、响应上限、
   超时和 GraphQL 错误处理；不新增钱包、Sui client、JSON-RPC 或交易构造器。
2. 第一来源固定为官方 testnet GraphQL，第二来源必须是不同公共主机；任何一源失败、
   返回不完整、分页未结束或两源归一化证据不同，整体都保持 `unverified`。
3. 参与包发布事实复用现有发布观察校验；货币注册、固定供应和源码 `verify-source`
   仍由独立观察器/门禁负责，本观察器不复制或冒充这些结论。
4. 输入同时绑定 currency package 与 participation package，绝不把两个包压成一个
   `package_id`。参与包 V1 只接受初始发布版本；升级包需要新的观察协议。
5. 只读取固定 GraphQL query。Move 内容使用 `contents.bcs` 的严格 Base64 与固定 V1
   BCS 布局解码，不依赖跨实现可能变化的 JSON 展示形状。
6. 本功能只有新版 V1 观察合同，不读取或回填旧原生 ESK 17/21 字段、Paper 跨 APK
   桥、旧 `vesting_policy_ref` 或旧 genesis manifest 的 published 状态。

## 严格输入合同

输入是公开、已复核的预期事实，不得含私钥、助记词、API token 或交易所凭据。根对象
和所有嵌套对象都拒绝未知字段，至少包含：

- `network=testnet`、完整 Base58 `chain_identifier` 和两个公共 GraphQL endpoint；
- currency package ID；participation package ID、发布交易摘要；
- allocation 交易摘要、冻结回执 ID、团队锁仓 ID、一次性 allocation cap ID、初始
  总供应 Coin ID；
- allocation checkpoint 序号与摘要，以及不早于它的 observation checkpoint 序号与
  摘要；
- 非全零 `sha256:<64 lowercase hex>` 已批准清单摘要；
- 固定供应 base units 与六个正数桶金额，全部使用无前导零十进制字符串；
- `distribution`、`team_beneficiary`、`treasury`、`liquidity_recipient` 四个非零且
  两两不同的 Sui 地址，以及负责持有 cap 和供应 Coin 的 `allocator` 地址；
- 团队锁仓 `start_ms`、`cliff_ms`、`end_ms`。

六桶名称和职责映射固定为：

| 桶 | 历史分配输出 | 预期 owner |
| --- | --- | --- |
| `user_migration_and_ecosystem` | 新建 `Coin<ESK>` | `distribution` |
| `team_vesting` | 新建 `TeamVesting` | `team_beneficiary` |
| `project_treasury` | 新建 `Coin<ESK>` | `treasury` |
| `liquidity` | 新建 `Coin<ESK>` | `liquidity_recipient` |
| `community_contributors` | 新建 `Coin<ESK>` | `distribution` |
| `security_operations_reserve` | 原始供应 Coin 的输出态 | `treasury` |

六桶金额必须都在 `u64` 范围且用 `BigInt` 求和后精确等于固定供应。时间必须满足
`start_ms < cliff_ms < end_ms`。合成策略中的全零清单摘要和 `synthetic:sui:*` holder
不能作为真实观察输入。

## 固定查询与历史证据

每个来源在一次固定查询中读取：

1. chain identifier；
2. participation package，以及发布交易的 effects digest、Lamport version、成功
   checkpoint 与完整 `objectChanges(first: 50)`；
3. allocation 交易的 sender、effects digest、Lamport version、成功状态、时间、checkpoint 与完整
   `objectChanges(first: 50)`；`hasNextPage=true` 或 `hasPreviousPage=true` 均失败；
4. 指定 observation checkpoint 的时间及该时点的冻结回执和团队锁仓对象。

两笔交易的每个 object change 都必须具有完整 flags、地址和至少一个可识别状态；每个
非空状态必须能明确分类为 Move object 或 Move package，并具有版本、digest、previous
transaction 与对应内容。未知、空缺或不可分类节点整体失败，不能借“无关对象”忽略。

allocation checkpoint 必须与交易 effects 完全一致；observation checkpoint 必须与
输入摘要一致且序号不小于 allocation checkpoint。checkpoint 被称为“RPC 观察到”，
不能称为本工具已经独立验证验证者委员会签名。

## 单次分配不变量

1. allocation 交易成功，effects digest、checkpoint digest 和全部对象 digest 都是
   合法 32 字节 Base58 摘要。
2. `GenesisAllocationCap` 的 ID 与 BCS 内 UID 一致；它在 participation package 发布
   交易中唯一创建，在 allocation 输入态由 `allocator` 持有，并在同一交易中
   `idDeleted=true`、`outputState=null`。创建版本必须等于发布交易 Lamport version；
   消费输入版本不得早于创建版本且必须小于 allocation Lamport version，同版本时
   digest、previous transaction、BCS 和 owner 必须仍等于创建态。
3. 初始供应对象是精确的 `0x2::coin::Coin<currency::esk::ESK>`，BCS 余额等于完整
   固定供应，allocation 输入态 owner 为 `allocator`。
4. 冻结 `GenesisAllocationReceipt` 在 allocation 中创建，类型精确属于 participation
   package，`owner=Immutable`、`hasPublicTransfer=false`、previous transaction 等于
   allocation 摘要；指定 observation checkpoint 的内容、版本和 digest 必须仍与
   创建态相同。
5. 回执 BCS 的 UID、32 字节清单摘要、总量、四个职责地址、六桶金额、团队时间表、
   执行时间及六个结果对象 ID 全部与输入和交易变化互相复核。
6. 回执记录的六个结果 ID 必须唯一。用户迁移、项目金库、流动性和社区贡献四枚
   Coin 是新建输出；团队结果是新建锁仓对象。
7. 安全储备不是新 Coin：其对象 ID 必须等于初始供应 Coin ID，变化必须同时具有
   输入与输出、不能标为创建或删除；输出 BCS 余额为安全储备金额且 owner 为 treasury。
   输入版本必须小于输出版本，输出版本等于 allocation Lamport version；同一交易
   新建的回执、团队锁仓和四枚用途 Coin 也必须等于该 Lamport version。
8. 五个普通 Coin/安全储备只核验 allocation 交易的历史输出，不要求在以后仍存在、
   仍保持原余额或仍由原地址持有。当前持仓和用户余额必须由另一项分页余额投影实现。
9. allocation 交易中若出现额外 ESK Coin、额外 Receipt、额外 TeamVesting 或额外
   GenesisAllocationCap 变化则失败；gas 等无关类型变化允许存在但不进入 ESK 证据。

## 团队锁仓不变量

1. 创建态类型精确为 participation package 的 `TeamVesting`，owner 与内部
   beneficiary 都等于 `team_beneficiary`，且 `hasPublicTransfer=false`。
2. 创建态 `claimed=0`、`remaining=total=team_vesting` 桶金额，时间表与回执和输入
   相同。
3. observation checkpoint 的当前态必须仍由同一 beneficiary 持有；UID、beneficiary、
   total 和时间表不得变化，版本不得早于创建态。
4. 当前态允许已经领取，但必须满足 `0 <= claimed <= total` 且
   `claimed + remaining == total`。
5. 回执 `executed_at_ms <= start_ms`；交易与 observation checkpoint 时间必须是严格
   有效的 UTC RFC3339 日历时间，且都不早于回执执行时间；observation 时间不得早于
   allocation 交易时间。同一 checkpoint 的两个时间必须精确落在同一毫秒。观察器
   不根据本机时间推算应领取量。

## BCS 解码边界

解码器只支持当前 V1 的四种已知布局并拒绝尾随字节：

- `GenesisAllocationReceipt`：UID、32 字节向量、总量、四地址、六金额、四时间、六 ID；
- `TeamVesting`：UID、beneficiary、total、claimed、三时间和 remaining Balance；
- `Coin<ESK>`：UID 与 Balance；
- `GenesisAllocationCap`：UID。

所有 address/ID 都规范化成 64 位小写十六进制，所有 `u64` 保持十进制字符串，避免
JavaScript number 精度损失。证据只输出 BCS SHA-256，不回显原始 BCS。

## 输出合同

输出 schema 为 `yilong.esk.sui.allocation_observation.v1`，状态只有 `observed` 或
`unverified`。成功时保存两来源一致的规范化证据；失败只返回固定错误码，不回显底层
网络错误、输入文件原文或 endpoint URL。

无论成功与否，下列字段都固定为 `false`：

- `publication_certified`、`source_verified`、`allocation_certified`；
- `address_control_verified`、`finality_certified`、`asset_identity_verified`；
- `balance_eligible`、`manifest_transition_allowed`。

`observed` 只允许解释为“两个 RPC 来源报告相同且符合预期”。它不证明真实 holder
控制地址、不创建平台余额、不自动把 `local_verified` 改成 `testnet_published`。

## 验收标准

1. 合成完整正例覆盖参与包发布、cap 发布时创建/分配时销毁、完整供应输入、冻结
   回执、四个新 Coin、一个变更安全储备 Coin、团队锁仓创建态和当前态。
2. 逐字段负例覆盖链、包、交易、checkpoint、分页、类型、owner、previous tx、版本、
   digest、BCS/Base64、清单、金额、对象 ID、cap、供应、锁仓和额外目标对象变化。
3. 双源任一失败或任一规范化证据差异都失败关闭；错误不泄露 endpoint 或底层消息。
4. 固定 query 不含 mutation、simulate 或 execute，所有变量来自严格规范化输入，并
   继续使用既有安全 transport。
5. CLI 坏参数在零网络请求下退出 1；`--help` 明确只读边界；成功/失败输出可机器读。
6. 官方 testnet 只运行无关公开样本的 schema smoke，证明固定 query 仍被当前 GraphQL
   schema 接受并确认该样本被完整领域校验拒绝；拒绝原因可以先落在 owner 等前置边界，
   不得把它写成 ESK 类型证明或真实 ESK 验收。
7. 回归现有 publication、currency、六桶静态验证和固定版本 Move 测试。

## 明确不做

- 不发布 testnet/mainnet，不读取钱包，不签名、不广播、不移动 ESK、SUI 或 USDT。
- 不实现地址签名挑战、多签成员证明、UpgradeCap 托管或源码 `verify-source`。
- 不显示或迁移用户链上余额，不做历史付款入账、兑换、赎回、量化交易或收益分配。
- 不改旧 genesis manifest V1 来伪装兼容；新的发布终局投影另立 V2 需求。
- 不承诺固定价格、固定收益、保本、法定股权或即时兑付。

## 真实正向验收前置项

真实 ESK 观察必须由项目方另行提供并审核：testnet 网络、两个公共 GraphQL 来源、两个
package、参与包发布交易、allocation/cap/supply/receipt/vesting 对象、两个 checkpoint、
非合成清单摘要、五个角色地址、六桶金额与锁仓日期。缺少任何一项时只能交付代码与
合成/非 ESK schema 验证，不能声称真实 ESK 已观察成功。
