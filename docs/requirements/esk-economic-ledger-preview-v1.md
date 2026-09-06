---
version_status: current
reviewed_at: 2026-09-06
requirement_status: accepted
financial_policy_status: unresolved
feature_id: esk-economic-ledger-preview-v1
owner: esk-primary
---

# ESK 离线资金用途一致性检查 V1

## 目标与边界

当前主任务负责全部新工作，旧任务只收尾已开工的 APK 与 Sui 发布准备。
本批次接续[政策草案合同](esk-early-support-policy-foundation-v1.md)，交付无网络、
无数据库写入的离线检查器：关联购币对账、资金来源声明、投资及准备金用途、
待定保障义务和利润分配提案。接受的是工程范围，经济条款仍未批准。

只证明调用方提供的这一个批次内部一致，不证明收款、利润实现、资金到位、
完整历史、真实归属或没有其他占用。摘要不是可信签名或审计证明。
本工具不创建正式 ESK 余额，不调用平台记账入口，不扩用 QSHARE/Paper 账本，
不改变公开币安卡片的总估值及更新时间范围，不生成应赔款、可兑付利润或付款交易。

## 复用基线

- 主仓基线 `fe488268647165d95d64ae61b91cf83900c0f799`。
- `scripts/esk-paid-reconciliation/preview.js` 校验付款、历史声明、身份映射与报价；
  其 `paymentKey` 是资金来源去重依据，不按本批声明的资金分类另造身份。
- `scripts/esk-early-support-policy/contract.js` 校验两年政策草案并提供规范摘要。
- 正式平台账本及其对账快照仍是既有服务的责任；本批不复制余额或 SQL 表。
- 量化项目 NAV、储备计算采用各自产品假设，不能据此决定新的 ESK 保底条款。

## 输入合同

输入版本 `elon.esk.economic_ledger_preview_input.v1`，`mode=offline_draft`。
精确字段为 `schema`、`mode`、`policy_draft`、`paid_reconciliation`、
`funding_lots`、`obligation_links`、`journal`。
前两个嵌套业务文档直接复用现有检查器，不另定义政策和销售规则。

所有资金数量为 USDT 最小单位的正十进制整数字符串，单笔与聚合不得超过 u128。
全批仅有既有购币对账的一个来源及资产，禁止在资金条目上覆写币种或精度。
不使用浮点数。来源声明不等于资产证明。

### 资金条目

`funding_lots` 最多 200 项，每项精确包含：
`lot_id`、`origin`、`external_payment_reference`、`transfer_index`、`amount_base_units`。
`origin` 只能是 `esk_purchase`、`sponsor_capital`、`realized_profit`。
引用格式及 transfer index 采用既有 payment source 规则。

- `esk_purchase` 必须引用唯一且 `review_ready` 的既有购币行，单位数量完全相同。
- 所有分类使用同一个 `paymentKey`；同一收款不能拆成多个资金条目或改名再次计数。
- sponsor/profit 条目不能引用本批购币对账的任何付款键，即使该行是服务购买、
  QSHARE、被阻止或未列为购币资金条目，也不能重新分类成可分配利润。
- 新的 sponsor/profit 引用只是调用方声明，报告始终保留其真实性未验证。
- 空资金条目或没有用途提案必须明确报问题，不作为有效的完整预览。

### 待定义务关联

`obligation_links` 最多 200 项，精确包含 `obligation_id`、`purchase_lot_id`、
`policy_digest`、`status=PENDING`、`protected_principal_base_units=null`、
`minimum_return_base_units=null`。
本批每个购币资金条目需恰好一个关联，义务 ID 不重复，目标只能是购币条目，
政策摘要须等于既有草案检查器输出。缺失或重复关联是问题，不自动生成法律义务。
未知保障数量保持 null；零不是未知。不得根据 ESK 数量套用 1:1 美元负债。

### 提案事件

`journal` 最多 500 项，只接受顺序提案和完整取消：

- propose：`sequence`、`event_id`、`idempotency_key`、`operation=propose`、
  `request_id`、`lot_id`、`purpose`、`amount_base_units`。
