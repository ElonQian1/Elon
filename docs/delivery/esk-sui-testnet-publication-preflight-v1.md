---
title: "ESK Sui 测试网发行离线预检 V1 交付"
status: current
implementation_status: implemented
verification_status: integration_passed
delivery_status: not_started
acceptance_status: accepted
owner: platform-assets-protocol-ci
reviewed_at: 2026-09-06
requirement: docs/requirements/esk-sui-testnet-publication-preflight-v1.md
---

# ESK Sui 测试网发行离线预检 V1 交付

## 本批结果

本批把已经固定的 ESK Currency、六桶分配和团队锁仓源码，接入一个严格离线的测试网
发行候选合同与仪式计划。它帮助项目方在申请真实执行授权以前发现参数、源码、对象接线
和恢复流程问题。

本批不是发行执行器。它不读取钱包、私钥或默认 Sui 配置，不查询 RPC，不领取测试币，
不构建交易字节，不请求签名，不广播，不移动资金，也不改变平台或用户余额。

## 交付矩阵

| 能力 | 实现 | 验证 | 交付 | 验收 | 当前证据 | 剩余缺口 |
|---|---|---|---|---|---|---|
| 严格候选合同与公开模板 | implemented | integration_passed | not_started | accepted | 45 项专项与三种候选模式 Schema 通过 | 远端 CI |
| 固定源码和工具链绑定 | implemented | integration_passed | not_started | accepted | 固定 Sui 版本、递归源码清单、两包输入和生产摘要均失败关闭 | 远端 CI |
| 无签名八阶段发行计划 | implemented | integration_passed | not_started | accepted | 赞助交易、Publish、完整 ObjectRef、Registry/allocation ABI 和 Clock 已锁定 | 远端 CI |
| 部分成功恢复与发布后证据交接 | implemented | integration_passed | not_started | accepted | 追加式 journal、三观察器及能力/源码对应/委员会终局 verifier 门禁已进入计划 | 三个新增证据生产器属于后续发行功能 |
| 两年早期团队兜底政策边界 | implemented | integration_passed | not_started | accepted | 候选固定显示 `clarification_required`、11 项未决条款、QSHARE 不自动适用及四类资金/售币关闭标志 | 项目方回答范围、起算、资产、主体、资金来源及退出/结算等问题后另发政策合同 |
| 真实 Sui 测试网发行 | not_started | not_run | not_started | deferred | `publication_status=not_performed` | 独立功能、正式参数、钱包、Gas 和逐步明确授权 |

状态轴彼此独立：本地测试通过不会变成真实发行，代码推送也不会变成链上验收。

## 仪式边界

离线计划固定描述以下依赖关系：

1. 发布 Currency，固定 `Command::Publish` 的有序模块与 `0x1`/`0x2` 依赖；Publish 是
   command 0，其唯一 `UpgradeCap` 返回值必须以 `Argument::Result(0)` 交给同一 PTB 的
   command 1 `TransferObjects`，不得误用 `NestedResult` 或留下未消费结果；同时记录待注册
   Currency、初始固定供应 Coin、MetadataCap 和 UpgradeCap；
2. 调用 `0x2::coin_registry::finalize_registration`，执行时只读解析并绑定 `0xc` 实际
   initial shared version，再以 mutable shared 引用和前一笔终局 effects 中的 Receiving
   `(id, version, digest)` 完成 OTW Currency 注册；不得猜测 `0xc` 的共享版本；
3. 用真实 Currency package ID 在隔离候选目录重绑定、重建并测试 Participation；
4. 发布 Participation，固定模块顺序及 `0x1`、`0x2`、真实 Currency package ID 依赖，
   并按与 Currency 相同的 `Result(0)` → 同 PTB `TransferObjects` 规则处理 Publish 返回的
   UpgradeCap，记录唯一 GenesisAllocationCap 和 UpgradeCap；
5. 由 deployer 使用前序终局 effects 中的完整 Coin/Cap ObjectRef 执行一次性六桶分配；
   `plan_sha256` 去前缀并十六进制解码为 32 字节，`0x6` 作为 immutable Clock，合约执行
   时仍强制 `clock.timestamp_ms <= start_ms`；签名和广播前还必须使用获准的只读 Clock
   或 dry-run 重复同一门禁，失败时不得签名或广播；
6. 按批准的 policy 处理两枚 UpgradeCap，并把现有能力交给批准角色：compatible 先验证
   policy 0 再转移；additive 调用 `0x2::package::only_additive_upgrades(&mut cap)` 后转移；
   dependency-only 调用 `only_dep_upgrades(&mut cap)` 后转移；immutable 调用
   `make_immutable(cap)` 按值销毁。三种转移及销毁都固定在能力交接的同一 PTB 中，并由
   publish effects 绑定 cap 的 package/version。当前源码没有 pause cap，pause 地址仅为
   未来保留角色；
