---
title: "ESK Sui 测试网发行离线预检 V1"
status: accepted
implementation_status: implemented
owner: platform-assets, protocol, protocol-ci
priority: p0
reviewed_at: 2026-09-06
decision_refs:
  - "docs/decisions/esk-sui-economic-foundation-v1.md"
  - "docs/decisions/esk-sui-allocation-vesting-v1.md"
  - "docs/requirements/esk-first-user-delivery-roadmap-v1.md"
  - "docs/requirements/esk-platform-sui-address-binding-v2.md"
---

# ESK Sui 测试网发行离线预检 V1

## 用户结果

项目方可以在不接触钱包、私钥、RPC、Gas 或链交易的环境中，把 ESK 测试网候选参数
交给一个严格的离线预检器。预检器输出字节稳定、可复算摘要的发行仪式计划，并明确
区分“合同已经验证”“仍待填写/审批”“可以申请执行授权”三种状态。

本功能不会真实发币。它不构建可签名交易字节，不读取默认 Sui 配置，不查询链、不领
测试币、不签名、不广播、不移动资金，也不会把平台 ESK 余额标成链上余额。未来实际
测试网发行必须登记为独立的 `esk-sui-testnet-publication-v1`，并在执行前单独确认网络、
钱包、Gas、正式公开参数和明确授权。

## 待明确的商业政策修订

项目方新增意向为：ESK 销售所得由团队用于投资，并在项目早期考虑承担两年的损失兜底。
这只是待完善的商业政策方向，不是本合同已经确定的保本、保收益或兑付承诺。当前仍需
项目方逐项明确保障本金还是本金加最低收益、两年按项目统一截止还是按每笔购买日起计、
结算资产、责任主体、补亏资金来源、收益计算与分配、转让或购买服务后的权利归属、提前
退出、到期结算，以及投资亏损和团队补足的会计记录。

V1 候选必须把该事项固定标为 `clarification_required`，只记录 24 个月政策意向和上述
未决条款；`approved_policy_digest` 保持空值，公开售币、自动收款、投资、收益分配和补足
功能保持关闭。旧版无保障口径和任何样例参数都不得自动晋级为正式售币合同。该政策不
自动适用于 QSHARE，也不改变 ESK、QSHARE、公司股份和服务订单分别记账的边界。

这项未决商业政策不阻断本功能的严格离线预检，也不自动阻断未来经单独授权的纯技术
testnet 验证；但在新售币合同或相关资金自动化启用前，必须形成新的版本化政策合同、
明确条款并取得批准摘要。当前实现不得替项目方补写答案。

## 依赖与复用

1. 复用固定供应 Currency、六桶与团队线性锁仓的既有 Move 源码，不修改其经济语义。
2. 固定使用 `testnet-v1.79.0`、源码提交
   `46f18562f1f5af2438d35828e8b62d5e0b972db7`，以及已经远端验证的生产字节码摘要：
   Currency `sha256:314273ecd53a54793c8b70f35e4a1e853fdc7c6751c20dc0baf0628907b03ca7`，
   Participation 本地依赖基线
   `sha256:fa691e2e7d7c1c347b8fd88a2dc9f3ca2590ee56813c0bb313ef2ea8d477d3ef`。
3. Participation 当前仍通过 `../esk_currency` 本地依赖构建，地址为 `0x0`。Currency 产生
   真实 package ID 后，必须在独立候选目录绑定该 ID、重新 build/test 并冻结新的发布
   字节码；上述本地基线摘要不得冒充最终 Participation 发布载荷。
4. 复用发布、Currency、六桶分配三个只读观察器作为发布后输入模板；观察器结果在本功能
   中必须保持未运行，不能伪造 package、对象、交易或 checkpoint。
5. 平台认证地址绑定 V2 只覆盖用户自持地址，不证明项目职责地址、多签阈值、恢复能力
   或能力对象托管。项目职责必须由单独的公开候选参数和审批证据描述。