- cancel：`sequence`、`event_id`、`idempotency_key`、`operation=cancel`、`request_id`。
- purpose 仅为 `investment`、`guarantee_reserve`、`profit_distribution`。
- 唯一事件 sequence 从 1 连续增长；事件 ID、提案 ID 唯一。
  相同幂等键且整个事件规范内容一致时忽略重放，否则冲突。
  重放检查先于顺序检查；取消必须引用已存在且未取消的提案，不允许再次使用其提案 ID。
- 全部事件仅在内存中顺序投影；提案状态只能为 PENDING/CANCELED。
  取消只释放预览中的拟分配数量，不执行真实资金解冻。
- 任一时点同条目全部活动用途总额不得超过该条目数量；即使后面取消也不能掩盖之前超额。
- 利润分配提案只能引用声明为 realized_profit 的条目；购币本金和出资不得当作利润。
- 此幂等保护只覆盖完整传入批次；删除历史或换批次重放无法由离线工具识别。
  没有持久化、可信日志锚或跨批次执行保证。

## 输出、安全与命令行

输出版本 `elon.esk.economic_ledger_preview_report.v1`。
有效输入报告 `review_status=consistent|needs_review`；始终 `policy_status=PENDING`。
含输入及政策摘要、政策审阅状态及缺失决定、固定问题码、条目/唯一事件/重放/提案计数。
只有一致批次可输出资金及活动用途合计字符串；有问题时合计为 null，禁止混用部分结果。
不输出原始用户引用、地址、付款引用、责任主体、自由文本或逐项持仓。

固定 `evidence_basis=operator_declared_consistency_only`，
`production_authorized`、`funding_verified`、`profit_realization_verified`、
`coverage_verified`、`funds_moved`、`balances_written` 全部为 false。
政策字段齐全也不改变这些状态；政策一致性问题并入报告，未决字段本身不妨碍引用核对。

输入必须为有界 UTF-8 JSON：最多 1 MiB、深度 12，拒绝重复键、危险对象键、
孤立代理字符、未知字段、越界数量及非整数索引；标识符使用有界 ASCII 全串匹配。
复用严格 JSON 解析和 `readStandardInput` 的 30 秒输入期限，不增加文件或网络入口。
命令 `node scripts/esk-economic-ledger/cli.js` 从 stdin 读取；`--help` 只显示用法。
一致报告退出 0，有问题或非法输入退出 2；错误只输出稳定代码，不回显输入或异常堆栈。
退出 0 只表示批次内部一致，不能被解释成政策批准或可执行交易。

## 文件与分工

| 路径 | 职责 |
|---|---|
| `contracts/esk/economic-ledger-preview-v1.schema.json` | 版本化输入及输出定义，嵌套旧合同委托现有检查器 |
| `contracts/esk/economic-ledger-preview-v1.fixture.json` | 合成购币、投资、准备金与利润提案，无真实账户 |
| `scripts/esk-economic-ledger/` | 输入、来源核对、事件投影、报告、薄 CLI 及独立测试 |
| `docs/requirements/esk-economic-ledger-preview-v1.md` | 本批范围与验收 |
| `docs/delivery/esk-economic-ledger-preview-v1.md` | 实际验证、交付及后续缺口 |

主任务管理需求、认领、审阅、集成及提交；子代理仅修改独占的新模块和测试路径。
不编辑旧政策、购币对账、共享 UI/CI、平台余额、Sui 或旧 APK 路径。
功能注册表仅由正式工具维护。本批是离线开发工具交付，收尾类型 CodePushed。

## 验收

1. 合成完整批次关联既有购币预览及政策摘要；合计精确，政策始终待定，全部执行标志为 false。
2. 同来源跨分类重复、被使用收款、非 ESK 路由、金额不符、错误义务或政策版本明确阻止。
3. 投资与准备金重复占用、销售本金分配、取消前超额、乱序、幂等冲突明确阻止；精确重放不重复计算。
4. 大整数、数量上界、未知/恶意/重复字段、编码与输入超限得到有界错误；输出不泄漏输入明文。
5. CLI 真实子进程、复用模块兼容及独立风险测试通过；源码和证据推送主线并运行正式收尾。
6. 本批不会改变在线资产卡片或完成真实账本、收益证明、保障赔付、售币和链上发行。
