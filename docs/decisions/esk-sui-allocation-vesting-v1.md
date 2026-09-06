---
title: "ESK Sui 六桶分配与团队线性锁仓 V1"
status: accepted
decided_at: 2026-09-05
reviewed_at: 2026-09-06
owners:
  - project
  - platform-assets
  - protocol
requirement_ref: "docs/requirements/esk-sui-allocation-vesting-v1.md"
supersedes: []
---

# ESK Sui 六桶分配与团队线性锁仓 V1

## 决定

ESK 创世供应通过独立、可升级的 `yilong_participation` Move 包完成一次性六桶分配。
现有 `esk_currency` 仍只负责创建 ESK、一次预铸和永久固定供应，不改 Move 模块、
供应语义或测试，也不重发；固定版编译器拒绝原跨行 inline table 后，仅把相同 Sui
URL、subdir 和提交规范化为合法单行 TOML，并刷新本地构建证据。货币核心不会加入
归属、收益、量化或赎回规则。

分配调用消费完整的总供应 Coin 和唯一 `GenesisAllocationCap`。五个普通用途桶直接
发送到显式职责地址，团队桶进入固定受益人拥有的锁仓对象；成功后唯一能力被消费，
并冻结一份包含完整复算材料的创世分配回执。该流程没有默认真实地址、金额或日期。

本决定采用参数化线性锁仓，但不批准任何真实商业参数。仓库中的策略 fixture 仅是
synthetic/local-only 结构样例，不能作为销售、团队报酬、测试网发布或主网发行条款。

## 包和模块边界

`yilong_participation` 本批只增加两个职责明确的模块：

- `genesis_allocation`：创建并消费一次性能力，校验输入，拆分六桶，转移普通 Coin，
  创建团队锁仓并冻结分配回执。
- `team_vesting`：保存 ESK 余额、固定 beneficiary、起始/cliff/结束时间和已领取量，
  提供只读投影、`claimable` 计算与 beneficiary-only 领取。

参与包不得创建第二种 ESK，不得取得或构造 `TreasuryCap<ESK>`，不得暴露 mint、burn、
撤销、追回、提前解锁、更换受益人、管理员代领或任意恢复入口。它也不实现服务支付、
ESK/USDT 兑换、交易、NAV、QSHARE、客户本金、收益计算或法定股权。

## 一次性分配

六个桶及其职责固定为：

| 桶 | 职责 |
| --- | --- |
| `user_migration_and_ecosystem` | `distribution` |
| `team_vesting` | `team_vesting`；对象固定发送给 `team_beneficiary` |
| `project_treasury` | `treasury` |
| `liquidity` | `liquidity` |
| `community_contributors` | `distribution` |
| `security_operations_reserve` | `treasury` |

四个职责地址必须非零且两两不同。六个金额必须分别大于零；用 `u128` 求和后必须
精确等于输入 Coin 的 `u64` 数量。成功交易消费输入 Coin，不向调用者返还余量。
除团队桶外产生五枚 Coin 并直接发送；团队桶的全部余额进入锁仓对象。

冻结回执记录六个桶的 base units、四个职责地址、团队时间表、32 字节创世清单摘要、
执行时间和团队锁仓对象 ID。回执是公开复算材料，不证明地址法律归属、审批完成或
链下商业事实；发布门禁还必须独立核对交易、对象、checkpoint 和角色证明。

## 线性归属公式

设团队锁仓初始总量为 `T`，开始、cliff、结束毫秒为 `S`、`C`、`E`，观察时间为 `t`。
创建要求 `Clock.now >= transaction start` 的边界具体实现为 `S >= Clock.timestamp_ms()`，
且严格满足 `S < C < E`。累计已归属量 `V(t)` 为：

```text
t < C:       0
C <= t < E: floor(T * (t - S) / (E - S))
t >= E:      T
```

乘法和除法使用 `u128` 中间值，再安全转换为 `u64`。本次可领量为
`V(t) - claimed`。cliff 前、非 beneficiary、零可领或已全部领取均失败；不能产生
零额 Coin。到 `E` 后领取全部剩余量，吸收整数向下取整产生的 dust。

领取者必须既是锁仓对象 owner，又与对象内固定 beneficiary 相同。领取 Coin 只能
直接发送给该 beneficiary，接口不能接受替代收款地址。始终保持：

```text
claimed + remaining = total
```

## 策略合同与状态

`esk-allocation-policy-v1.schema.json` 使用严格对象形状、十进制字符串数量和精确六桶
集合。fixture 固定四个明显的 synthetic holder reference、合成毫秒时间和现有
25/20/25/15/10/5 结构样例，同时明确：

fixture 的 `manifest_digest` 只是检验 32 字节 Move ABI 与冻结回执绑定形状的明显合成
值，不是任何真实发布清单的摘要，也不能作为清单内容已核验的证据。