## 输入合同

入口只读取调用方显式指定的一个本地普通 JSON 文件，最大 128 KiB。输入采用
`yilong.esk.sui.testnet_publication_candidate.v1`，拒绝未知字段、重复键、BOM、符号
链接、网络/设备路径、非规范 Base64 和任何秘密字段名或秘密值。

候选至少包含：

- `scope`：固定 `network=testnet`；模式只能是 `template`、`synthetic_test` 或
  `release_candidate`；
- 固定工具链、两个包的源码/生产字节码摘要和仓库基线提交；
- 固定 1,000,000,000 ESK、6 位精度、六桶 base units 与 basis points；
- 固定 `commercial_policy_revision.status=clarification_required`、24 个月早期团队兜底
  意向、QSHARE 不自动适用、未决条款列表和全部售币/资金自动化关闭标志；
- distribution、treasury、liquidity、team beneficiary、metadata、currency upgrade、
  participation upgrade、pause、gas sponsor 和 deployer 的公开 Sui 地址；
- 团队 `start_ms`、`cliff_ms`、`end_ms`，测试网升级策略，以及每笔交易最大 Gas 预算；
- 经济参数、职责地址控制、多签/恢复演练和发布候选复审的批准摘要及时间。

`template` 允许上述待用户决定字段为 `null`，只能得到阻断报告。`synthetic_test` 只供
自动测试，地址必须显式带合成标志且输出永远不可申请执行授权。`release_candidate`
拒绝空值、零地址、重复长期职责地址、合成地址、过期批准、金额/比例不守恒、非法
时间顺序、`upgrade_policy=pending`、缺批准摘要以及超出固定上限的 Gas 预算。

输入不得包含助记词、私钥、keystore、密码、Bearer/API token、Cookie、签名、Gas coin
对象、交易字节、交易摘要或链上对象/checkpoint 结果。公开地址和 SHA-256 审批摘要不是
秘密，但它们不等于控制权已经由本工具验证。

## 输出合同

输出采用 `yilong.esk.sui.testnet_publication_preflight.v1`，对移除根
`plan_sha256` 后的递归 ASCII 键升序、数组顺序保持、无空白规范 JSON 计算 SHA-256。
相同输入与相同仓库源码必须得到相同计划摘要；生成时间不得进入摘要或计划正文。

输出包含：

- 候选 ID、模式、固定源码与工具链摘要、参数摘要和职责地址摘要；
- 有序 DAG：Currency publish → Currency Registry 最终注册 → 用真实 Currency package
  ID 重绑定并重建 Participation → Participation publish → 一次性六桶分配/团队锁仓 →
  MetadataCap 与两枚 UpgradeCap 等职责交接 → 三观察器双源复核 → Evidence/Manifest V2；
- 每个步骤的前置条件、授权角色、公开输入、待产生输出槽位、最大 Gas 和失败停止规则；
- 发布后观察器的待填模板，以及源码对应性、能力归属、终局性和部分成功恢复清单；
- `blocking_reasons`、`user_actions_required` 和明确的下一安全动作。
- 参数摘要中完整展示待明确商业政策，且明确技术 testnet 预检与公开售币激活是两条
  独立状态轴。

计划中的 package/object/transaction/checkpoint、签名、Gas payment、RPC endpoint、链 ID、
PTB/transaction bytes 等链输出必须全部为 `null`。固定真实性边界为：

- `execution_authorized=false`；
- `transactions_constructed=false`；
- `transactions_signed=false`；
- `transactions_broadcast=false`；
- `rpc_queried=false`；
- `funds_moved=false`；
- `public_sale_activation_allowed=false`；
- `funds_acceptance_automation_allowed=false`；
- `investment_automation_allowed=false`；
- `return_or_top_up_automation_allowed=false`；
- `publication_status=not_performed`；
- `chain_finality_verified=false`；
- `asset_identity_verified=false`；
- `balance_eligible=false`；
- `manifest_transition_allowed=false`。

