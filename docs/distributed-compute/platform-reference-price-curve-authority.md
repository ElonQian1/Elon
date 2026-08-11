---
title: 平台参考价格回退曲线批次权威
status: current
reviewed_at: 2026-08-11
owners: ai-economy, backend, security
implementation_status: design_frozen
---

# 平台参考价格回退曲线批次权威

## 1. 权威范围与当前结论

本文冻结平台参考价格回退曲线的最小来源合同。平台管理员提交一份精确批次，另一名平台管理员独立复核；只有 exact approved batch 才能由后续 Store-private v223 application 在一个事务中直接生成唯一的既有 v171 Price Snapshot。application 不是第二套 Snapshot Registry，也不是先保存一份等待未知 consumer 的 staging receipt。

本 docs-first 批次只冻结设计与源码计划。领域、Store-private 与 v223 迁移源码尚未写入，未编译、未执行迁移或运行；没有 service、HTTP、MCP 或 PC 入口。后续源码形成前，不得宣称平台参考曲线已经可以提交、复核、应用或进入报价候选。

这条来源仍是 `fallback_curve`：它只表示一份经过平台四眼治理、与当前 Offer 合同精确一致的参考回退报价。`sample_count=0`，不表示平台读取过外部行情、真实成交、已接受交付或订单簿，也不得称为 `index`、`mark`、`trade`、YCI、真实市场价或平台签名价格源。

## 2. 与既有 Offer owner fallback 的关系

v171 已有的本人报价入口根据一份 active Offer 生成 `fallback_curve` Snapshot，来源 ID 绑定该 Offer。平台参考曲线不会替换、升级或复制该入口，而是计划成为同一 v171 Registry 的第二个受控 producer：

- 两条路径都只能写既有 `compute_price_snapshots`，不得建立平行快照表、当前根或 Job 报价权威；
- 本人路径表达 Provider 对自己 Offer 的规范化报价；平台路径表达四眼批准的参考回退批次；
- 两者都不是真实指数、标记价或成交价，`sample_count` 都固定为 `0`；
- Job、Broker 和历史审计继续只消费 v171 Snapshot，不感知 producer 私有 DTO；
- 任一 Snapshot 被引用后都保持不可变，不因批次、Offer 或未来价格变化而重写。

平台路径的来源 ID 固定使用 `platform_reference_curve:<curve_id>`，来源版本固定为正整数 `curve_version`，来源摘要绑定 exact batch entry。该命名只区分来源治理，不提高价格证据等级。

## 3. 平台批次合同

未来 submit 入口只允许当前 `admin/owner` 提交。首版一份批次必须采用拒绝未知字段的版本化信封、RFC 8785 JCS、SHA-256 摘要、稳定幂等键与明确确认，并只包含一个 `curve_id/curve_version` 及 1 至有界上限条 entry。

每条 entry 必须精确绑定：

- `provider_id`、`offer_id`、`offer_version` 与 `offer_digest`；
- `sku_id` 与 `sku_digest`；
- Offer 内一个 `delivery_window_id/delivery_window_digest`；
- `pricing_mode`，首版只允许 `spot` 或 `capacity_future`；
- exact `curve_id/curve_version`，以及 `capacity_future` 所需的 exact `instrument_id`；
- `currency=CNY`、整数微单位价格组件、消费者与 Provider 最大金额；
- `fee_rules=[]`、受限 TTL、舍入模式、稳定 entry ID 和有界说明。

价格、金额、用量和比例不得使用浮点数。entry 不能携带 `trade_id`、样本集合、订单、持仓、容量承诺、外部行情正文或签名私钥。`spot` 不能伪装成真实现货成交，`capacity_future` 也只表示 Offer 已采用未来容量定价合同，不表示存在订单簿、持仓或已锁容量。

同一批次内 `(offer_id, offer_version, delivery_window_id)` 必须唯一；同一 `curve_id/curve_version` 不能绑定不同规范批次。submit 只形成 `submitted` batch/entries，不创建 Snapshot 或任何市场对象。

## 4. 独立复核

reviewer 必须是另一名当前 `admin/owner`，且 `reviewed_by_admin_user_id != submitted_by_admin_user_id`。`approved`、`changes_requested` 与 `rejected` 都生成绑定 exact `batch_id/batch_digest` 的不可变 review receipt；退回或拒绝必须给出有界原因。

复核只确认平台愿意按这份精确参考回退合同生成报价快照。reviewer 不能把 `sample_count=0` 升级为市场观测，不能声明曲线是指数、mark、成交、可交割头寸或外部价格证明，也不能在 review 中更改 Offer、窗口、价格组件、金额或 TTL。

Store-private 源码即使形成，也只能校验管理员 ID 形状和四眼分离；当前角色与权限必须由未来 service/API 独立重审。没有该入口前，数据库方法不得被描述成生产管理员能力。

## 5. Atomic application 与 v171 唯一真源

application 只消费仍为 `approved` 的 exact batch/review、预期摘要、稳定幂等键和固定确认语，不接收新的曲线或价格字段。计划中的事务必须使用一个 `BEGIN IMMEDIATE`，并按每条 entry 重新核验：

