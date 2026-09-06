---
title: "ESK Sui 结算层与固定供应货币核心 V1"
status: accepted
decided_at: 2026-09-04
reviewed_at: 2026-09-06
owners:
  - project
  - platform-assets
  - protocol
requirement_ref: "docs/requirements/esk-sui-genesis-foundation-v1.md"
supersedes: []
---

# ESK Sui 结算层与固定供应货币核心 V1

## 决定

ESK 第一条公链落地路线采用 Sui Currency Standard。Sui 只承担公开资产、低频结算
对象和可验证回执，不承担币安量化策略、撮合、行情计算、交易所账户托管或实时 NAV。
业务系统始终通过版本化适配器连接结算层，因此未来研究自有公链不要求重写主项目
的身份、订单、量化和会计核心。

ESK 的货币核心使用一次预铸、固定总供应且不可继续增发的设计。算力奖励、早期用户、
团队归属、流动性和生态激励都从创世分配桶释放，而不是让运营服务持有可无限增发的
`TreasuryCap`。V1 以六位精度和十亿枚示例总量建立可编译基础；主网经济参数必须由
后续版本清单和批准记录确认，不能把本轮 fixture 当成已经生效的销售条款。

## 两包边界

链上代码按变化速度拆成两个包：

1. `esk_currency` 是极小且尽量不可变的货币核心，只创建 ESK、一次铸造、固定供应和
   元数据能力。
2. `yilong_participation` 是后续可升级协议，只管理 ESK 锁仓参与、团队可分配利润
   周期、领取回执和 ESK 参与协议暂停状态。

本轮只交付第一个包及第二个包的边界，不提前实现利润协议。这样 ESK 的资产身份不会
因为收益规则、法域准入或量化产品更新而反复升级。

## 现有 ESK 决定保持有效

本决定实现而不改写 `esk-consumable-economic-participation-v1`：

- ESK 仍是同一个服务支付、治理和团队批准利润快照参与资产；
- 用户消费或转让 ESK 后，余额减少，之后快照权重也减少；
- ESK 由市场定价，不是永久 USDT 锚定、固定净值或无条件回购承诺；
- 不另发一个“团队权益币”来偷偷替代 ESK；
- `QSHARE`、客户本金、公司法股权和服务订单继续是独立事实。

链上 Coin 只证明某地址控制特定数量的 ESK。它不会单独证明用户身份、KYC 结论、
服务交付、公司股东身份、某次利润资格或量化基金份额。

`yilong_participation` 不得创建或复制 QSHARE、NAV、申赎、Senior/Junior 资本栈或其
链投影；这些能力继续服从量化 V21 合同。

## 为什么先用 Sui

当前 Sui 对象模型适合表达能力、仓位、周期和不可变回执；Currency Registry 能让
钱包和索引器以统一方式发现资产。项目无需先开发共识、节点网络、浏览器和钱包，便可
先验证公开发行、权限交接、索引回放和 gas sponsor。

选择 Sui 不是永久排他绑定。主项目只保存标准化的 `chain asset reference`、发布证据
和 finality receipt；量化系统仍以自身对账/NAV 为真源。未来迁移到自有链时，迁移事件
必须逐条关联旧 Sui type tag、最终 checkpoint 和新链领取回执。

## Currency 创建流程

V1 固定到 Sui `testnet-v1.79.0` 源码提交
`46f18562f1f5af2438d35828e8b62d5e0b972db7`，避免依赖浮动分支。货币包使用：

```text
new_currency_with_otw
  -> TreasuryCap.mint(total_supply)
  -> CurrencyInitializer.make_supply_fixed(TreasuryCap)
  -> CurrencyInitializer.finalize
  -> finalize_registration on CoinRegistry 0xc
```

初始化把总供应和 `MetadataCap` 交给发布账户。fixture 的单签一次性部署者只用于本地
和测试网演练；主网 Publish PTB 的 sender 必须是发布多签，并在同一 PTB 把发布产生的
`UpgradeCap` 直接交给独立升级多签。随后按创世清单把供应桶和 `MetadataCap` 交给
不同治理地址或对象。OTW Currency 需要第二笔 `finalize_registration` 交易，缺少该
交易不得把状态标成已完成注册。

初始化器不会自动把总供应 Coin 拆成六个分配桶，也不执行团队归属。每个已发布状态
都必须记录真实拆分与角色交接交易，并由独立只读验证器从最终对象余额复算各桶和总量。
归属限制必须由可执行合约或经批准、可验证的托管流程落实；单独填写交易摘要或政策
文本不能作为已落实的证据。

