---
title: "ESK Sui 六桶分配与团队锁仓 V1"
status: accepted
implementation_status: local_verified
owner: platform-assets, protocol
priority: p0
reviewed_at: 2026-09-05
decision_refs:
  - "docs/requirements/esk-sui-genesis-foundation-v1.md"
  - "docs/decisions/esk-sui-economic-foundation-v1.md"
---

# ESK Sui 六桶分配与团队锁仓 V1

## 用户结果

项目方可以把 ESK 货币核心产生的一枚总供应 Coin，在一笔原子交易中按显式参数
拆成六个用途桶；团队桶必须进入不可绕过、不可撤销的链上锁仓对象，其余五桶直接
交给清单指定的职责地址。执行结果留下不可变回执，供发布门禁和独立观察器复算。

这项能力只实现创世分配和团队锁仓，不代表 ESK 已发布，不移动真实用户资金，也不
建立量化份额、USDT 兑换、收益承诺、法定股权或客户理财关系。

## 已接受的边界

1. `esk_currency` 继续只创建一次总供应并永久关闭增发；本需求不修改其初始化器、
   供应常量、资产身份或测试。固定版编译器若拒绝既有依赖表语法，允许把同一 URL、
   subdir 和 40 位提交整理为等价的合法 TOML 单行，并重新生成本地验证摘要；这不是
   代币语义变化，也不授权重发链上包。
2. 新逻辑属于独立、后续可升级的 `yilong_participation` Move 包。它依赖 ESK 类型，
   但不得创建第二种 ESK、`TreasuryCap<ESK>`、mint 或 burn 能力。
3. 分配执行需要一次性 `GenesisAllocationCap`。成功后该能力被销毁，因此同一包版本
   只能产生一份创世分配回执。
4. 六个桶名固定为：
   `user_migration_and_ecosystem`、`team_vesting`、`project_treasury`、`liquidity`、
   `community_contributors`、`security_operations_reserve`。
5. 六桶金额由交易显式传入，必须都大于零，并以 `u128` 复算后精确等于输入 Coin
   数量；Move 源码不得硬编码正式总量、比例或日期。
6. 用户迁移桶和社区贡献桶共用 `distribution` 职责地址；项目金库桶和安全储备桶
   共用 `treasury` 职责地址；流动性桶使用 `liquidity` 地址；团队锁仓对象属于固定的
   `team_beneficiary` 地址。
7. 四个职责地址必须非零且两两不同。测试只使用合成地址，仓库 fixture 不写入真实
   钱包、私钥、多签成员或交易所凭据。
8. 团队锁仓采用“起始时间 + cliff + 结束时间”的参数化线性模型：cliff 前可领取量
   为零；cliff 起按从 start 到 end 的累计比例向下取整；end 起释放全部剩余量。
9. 分配时要求 `start_ms` 不早于 Sui `Clock` 当前时间，且
   `start_ms < cliff_ms < end_ms`。当前版本不替项目方选择真实日期。
10. 领取者必须同时是对象 owner 和对象内记录的固定 beneficiary；Coin 只能直接发送
    给该 beneficiary，调用方不能指定另一个收款地址。
11. 不提供撤销、追回、提前解锁、更换受益人、更改时间表、管理员代领、销毁未归属
    余额或任意资产恢复入口。
12. 新版直接切换：不新增旧原生 ESK、旧客户端桥接或双版本兼容层。需要保留的是
    资产账本与审计证据，而不是旧客户端实现。

## 本轮产物

### Move 包

- `genesis_allocation` 模块创建唯一能力、验证六桶守恒、拆分 Coin、创建团队锁仓并
  冻结创世回执。
- `team_vesting` 模块保存团队余额、固定 beneficiary 和时间表，并提供只读 getter、
  `claimable` 与 beneficiary-only `claim`。
- 分配回执记录六桶 base units、四个职责地址、团队时间表、32 字节清单摘要、执行
  时间和团队锁仓对象 ID。回执冻结后不可修改或转移。

### 策略合同

独立 JSON Schema 与 synthetic fixture 描述 Move 调用所需的六桶金额、职责引用、
线性锁仓参数和验证状态。fixture 中十亿 ESK 与 25/20/25/15/10/5 比例仅复用现有
本地结构样例，不构成主网经济决定。

