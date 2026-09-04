---
title: "ESK 正式账本付款快照接入手册"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets]
---

# ESK 正式账本付款快照接入手册

## 用途与边界

把主项目正式账本已占用的付款键交给历史付款预演，避免把已准备、已登记的同一付款
再次列为待人工复核项。未取消准备和正式入账都占用；仅取消且未重新准备才释放。
正式登记事务仍须重新验证唯一性和审批，离线预演不是最终授权。

此接口不查询交易所、USDT 链上到账或 Paper 历史，不改变 ESK 数量、资金和政策。
`platform_history_complete=true` 只覆盖本次读取的正式平台范围；外部及其他产品历史
仍由运营核对。完整需求见 [需求 V1](requirements/esk-platform-reconciliation-snapshot-v1.md)，
实际验证与上线状态见 [交付证据](delivery/esk-platform-reconciliation-snapshot-v1.md)。

## 获取快照

`GET /api/admin/assets/esk/platform-reconciliation-snapshot`，不加任何查询参数，请求体为空。
使用真实、有效、active 的数据库 admin/owner 会话；普通用户、静态管理员凭据、
虚拟 owner、已撤销或过期会话不允许读取。接口复用主项目服务器，无需子项目服务器。

通过现有受保护的管理员连接读取。当前公共 HTTP 可用于公开 APK 下载和匿名检查，
不能在其上发送真实会话或私有付款材料；没有受保护连接时先停止私人读取。
不要把会话放在 URL、命令参数、Git、聊天或诊断日志里。

响应的 15 个字段原样使用，不手工删键、补键或重算摘要来绕过失败。响应不含原始
付款引用、用户标识、金额或审批材料，使用 `no-store/no-cache/no-referrer`。
已有通用认证可能更新会话 `last_seen_at`；快照 Store 事务本身只读，不写任何业务数据。

| 结果 | 处理 |
| --- | --- |
| 200 | 获得固定政策、同一事务内的完整有界占用快照；暂停新登记不影响读取 |
| 401 / 403 | 修复登录或权限，不换成静态管理员凭据 |
| 400 | 移除查询字段和请求体；不要发送 JSON `null` |
| 503 `ESK_PLATFORM_INVALID_POLICY` | 尚无已固定政策；不能当作空历史，也不要为读取而开启登记 |
| 409 `ESK_PLATFORM_LIMIT_EXCEEDED` | 超过 10000 个占用键；不能截断后宣称完整，需另行设计扩容 |
| 500 `ESK_PLATFORM_LEDGER_INCONSISTENT` | 账本关系或政策损坏；暂停使用该快照并检查原因 |

## 送入离线预演

先按 [原预演手册](esk-paid-reconciliation-preview.md) 整理来源、外部/其他系统历史、
脱敏用户映射、销售条款和付款行。原 `snapshot` 仍必须独立满足覆盖、来源和时效检查。
将该原 V1 对象和接口的完整响应置于以下信封（占位文字不是可运行付款样例）：

```text
schema: "yilong.esk.platform_reconciliation_input.v1"
reconciliation: 原 V1 完整输入对象
platform_snapshot: 本次接口完整响应对象
```

安全标准输入管道的目标命令为 `node scripts/preview-esk-platform-reconciliation.js`。
只接受零参数或 `--help`；不支持输入文件路径、Token、URL、`--commit` 参数。
严格 UTF-8 JSON，无 BOM/重复键/未知字段；整个信封最多 1 MiB、12 层，输入 30 秒超时。
限额是整份输入的共同上限，不保证所有字段都同时容纳各自最大数量。
不要将真实输入或报告提交 Git；工具不联网、不写文件、不输出可直接登记的请求。

程序先检查平台快照摘要、来源、严格升序和计数，并要求其观测时间在复核时间
`reconciliation.as_of` 之前或相同、相差不超过 24 小时。随后合并占用键，交给原预演。
两来源交叠不算错误；人工历史内部重复、过期、未来时间或未完整核对仍然阻断。
平台来源不匹配、超限、被改写等会整体失败，不降级到缺少平台快照的模式。

输出为 `yilong.esk.platform_reconciliation_preview.v1`，原逐行报告位于 `preview`。
`input_digest` 绑定完整原信封，`platform_snapshot_digest` 绑定提供的平台响应。
`report_digest` 把自身置 `null` 后做规范 JSON SHA-256；摘要不是数字签名。
报告明确 `platform_snapshot_authenticity_verified=false`，不会证明输入来自正式服务器。

退出码 0 是无阻塞预演，2 是业务待复核，1 是格式或快照错误。即使为 0，仍然
`funds_moved=false`、`balances_written=false`、`commit_eligible=false`，不代表发币或到账。
同一付款即使改变用户、金额或批次，仍会因占用返回 `PAYMENT_ALREADY_USED`。

## 开发与协作验收

纯合成回归入口：`node scripts/test-esk-platform-reconciliation.js` 和
`node scripts/test-esk-paid-reconciliation.js`。跨语言测试使用真实 Store 产生合成快照，
经标准输入运行实际 CLI；HTTP 测试另覆盖生产 Router 和本地 TCP。
Rust 测试按仓库 `scripts/validate-rust.ps1` 入口执行，不绕过共享缓存与验证门禁。

本切片没有真实用户付款核验、资产分配、生产政策启用或私人管理员读取证据。
后续需要受保护运营入口、外部历史覆盖、真实用户映射和逐笔核验，再按正式账本流程
准备及审批。子 APK 正式签名发布、本人资产验收和 Sui 上链各自验收，不由此报告替代。