清单因此保存六份逐桶回执，逐一绑定数量、接收角色及 holder、政策、结果对象、交易和
checkpoint；团队归属桶必须指向 Move 合约执行。单一汇总 digest 不能替代逐桶事实。

## 固定供应与元数据

`make_supply_fixed` 消耗 `TreasuryCap`，同时禁止继续铸造和销毁。这意味着：

- 后续奖励只能释放已预铸库存；
- 库存分配、归属和锁定必须可对账；
- 忘记预留的供应不能通过增发补救；
- 需要销毁机制时必须新版本决策，不能声称 fixed supply 支持 burn。

`MetadataCap` 与 `UpgradeCap` 分开保管。测试网允许更新未定图标和描述；主网上线前
必须确定永久元数据、持有多签和是否最终删除 `MetadataCap`。删除不可逆。

主网上线前还必须明确 `UpgradeCap` policy（compatible、additive、dependency-only 或
immutable）并留下治理证据。Move 中公开的供应常量只用于源码和清单交叉检查；上链后
总供应与 fixed-supply 状态只信任 Currency Registry `0xc` 的已注册 Currency 对象。

## 可转让与准入

ESK 货币核心本身不创建 deny list 或全局转账暂停能力，保持可自由转让。地域限制、
制裁筛查、服务购买资格和受限量化产品准入由平台与独立参与协议执行。主网上线前
必须再次显式确认这一不可逆取舍；若目标发行安排要求受监管 Currency，则发布新的
包版本和迁移计划，不能在 UI 中假装原包具有冻结能力。

## 供应和权限交接

创世清单是发布交易的唯一机器可读输入。它必须让所有分配桶精确等于总供应，并为
每个桶指定用途、接收角色及归属政策。任何真实地址进入主网清单前，都要完成所有权
证明、多签阈值和恢复演练。

以下职责不能由同一个单人热钱包长期持有：

- 发布/升级；
- 元数据；
- 项目金库；
- 用户分配；
- 参与协议治理与暂停；
- gas sponsor。

交易所 API key 不属于任何链上治理角色。量化机器人即使拥有交易权限，也不得拥有
提现、ESK 供应、升级、元数据或分配能力。

## 平台账本到链上

现有 ESK Paper 余额继续由主项目追加式账本负责，不能因部署 Coin 自动改成链上余额。
历史付费用户通过独立迁移功能逐条生成 claim：先核对收款，再取得用途确认、成交批次、
披露同意、地址绑定和审批，最后产生幂等链上分配回执。

平台聚合视图可以同时显示 `paper_recorded`、`platform_recorded` 和 `onchain` 三个来源，
但必须分别标注。Paper/平台登记阶段以主项目追加式账本为余额真源；上链后以 Sui 的
数量、所有权和终局性为真源。主项目只维护身份—地址绑定、资格、发行映射和已终局链上
索引投影。迁移完成后通过相反方向的结转分录避免双计，不覆盖旧记录。

## 量化和利润分配

项目融资 USDT 进入项目金库会计后，项目方可以按治理限额投入自营量化；项目承担该
金库头寸损失。只有已实现、已对账、扣除成本和准备金、经审批的团队收入才可以成为
ESK 分配来源。新买家的资金和新铸 ESK 不能支付旧持有人的“收益”。

客户选择量化申购时进入独立 `QSHARE` 产品。项目方“承担损失”应实现为金额上限明确、
可观测的 Sponsor Junior Capital 瀑布；超过上限的损失由产品协议处理，不能承诺无限
兜底。量化 V20 仅作为现有 UI/产品体验及 `fund-product-core`、市场、Paper 基础能力
的复用基线；量化
`origin/main@04b09b5849cf9e64f5992811a21522e3eb72d003` 已接受的 V21
`tokenized-managed-share-product-v21.md` 与
ADR 0004 是 QSHARE-P1、份额、NAV、申赎、Senior/Junior 资本栈和链无关状态投影的
唯一合同权威。量化仓库不复制主项目 ESK 余额，也不改写既有 Paper NET 合同。

## 证据和发布门禁

测试网清单沿 `planned -> local_verified -> testnet_published` 推进；主网清单独立沿
`planned -> local_verified -> mainnet_ready -> mainnet_published` 推进。每次持久化
推进都新增修订并绑定前一清单的 ID、修订号和规范化 UTF-8/LF 内容 SHA-256，旧记录不可覆盖。
没有本地 Sui CLI 时，源码只能标记 `UNCOMPILED`；只有真实运行 `sui move build` 和
`sui move test` 后才能记为 `local_verified`。

