# ESK Paper 首批用户批量登记运营闭环 V1 验收

状态：实现、假数据验收和服务端发布已完成；仍为 Paper-only，不涉及链上发行或真实资金。

对应需求：[`requirements/esk-paper-first-user-allocation-operations-v1.md`](requirements/esk-paper-first-user-allocation-operations-v1.md)

## 已实现能力

- 新增管理员入口 `POST /api/admin/assets/esk/paper-allocation-batches`，请求和响应均为显式 V1 合同。
- `dry_run` 与 `commit` 使用同一套精确金额、用户存在性、批内重复和全局幂等冲突校验；dry-run 不写数据库。
- commit 必须携带 dry-run 返回的 SHA-256 摘要和固定确认文本，输入变化时失败关闭。
- 版本 `282` 增加追加式批次与条目关联表。批次、关联和已有 ESK 账本均由触发器禁止更新与删除。
- commit 在一个 SQLite `IMMEDIATE` 事务中写批次、全部账本条目和关联；任一用户或条目失败时整批回滚。
- 同一批次 ID 和相同摘要重放返回原条目 ID，`replayed=true` 且不重复增加余额；内容漂移返回冲突。
- PowerShell 运营工具默认只预检，只从环境变量读取管理员令牌；提交前会重新 dry-run 并核对操作员提供的摘要。

## 运营入口

输入模板：[`examples/esk-paper-allocation-batch.example.json`](examples/esk-paper-allocation-batch.example.json)

令牌由安全运营环境注入，不能写进 JSON、命令历史、聊天或 Git：

```powershell
$env:ELON_ADMIN_TOKEN = '<从安全运营环境取得>'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\esk-paper-allocation-batch.ps1 `
  -InputPath .\first-users.json `
  -BaseUrl https://<一龙主项目域名> `
  -ReceiptPath .\receipts\first-users.dry-run.json
```

人工核对 dry-run 的 `batch_id`、`entry_count`、`total` 和 `request_digest` 后，才显式提交：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\esk-paper-allocation-batch.ps1 `
  -InputPath .\first-users.json `
  -BaseUrl https://<一龙主项目域名> `
  -Commit `
  -ExpectedRequestDigest <dry-run 返回的 64 位摘要> `
  -ReceiptPath .\receipts\first-users.commit.json
```

每条输入只放主项目内部 `user_id`、ESK 数量、非敏感内部订单引用和唯一幂等键。不要放姓名、邮箱、电话、付款截图、银行卡、钱包私钥、KYC 文件或聊天正文。

## 自动化和运行证据

2026-09-02 在隔离 worktree 完成：

- `elon-server` 生产二进制 `cargo check --bin elon-server` 与 `cargo build --bin elon-server` 通过。
- ESK batch 测试二进制成功编译，四组测试全部通过：摘要规范化和重复输入、dry-run 零写入与追加保护、两用户原子提交/重放/漂移、缺失用户和已占用条目幂等键整批回滚，以及管理员 HTTP 鉴权/摘要/确认/回执合同。
- 新旧 ESK 模块合并回归共 `10 passed / 0 failed`，继续覆盖原有单笔登记、账户隔离、卖回占用/取消和并发超额保护。
- PowerShell 脚本语法和示例 JSON 解析通过。
- 临时真实 HTTP 服务、全新数据库和两名假用户端到端通过：dry-run 后 Alice 仍为 `0.000000 ESK / revision 0`；commit 后 Alice 为 `12.500000 / revision 1`、Bob 为 `3.250000 / revision 1`；相同 commit 重放返回 `replayed=true`，两人修订号保持 `1`。
- 端到端 dry-run 与 commit 的合计均为 `15.750000 ESK`、摘要一致；用户账户继续返回 `simulated=true`、`funds_moved=false`。临时服务、数据库和回执已在验收后删除。
- 服务端已发布为 `v0.3.1714`；线上健康与版本检查返回实现提交 `0be0a348ddb37220c7c31d0827873d714c8beb00`，`CODE_SYNC_STATUS=synced`、`SERVER_RELEASE_STATUS=published`。本批未在生产接口创建假用户或写入假 ESK。

## 兼容与边界

- 现有单笔管理员登记、用户 ESK 总额/可用额/卖回占用、卖回申请与量化只读投影没有改名或迁移；批量入口只向同一主账本追加条目。
- 同一用户可在一批中有多笔不同订单；重复引用和重复幂等键被拒绝。
- 完整 workspace `cargo check` 仍会被基线中 `elon-pc-node` 的 SQLite VFS 测试/条件编译错误阻断；本批命中的 `elon-server` 生产目标、测试目标和实际 HTTP 路径均已独立通过。
- 本功能没有价格换算、付款确认、用户创建、链上铸币、钱包托管、量化下单、收益计算、卖回批准或付款。真实客户清单仍需运营人员在仓库外核对，并在生产写操作前另行取得明确授权。