- `state=local_verified`；
- 固定版本 Move build/test 均为 `passed`，并绑定包输入、生产字节码与测试输出摘要；
- package、对象、交易和 checkpoint 均不存在；
- 没有真实 holder、经济参数批准、签名、广播、资金移动或发布状态推进。

Schema 能约束数据形状，但总量守恒、职责映射、地址互异、时间顺序和状态一致性仍由
独立语义测试复核。Node 静态测试不能冒充 Move Runtime、对象执行或链上终局性证据；
`local_verified` 也不能冒充 `testnet_published`。

## 升级与不可绕过边界

参与包的正式升级策略仍为 `pending`。在确定 UpgradeCap 托管、升级范围和审计流程前，
“当前源码无后门”不等于未来升级不可加入后门。测试网或主网发布前必须批准策略，
并由与分配执行者不同的验证方复核实际发布源码和能力持有人。

货币核心当前把可自由转移的总供应 Coin 交给发布者。参与包无法通过 Sui 类型系统
强制发布者调用本入口，也无法阻止发布者绕开它直接转移该 Coin。V1 的不可绕过性由
发布门禁实现：缺少唯一冻结回执、六桶对象复算或团队 Move 锁仓证据时，清单不得进入
published。若未来要求货币初始化器在类型层直接绑定分配包，必须新立破坏性货币核心
升级需求，不能在此功能中静默改变既有 ESK 身份。

## 新版与兼容策略

本功能只有 V1 新协议，不为旧原生 ESK 17/21 字段桥、旧量化入口或 Paper 跨 APK
展示增加兼容层。历史 Paper 和正式平台账本、审核记录及既有链下资产页面继续保留；
它们不会因 synthetic fixture、Move build/test 或分配回执自动转成链上余额。

## 本轮本地验证证据

2026-09-05 使用官方 `sui 1.79.0-46f18562f1f5` 和固定提交
`46f18562f1f5af2438d35828e8b62d5e0b972db7` 的依赖源码，以
`--warnings-are-errors` 构建并测试。参与包 13/13 通过，货币核心回归 3/3 通过；但当时
在 `move test` 之后取样构建目录，不能据此证明生产字节码没有变化。

参与包 `move_build.evidence_digest` 使用 `production_bytecode_bundle_v1`：只包含
`genesis_allocation.mv` 与 `team_vesting.mv`，按模块名排序，对每个写入
`模块名 + NUL + 原始字节 + NUL` 后计算 SHA-256。`move_test.evidence_kind` 为
`canonical_test_receipt_sha256_v1`：CI 只从运行输出提取白名单测试行、规范化 ANSI 与
CRLF，并精确忽略 Sui 1.79 对显式依赖的一条固定提示，再与受管 evidence 字节及摘要
对比。任何其他输出都失败关闭。工具链合同与 synthetic fixture 共同绑定工具链二进制、
官方源码归档、包输入、唯一归档根、187 个允许文件及其规范内容集合摘要；不使用 live
`.git` 依赖缓存。

2026-09-06 的可复现 CI 复核发现，原 `dded0663...` 在 `move test` 覆盖构建目录后
取样，虽只挑出两个生产模块名，模块本身仍是 test-mode 字节码，因此不能证明将要
发布的生产包。CI 现于 `move build` 完成后、运行测试前冻结精确两个生产模块，纠正后
`production_bytecode_bundle_v1` 为
`fa691e2e7d7c1c347b8fd88a2dc9f3ca2590ee56813c0bb313ef2ea8d477d3ef`。这不改变源码、
13 项测试或链状态；后续发布只能引用纠正后的摘要。

验证使用空 keystore、`envs: []` 的显式 client 配置和独立 `MOVE_HOME`；固定
`--build-env testnet` 只选择构建环境，不创建 Sui RPC client，也不向任何链端点发起
请求。冷缓存只允许从 GitHub 获取上述固定源码归档。未读取真实钱包、未签名、未广播、未
生成 package/object/transaction/checkpoint 证据。

## 正式参数门禁

真实发布前仍须单独批准：总供应和六桶金额/比例、四个职责地址及所有权证明、团队
beneficiary、`S/C/E` 日期、参与包升级策略、多签与恢复方案、安全审计及迁移安排。
任何一项未批准时，本地样例不得被复制为生产参数。

## 后果

好处是六桶守恒和团队长期锁仓从文字政策变成可执行、可复算的对象事实，同时保持
货币核心最小。代价是线性舍入、对象托管和 UpgradeCap 都必须被持续审计，而且发布
门禁而非类型系统承担“发布者确实走了分配入口”的最终保证。

## 复审触发器

以下变化必须新建 ADR：修改六桶集合或职责映射、采用非线性归属、允许撤销/追回/
提前释放/更换 beneficiary、加入管理员领取或恢复能力、改变 ESK 供应与精度、将
QSHARE/NAV/客户本金或法定股权并入本包，以及把发布门禁改为货币核心强制调用。