测试网和主网发布分别需要 package ID、ESK type tag、发布交易、Currency 注册交易、
checkpoint、注册前待接收 Currency 对象、注册后 Currency 对象、能力对象、供应对象、
chain identifier、`sui client verify-source` 结果和独立只读端点复核；发布端点与独立
端点必须分别记录且不能相同。主网还要求安全审计、多签仪式、恢复演练、准入和迁移
演练证据。本 ADR 不构成签名、广播或资金移动授权。

工具链 release 必须与清单网络一致。当前 `testnet-v1.79.0` 及其摘要只证明测试网
源码的本地可编译性；任何主网清单都必须固定当时批准的 `mainnet-v*` release、协议
兼容证据和新的 build/test 摘要，不能沿用测试网验证结果。

## 本轮验证记录

2026-09-04 使用官方 `sui 1.79.0-46f18562f1f5` 和从相同提交提取的 Sui Framework
执行 `sui move build` 与 `sui move test`。构建无编译警告，三个合同测试全部通过，其中
初始化场景验证总供应 Coin 与 MetadataCap 的交付，并验证没有 TreasuryCap 存活；
三个合同测试全部通过，受限规范化测试回执 SHA-256 为
`e1f934234dd2b6d9236d8e46a1430c732836962787067234368dc1a84212244a`。

2026-09-06 的可复现 CI 复核发现，原先记录的 `b1881cd1...` 是 `move test`
覆盖构建目录后的 test-mode `esk.mv`，包含 `#[test_only]` 入口，不能充当生产发布
字节码证据。CI 改为在 `move build` 后、`move test` 前冻结生产模块；当前生产
`esk.mv` SHA-256 为
`314273ecd53a54793c8b70f35e4a1e853fdc7c6751c20dc0baf0628907b03ca7`。
该修正不改变 Move 源码、测试结果、供应参数或任何链状态；旧摘要只保留为历史错误
说明，后续发布计划不得引用它。

这些证据只把清单推进到 `local_verified`。package ID、type tag、交易摘要、checkpoint
和对象 ID 仍全部为空；没有执行签名、广播、测试网发布或主网发布。

本轮离线验证器会绑定 `Move.toml`、可选 `Move.lock` 和包内全部 `.move` 文件，并在
后续修订中递归读取全部前序清单、重算规范化 UTF-8/LF 内容摘要、检查每份清单、时间递增、合法
状态边和不可变字段。它不具备
治理材料/链查询核验能力，所以明确拒绝全部 `mainnet` 清单和测试网 `published` 清单；
测试网发布功能必须先实现在线查询、chain identifier 复核、逐桶对象复算和第二端点
复核，主网功能还要固定主网兼容包并核验门禁材料，才能认证对应状态。

仓库现已提供固定版本安装、双摘要校验、隔离 Move 依赖缓存、生产字节码冻结和独立
Windows CI job。验证配置显式使用空 keystore、无 Sui 环境的 client 配置与独立
`MOVE_HOME`；build/test 不创建 Sui RPC client，也不读取默认用户配置。冷缓存允许
GitHub 获取固定提交的 Move 依赖，这不构成链查询。后续代理不得把已完成的本地
build/test 或 CI 封装重复列为待实现能力；远程 CI 运行结果仍须按具体提交单独记录。

## 后果

正面后果是 ESK 可以快速获得真实、公开且可验证的资产核心，同时量化热路径和未来
产品迭代不会被 Move 包锁死。固定供应降低了运营密钥被滥用增发的风险。

代价是供应参数错误难以修复，初始库存和能力交接必须一次做对；自由转让 Currency
无法在币层直接冻结受限地址；链下身份和链上地址仍需安全映射；Sui 和未来自有链之间
需要明确迁移协议。

## 被拒绝的替代方案

- 先开发自有公链：延迟真实验证，并同时引入共识、钱包、索引器和桥接风险。
- 把 Binance 量化写入 Move：交易频率、隐私、延迟和交易所托管边界都不合适。
- 保留无限 `TreasuryCap`：虽然运营灵活，但与用户可验证供应的目标冲突。
- 把 ESK、QSHARE 和公司股份合成同一链上对象：会混淆余额、NAV、赎回和法律名册。
- 直接把现有 `task_sui_*` 改成 ESK：该域固定是 CNY 任务影子回执且明确禁止广播。

## 复审触发器

以下变化必须新建 ADR：总供应或精度变化、fixed 改 burn-only、启用 deny list、拆分 ESK
权利、引入第二个团队权益币、绑定法定公司股份、支持新的结算链或把 QSHARE 上链。