策略合同不得填写真实 holder 地址、package ID、对象 ID、交易摘要或 checkpoint。
没有固定版本 Move build/test 证据时，状态只能是 `source_implemented`；本轮已经用
固定版本工具链完成本地构建与测试，因此 fixture 可以且只能提升到
`local_verified`，仍不得写成 `testnet_published` 或 `mainnet_published`。

## 核心不变量

1. 输入 Coin 在成功交易后不再存在，且没有余量返回调用者。
2. 五枚普通桶 Coin 与团队锁仓余额之和始终等于输入 Coin 金额。
3. 团队对象始终满足 `claimed + remaining == total`。
4. 同一毫秒重复领取不得产生第二笔零额 Coin；cliff 前领取必须失败。
5. end 时领取全部舍入余数，不遗留 dust。
6. 线性计算使用 `u128` 中间值，结果再安全转换为 `u64`。
7. 分配回执只能对应本包唯一能力的一次消费，并包含六桶完整复算材料。
8. 任何函数均不得取得或构造 ESK 的增发能力。

## 验收标准

1. 合成配置消费一枚 ESK Coin，严格生成六桶；逐桶数量、角色和接收方匹配，合计
   精确等于输入供应。
2. 团队桶进入参数化锁仓对象；提前、越权、重复零额和非法时间表失败；合法领取后
   `claimed + remaining == total`。
3. Move scenario 覆盖初始化、拆分、锁仓、cliff、中点、end 与对象余额复算；不产生
   `TreasuryCap`，不新增 mint/burn、交易、收益或 QSHARE 逻辑。
4. 固定 Sui `testnet-v1.79.0` 工具链 build/test 通过后，才允许把实现状态提升为
   `local_verified`；必须同时证明 `esk_currency` 已发布源码未改。
5. 独立静态测试校验包边界、危险 API 缺失、fixture/schema 一致性、源码绑定和
   本地运行证据状态的一致性，不得把本地验证冒充链上发布。
6. fixture 明示 synthetic/local-only，不生成链交易证据，不触发签名、广播、真实
   holder 分配或发布状态迁移。

## 明确不做

- 不修改或重新发布 `esk_currency` 的 Move 模块、供应参数或测试；只允许上述等价的
  依赖 manifest 语法修复。
- 不决定正式主网总量、比例、地址、锁仓日期或升级策略。
- 不实现 ESK/USDT 自动兑换、做市、托管、赎回或收益分配。
- 不实现客户量化本金、NAV、`QSHARE`、策略交易或币安 API。
- 不读取钱包配置，不创建私钥，不签名，不广播，不调用真实链上写交易。
- 不承诺固定价格、固定年化、保本、无限兜底或法定公司股份。

## 状态与发布门禁

本轮已经在固定版本 Sui CLI 下完成 build/test 并保存可复核摘要，因此最高状态为
`local_verified`。Node 静态测试只复核合同与源码绑定，不能替代 Move Runtime 结果。

即使 Move build/test 通过，也不代表可以发布测试网或主网。发布前仍需明确真实多签
地址、正式经济参数、升级策略、安全审计和用户迁移方案，并由独立观察器核对唯一
冻结回执、六桶对象及链上终局性。

当前货币核心已把可自由转移的总供应 Coin 交给发布者，因此参与包无法从类型系统上
强制发布者调用本分配入口。V1 通过发布门禁拒绝缺少唯一冻结回执的发布状态；如果
未来要做到发布者也绝对无法绕过，必须另立破坏性货币核心升级需求，不能在本需求中
静默改变已经接受的 ESK 初始化合同。

## 本地验证记录

2026-09-05 使用官方 `sui 1.79.0-46f18562f1f5`、固定提交
`46f18562f1f5af2438d35828e8b62d5e0b972db7` 的 Sui Framework 源码执行构建与测试，
并启用 `--warnings-are-errors`。参与包 13 项测试全部通过；货币核心 3 项回归测试全部
通过，生成的 `esk.mv` 摘要与原验证记录一致。

参与包构建摘要按 `production_bytecode_bundle_v1` 计算：按模块名排序，把
`模块名 + NUL + 原始 .mv 字节 + NUL` 依次输入 SHA-256；测试摘要是测试标准输出文件
原始字节的 SHA-256。依赖缓存中的 187 个 Framework/MoveStdlib 文件逐一与固定提交的
官方 Git blob SHA 对照，187/187 匹配，无额外文件。

这些事实只证明源码本地可编译且合成场景通过。没有读取或创建真实钱包，没有签名、
广播、资金移动或链上对象；所有发布证据继续为空。
