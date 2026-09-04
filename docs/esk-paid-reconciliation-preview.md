---
title: "ESK 历史付款对账预演操作手册"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets]
---

# ESK 历史付款对账预演操作手册

## 能解决什么

在把用户清单交给任何登记接口前，先查同一付款是否重复、是否已有历史用途、
脱敏用户映射是否唯一，以及拟登记 ESK 是否与明确销售条款一致。
它补在已完成的 [Paper 批量登记](requirements/esk-paper-first-user-allocation-operations-v1.md)
之前，不改写该接口，也不输出其可直接提交的 `user_id/entries/confirmation` 清单。

输入的“confirmed”、同意和审批摘要均是运营声明；工具只检查声明的一致性，
不会访问交易所、区块链、主项目数据库或验证签名，不代表核实了真实到账。
真实付款必须先从权威账务来源核对，再由后续受控入账流程重新检查。

## 本地运行

需要 Node.js 18+。输入是严格 UTF-8、无 BOM 的 JSON，经标准输入传入；
最多 1 MiB、12 层、1000 个付款行。禁止未知字段、重复键（包括转义后重名）、
非整数 JSON 数字、金额浮点数、私钥、姓名、邮箱、截图或聊天原文。
只允许零参数运行或 `--help`，没有 `--commit` 模式，没有文件写入或联网能力。
标准输入超过 30 秒仍未完成会失败退出。

仅用合成示例进行演练，在仓库根的命令提示符中运行：

```text
node scripts/preview-esk-paid-reconciliation.js < contracts/assets/esk-paid-reconciliation-v1.fixture.json
```

PowerShell 中可运行同一个命令提示符命令，避免文本管道改变编码：

```powershell
cmd.exe /d /c "node scripts/preview-esk-paid-reconciliation.js < contracts/assets/esk-paid-reconciliation-v1.fixture.json"
```

真正的输入应由运营在仓库之外保管，经安全的标准输入管道传入；不要把付款文件或
输出提交到 Git，也不要把凭据放在命令参数中。程序只输出脱敏复核结果，不自动保存。
仓库 fixture 所有主体、凭据摘要、付款和条款都是合成值，其价格不构成销售承诺。

## 输入职责

完整结构见 [合成 fixture](../contracts/assets/esk-paid-reconciliation-v1.fixture.json)。

| 字段 | 含义与负责人 |
| --- | --- |
| `batch_id / as_of` | 本轮复核标识和固定 UTC 毫秒时间；不是自动生成的新付款身份 |
| `source` | 经人工确认的付款账户命名空间、网络、USDT 精确资产定位、精度、引用编码 |
| `snapshot` | 同来源的历史已用付款指纹快照、覆盖声明和时间；不能从空数组推定完整 |
| `users` | 脱敏主体 SHA-256 到脱敏目标用户 SHA-256 的一对一映射；存在性仍待服务端复核 |
| `sale_batches` | 明确批准的支付 base units/ESK base units 比例、披露版本和条款摘要 |
| `rows` | 每笔付款的外部引用和事件序号、精确金额、付款状态、主体、单一用途、拟分配及同意/审批摘要 |

`source.reference_format=hex32` 时付款引用可带 `0x/0X`，大小写会规范化；
`opaque` 时引用严格大小写敏感。十六进制资产地址按补齐 64 位、转小写规范化，
Base58/提供商资产标识保留大小写。接入方必须事先规定正确的来源命名空间和事件序号
语义，不得为同一账务来源起别名；跨命名空间、跨网络或不同资产标识不做猜测合并。

付款键由 `identity.js` 的 `paymentKey(source,row)` 生成，绑定规范化来源、付款引用和
事件序号，不绑定 batch、行名、用户、金额或销售批次。同一交易可以有多个转账事件，
必须由来源适配器给出正确事件序号；工具不会证明事件序号对应真实转账。

