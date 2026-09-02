---
title: "ESK Paper 首批用户批量登记运营闭环 V1"
status: accepted
owner: platform-assets
priority: p0
reviewed_at: 2026-09-02
implementation_refs:
  - "route:/api/admin/assets/esk/paper-allocation-batches"
  - "file:server/src/esk_asset/"
  - "file:server/src/store/common/esk_asset_batches.rs"
  - "file:scripts/esk-paper-allocation-batch.ps1"
---

# ESK Paper 首批用户批量登记运营闭环 V1

## 用户结果

运营人员能够把已经确认、并已映射到主项目内部用户 ID 的首批 ESK Paper 登记清单先完整预检，再原子提交。成功后，每名用户从主项目 ESK 账户和量化子项目只读投影看到同一笔余额事实；重复提交同一批次不会重复增加余额。

本功能只解决项目内部 Paper ESK 的安全登记和可核对回执，不证明链上发币、真实资产托管、量化入金、投资收益或官方卖回付款。

## 输入边界

- 请求只接受稳定批次 ID，以及 1 至 100 条 `user_id`、精确十进制 `amount`、非敏感 `reference` 和全局 `idempotency_key`。
- 输入不得包含姓名、邮箱、电话号码、付款截图、银行卡、钱包私钥、KYC 文件或聊天原文。运营人员应在请求外完成付款与内部用户 ID 的映射。
- 金额继续使用六位小数精度和整数 base units，不接受浮点数、负数、指数格式或超范围值。
- 同一用户可以在一批中拥有多笔独立登记；同一批内重复的 `reference` 或 `idempotency_key` 必须失败。
- 未知字段、空批次、超过 100 条、用户不存在或任一条无效时，整批失败。

## 两阶段操作

同一个版本化入口支持 `dry_run` 与 `commit`：

1. `dry_run` 执行与提交相同的字段、用户和冲突检查，但不写任何账本或批次记录；返回确定的 SHA-256 请求摘要、总笔数和总 ESK。
2. `commit` 必须携带 dry-run 返回的 `expected_request_digest` 和固定确认文本 `RECORD PAPER ESK BATCH`。服务端重新计算摘要并比对，任何输入漂移都失败关闭。
3. `commit` 在一个数据库事务中写入批次回执、全部 ESK 追加式账本条目和批次条目关联；任何一条失败时全部回滚。

`ESK_ASSET_MODE` 不是 `paper`、管理员令牌无效或确认文本不匹配时，不得预检或提交。

## 幂等与审计

- `batch_id` 是批次级幂等边界。相同批次 ID 与相同请求摘要再次提交时返回原回执并标记 `replayed=true`，余额不增加。
- 相同批次 ID 携带不同内容必须冲突失败。
- 首次提交时，任一条 `idempotency_key` 已被单笔接口或其他批次使用，整批冲突失败，不把部分旧记录伪装成本批成功。
- 批次和批次条目表均为追加式，数据库触发器禁止更新与删除；批次条目必须关联真实账本条目。
- 回执固定标记 `simulated=true`、`funds_moved=false`，并返回批次 ID、摘要、精确总额、条目结果和创建时间，供运营留存。

## 运营工具

仓库提供只从本地 JSON 文件读取清单的 PowerShell 工具：

- 管理员令牌只从指定环境变量读取，不作为命令行参数或日志输出；
- 默认只 dry-run；提交模式必须显式提供 dry-run 摘要；
- 工具先重新 dry-run，比对摘要后才发送 commit；
- 可选写入本地回执文件，但不得把输入清单、令牌或完整 HTTP 请求打印到终端。

工具不负责创建用户、解析付款截图、换算购买价格、批准卖回、转账、下单或链上铸币。

## API 结果

`POST /api/admin/assets/esk/paper-allocation-batches` 返回 `yilong.esk.paper_allocation_batch_receipt.v1`。dry-run 状态为 `validated`，没有条目 ID 和持久化时间；commit 状态为 `committed`，每条带真实账本条目 ID。所有金额同时提供固定六位十进制字符串和 base units 字符串。

错误必须落在以下可操作边界：鉴权失败为未授权，输入或摘要/确认不匹配为错误请求，批次或条目幂等漂移为冲突，用户不存在为未找到，存储异常不回显内部细节。

## 验收标准

1. dry-run 对合法清单返回确定摘要和正确总额，数据库账本及批次表均无新增。
2. commit 只有在摘要和固定确认文本匹配时原子写入；提交后各用户余额与条目合计准确。
3. 完整重放返回相同条目 ID 且不增加余额；批次内容漂移、已占用条目幂等键或批内重复键均整批失败。
4. 任一用户不存在或任一金额无效时，其他合法条目也不入账。
5. API、数据库迁移和运营工具有自动化测试或可重复验收证据；现有单笔登记、用户余额、卖回申请和量化只读投影保持兼容。
6. 生产发布只启用 Paper 登记能力；不启用链上发行、真实资金、收益承诺、卖回结算或交易执行。