完整 `release_candidate` 只能得到 `candidate_status=prepared_not_authorized`，表示技术
发行参数可以提交人工执行审批；它仍不能执行任何交易，也不表示未决商业政策已经完成
或公开售币可以启用。模板输出为 `user_action_required`，合成测试输出为
`synthetic_verified`。

## 多交易恢复与停止边界

计划必须逐项描述未来执行器的追加式 attempt journal 和未知结果处理，但本功能不创建
该 journal。任何实际阶段出现未知提交结果时，未来执行器必须先按已知交易摘要或稳定
请求键查询，不得盲目重发。Currency 发布成功而注册失败、Participation 重建/发布失败、
allocation 中止、cap 交接失败、观察器分歧或终局证据不足时，都必须停止且不得推进平台
余额。链上成功无法靠数据库回滚，只能保留证据、隔离后续步骤并按批准流程恢复。

## CLI 与无副作用边界

```text
node scripts/prepare-esk-sui-testnet-publication.js preflight <candidate.json>
node scripts/prepare-esk-sui-testnet-publication.js template
```

- `template` 只向标准输出写一个不含真实地址、批准或秘密的待填写 JSON 模板。
- `preflight` 只读显式文件和固定仓库文件，标准输出只有机器 JSON。
- 失败只返回固定错误码，不回显输入、路径、地址、摘要之外的值或底层异常。
- 源码不得导入网络、子进程、钱包、Sui RPC client、交易构造器、签名器或环境变量接口。
- CLI 不写仓库、用户目录、Sui 配置、数据库或链；不调用任何外部命令。

## 验收标准

1. 模板、合成向量和完整 release-candidate 各有确定性测试；同一输入重排 JSON 字段或
   改变空白后计划摘要不变，任一语义字段变化都会改变摘要或触发失败。
2. 固定工具链、源码、Currency/Participation 基线摘要与当前仓库一致；漂移失败关闭。
3. 供应/六桶守恒、地址规范与隔离、锁仓时间、升级策略、批准摘要/时间和 Gas 上限均有
   正反例；现有 synthetic fixture 不能被自动晋级为正式候选。
4. 待明确商业政策在模板、合成和 release-candidate 中都保持未决；样例/旧条款不得晋级，
   QSHARE 不自动纳入，公开售币及收款、投资、收益/补足自动化固定关闭。
5. DAG 精确包含两包顺序、Currency 最终注册、Participation 真实 package ID 重绑定及
   重新 build/test 门禁、一次性 allocation、能力交接、三观察器和 V2 证据交接。
6. 所有链输出保持空值，全部真实性/资金/迁移标志保持 false；静态和运行守卫证明零
   wallet、RPC、network、transaction、sign、broadcast 和 funds 副作用。
7. 既有 Genesis、Allocation、固定工具链 CI、三个观察器及地址绑定合同回归不受影响；
   源码规模、文档模块化与 Feature Registry 漂移门禁通过。

## 明确不做

- 不批准或编造正式供应、六桶、日期、职责地址、多签、恢复或 Gas 参数。
- 不生成、序列化、dry-run 或提交 PTB/TransactionData，也不创建签名请求。
- 不访问 Sui testnet/mainnet、钱包、faucet、交易所、USDT 或币安 API。
- 不修改 Move.toml 为虚构 package ID，不发布或注册 Currency/Participation，不分配 ESK。
- 不生成 Evidence/Manifest V2 的成功证据，不改变平台余额或任何用户链上状态。

## 后续交接

项目方提供并批准公开 release-candidate 参数后，本工具先生成
`prepared_not_authorized` 计划。随后新建 `esk-sui-testnet-publication-v1`，逐步取得
只读 testnet/dry-run、Gas、钱包/多签签名、广播和发布后验证的单独授权。任何授权只
覆盖明确步骤，不能由“继续开发”或本预检计划推导。