7. 用发布、Currency 和分配三个既有观察器进行双源复核；分配观察器固定
   `allocator=roles.deployer`。另设能力交接、源码对应和委员会终局三个 verifier，
   能力 verifier 必须覆盖 MetadataCap 与两枚 UpgradeCap；
8. 只有能力交接、源码对应性、委员会终局性和其余观察结果均有真实生产器并通过后，
   才申请 Evidence/Manifest V2 交接。现有 publication observer 只证明两个 RPC 报告对
   package/交易/checkpoint 的一致性，不冒充源码对应或委员会签名终局证明。

所有 package、object、transaction、checkpoint、RPC、Gas payment、签名与交易字节输出在
本批必须保持 `null`；所有执行、链终局性、资产身份、余额资格和平台迁移标志必须保持
未执行或 `false`。

新增商业政策只按“团队考虑对 ESK 销售所得投资提供两年早期损失兜底”记录为待明确
意向。保障范围、起算方式、计价资产、责任主体、补亏来源、收益计算、转让/消费权利、
退出、到期结算和补足会计均未确定，因此旧无保障口径和样例都不能晋级正式售币合同。
QSHARE 不自动适用；公开售币、自动收款、投资、收益分配及补足自动化均保持关闭。这个
门禁不阻断本批离线技术预检，但必须在真实售币或资金功能启用前以新版政策合同解决。

五笔链交易都固定 `TransactionData.sender=deployer`、`GasData.owner=gas_sponsor`，逐笔从
追加式 attempt journal 取得最新 Gas `(id, version, digest)` 和参考 Gas price，并使用候选
内对应预算；发布者与 Gas sponsor 两个角色都必须签署同一笔赞助交易。这个合同只描述
未来执行器的输入，不读取 Gas、不构造交易，也不请求任何签名。

## 恢复规则

- 真实执行器必须使用不可覆盖的追加式 attempt journal；本批只定义合同，不创建 journal。
- 提交结果未知时，必须先按已知交易摘要或稳定请求键查询，禁止盲目重发。
- Currency 发布成功但注册失败时，保留成功证据并停止 Participation。
- Participation 重建或发布失败时，不得使用本地 `0x0` 依赖摘要冒充发布载荷。
- 分配交易结果未知时，先查询交易及对象并停止，不得重新执行一次性分配。
- 能力交接、观察器或终局性不完整时，平台余额不得晋级为已上链。

## 正式候选仍需项目方提供

这里只需要公开参数和审批证明，不需要向代码库或 AI 提供私钥：

- distribution、treasury、liquidity、team beneficiary、metadata、两类 upgrade、保留 pause、gas sponsor 和 deployer 的公开测试网地址；
- 团队锁仓 `start_ms`、`cliff_ms`、`end_ms`；
- Currency 与 Participation 的升级策略；
- 五个阶段各自的最大 Gas 预算；
- 经济参数、职责控制、多签恢复演练和候选复审的批准摘要与有效期。

候选完整时最多得到 `prepared_not_authorized`。这表示可以提交下一阶段人工审批，不表示
可以访问钱包或执行交易。候选内四个 approval digest 的真实性及其各自计划前 subject
绑定仍固定为未验证；它们只是计划前置证据，不是执行授权。真实执行必须在计划摘要
产生后另建不参与该摘要计算的步骤级 attestation，明确绑定 `plan_sha256`、授权步骤、
签署地址、阈值和有效期。能力交接、源码对应和委员会终局三个证据生产器尚未实现时也会
作为阻断理由保留，不能越过。

## 真实发行的后续入口

后续必须新建并认领 `esk-sui-testnet-publication-v1`，再按步骤分别确认只读网络检查、
dry-run、Gas、钱包或多签、签名、广播和发布后观察器验证。测试网成功不能自动升级为
主网发行；主网密钥、生产 Gas、发行和资金操作仍需独立授权及安全审查。

真实发行完成后，仍需单独完成用户地址绑定、历史付款对账、分配导入、平台余额反向
结转和双计防护，才能把用户资产标成已上链。

## 验证与交付证据

- 本地专项：45/45 通过；三种候选模式和四种升级策略输出 Schema 通过。
- 既有观察器回归：发布 65、Currency 312、分配 98、共享 transport 15、地址绑定 58
  项均通过；网络写入为零，transport 仅使用 stub。
- 固定供应/分配回归：Genesis foundation 与 allocation/vesting 全部通过。
- 三路独立复审：最终快照均为 P0=0、P1=0、P2=0；“考虑兜底”措辞、决策中立动作和
  候选对象快照问题已在复审中修正并加入回归。
- 固定 Sui Move 构建与测试：工具链合同、本地无钱包/无 RPC 边界及 CI 接线守卫通过；
  本批仍需远端 CI 复核。
- Git 提交与远端分支：尚未产生。
- Sui 网络、钱包、签名、广播、资金和真实用户验收：均未执行。