`sourceFingerprint(source)` 绑定来源配置，包括精度和引用编码；运营工具应调用
该函数构造历史快照的来源摘要。变更来源配置时必须重新取得匹配的历史快照，
不能只重新计算摘要使旧快照看似有效。原 CLI 仍接收人工历史快照；正式平台账本的
占用记录现由 [只读快照接入](esk-platform-reconciliation-snapshot.md) 补充。
该接入不核实外部到账，也不能替代旧系统、其他产品和外部付款历史的覆盖核对。

USDT 金额按声明的 0..18 位精度转换为整数，最大 u128。ESK 单条和单用户拟分配
不超过主项目 i64 base units；合计只使用 BigInt。销售比率必须整除到 ESK 最小单位，
不默认 1:1、不自动四舍五入、不推断历史购买价或手续费。含费用的订单应由后续
明确的净额/费用合同处理，本版本不接受未说明的扣费差额。

## 结果如何使用

结果 Schema：`yilong.esk.paid_reconciliation_preview.v1`。

- `review_ready`：指定快照范围内一致，可以进入人工复核；不是已入账。
- `blocked`：该行有待解决的明确原因，不计入 `proposed_totals`。
- `routed_elsewhere`：已声明服务购买或量化申购，进入相应流程，不计入 ESK。
- `proposed_totals`：只累计待人工复核的 ESK 行，仍是提议数量，不是任何余额。

退出码 `0` 表示无阻塞的预演（可能全部路由到其他产品）；`2` 表示有业务缺口；
`1` 表示输入格式不合法。无论退出码如何，下列字段始终为 false：
`funds_moved`、`balances_written`、`commit_eligible`、`payment_authenticity_verified`、
`identity_verified`、`approvals_verified`。

逐行错误示例：`DUPLICATE_BATCH_PAYMENT/PAYMENT_ALREADY_USED` 是重复付款；
`SUBJECT_MAPPING_MISSING/*_AMBIGUOUS` 是映射缺失或冲突；
`SNAPSHOT_SOURCE_MISMATCH/HISTORY_INCOMPLETE/SNAPSHOT_STALE` 是快照不能用于本次检查；
`CONSENT_MISSING/APPROVAL_MISSING` 要补确认；`ESK_QUOTE_MISMATCH/
NON_INTEGRAL_ESK_QUOTE/USER_TOTAL_OVERFLOW` 要复核条款和精确数量。

`input_digest` 绑定规范 JSON 输入，`snapshot_digest` 绑定历史快照；
`report_digest` 的算法是把结果中该字段置 null 后，以 `identity.js` 的规范序列化
计算 SHA-256。修改批次、付款、条款或快照都会改变复核输入摘要。摘要不是签名，
不能证明来源真实性或防止有权改写整份输入的人重新生成摘要。脱敏哈希也不是加密；
主体映射应在可信系统中生成，不把低熵姓名/电话直接做公开哈希。

## 验证与后续交接

```text
node scripts/test-esk-paid-reconciliation.js
node scripts/test-esk-asset-contract.js
node scripts/test-esk-sui-genesis-foundation.js
```

本轮为 `implemented + offline_passed` 工具切片；没有执行真实用户数据验收、
服务器部署、APK 发布、真实付款核验或余额写入。共享 Feature Registry 绑定当前
需求、源码和测试；准确推送身份以提交及统一收尾回执为准。

当前扩展：[正式付款占用快照与预演接入](esk-platform-reconciliation-snapshot.md)，
已有 [正式登记账本](esk-platform-recorded-assets.md)。本手册原 CLI 的离线属性不变。
待完成：外部历史来源覆盖、运营用户映射核实，以及授权后的真实付款与逐笔审批验收。
实际写入必须在服务端事务中再次检查付款唯一性、用途、当前余额与审批，不能只信任
离线 `review_ready`；随后才是地址绑定、claim、迁移反向结转与两端资产来源展示。
ESK 销售和 QSHARE 申购不能共用同一笔付款，不能据本报告自动进量化仓位。