1. Provider 与 Offer 存在，Offer 仍为当前 `active` exact revision/digest；
2. Provider、SKU、交付窗口、pricing mode、curve/instrument、币种与价格组件同时精确匹配 entry 和 Offer；双方最大金额精确匹配 entry，并通过 v171 的 checked `i128` 上限校验；
3. Offer 的 `fee_rules=[]`，其有效期和价格条款有效期仍覆盖即将生成的短时 Snapshot；
4. Snapshot/Quote ID 由 batch/entry 身份确定性生成，不能由 application 调用方指定；
5. `quoted_at` 由 Store 生成，`expires_at` 不晚于批准 TTL、Offer 或价格条款有效期；
6. `price_source.source_kind=fallback_curve`、`sample_count=0`、`trade_id=None`，source ID/version/digest 与 exact entry 一致；观察窗口固定为 `[quoted_at-1s, quoted_at]`，只满足回退来源合同，不表示存在市场样本；
7. 通过从既有 v171 Registry 提取的 transaction-local registration kernel 登记并 exact readback Snapshot。

全部 entry、Snapshot binding 与 application receipt 必须在同一事务成功；任一 Offer 变旧、摘要冲突、窗口失效、ID 已被不同合同占用或 v171 校验失败，整批零提交。相同 application 幂等键只允许精确重放，并逐份复核既有 Snapshot；不得跳过当前 Offer 审计或把部分成功称为已应用。

application 的直接效果固定为：每条 entry 恰好对应一份唯一 v171 Snapshot，并保存 entry→Snapshot 的不可变 binding。它不创建新 Snapshot schema，不修改既有 Snapshot，不自动创建或推进 Job。

## 6. v223 计划账本

计划中的 v223 只增加平台参考曲线来源与 v171 binding：

- `compute_platform_reference_price_curve_batches`：规范 batch、exact 投影、状态与幂等；
- `compute_platform_reference_price_curve_entries`：逐 entry 规范合同与批次内唯一性；
- `compute_platform_reference_price_curve_reviews`：一份 batch 对应一份不可变四眼复核；
- `compute_platform_reference_price_curve_applications`：一份 approved batch/review 对应一份不可变 application receipt；
- `compute_platform_reference_price_curve_snapshot_bindings`：每条 entry 精确绑定一份既有 v171 Snapshot。

DDL 必须用投影、追加式历史、禁止 replace/update/delete、状态来源、反向边和唯一性门卫阻断拆分写入。batch 只允许 `submitted→approved|changes_requested|rejected`，以及 `approved→applied`；entry、review、application 与 binding 均不可覆盖。

v223 不得新建另一张 Price Snapshot 表，不得修改 v171 历史行，也不得创建 Curve current root、Order、Trade、Position、Commitment 或 Clearing Receipt。v171 transaction-local kernel 只能保持 Store-private，由既有公开 wrapper 与 v223 application 事务复用；不能因此开放通用“任意 Snapshot”写入口。

## 7. 市场、容量与资金效果

成功 application 计划返回以下固定效果：

- `market_effect: quote_candidate_enabled`；
- `job_effect: none`；
- `reservation_effect: none`；
- `capacity_effect: none`；
- `funds_effect: none`；
- `settlement_effect: none`。

现有候选查询仍须重审 Provider/Offer 当前状态、Snapshot 未过期、Job SKU/窗口和消费者预算；存在一份平台参考 Snapshot 不等于已经选择报价。只有后续消费者显式锁定 Snapshot，Broker 才能按既有合同尝试容量和 CNY 预授权，v223 application 本身不能调用这些步骤。

## 8. P0 禁线

- 禁止把平台提交、管理员批准或 application receipt 称为外部行情验证、YCI、index、mark、trade 或真实价格发现。
- 禁止设置非零 `sample_count`、绑定 `trade_id`，或使用 `PRICE_SOURCE_INDEX`、`PRICE_SOURCE_MARK`、`PRICE_SOURCE_TRADE`。
- 禁止绕开 exact active Offer，把平台 entry 的价格强行覆盖到不一致的 Offer。
- 禁止建立第二套 Snapshot、Job、Reservation、Claim、Broker 或结算注册表。
- 禁止让 application 创建 Job、选择报价、预留容量、冻结余额、派发 Attempt、生成用量/Receipt 或移动任何资金。
- 禁止因 docs、领域或 v223 源码形成就宣称 HTTP/MCP/PC、角色权限、并发、迁移或生产链路可用。

## 9. 交付与验收状态

本 docs-first 状态为 `design_frozen`。后续同批计划形成领域合同、Store-private submit/review/application/exact readback、v223 DDL/trigger 和 v171 transaction-local kernel 接线；仍不开放 service、HTTP、MCP 或 PC。

按当前架构铺设要求，只允许定向格式化、源码/文档模块化、链接/术语搜索、行数和 `git diff --check`。不执行编译、测试、迁移、SQLite trigger、权限、并发、HTTP 或真实运行验证。源码形成后状态最多提升为 `implementation_uncompiled`、`implementation_unrun`，不能宣称生产可用。
