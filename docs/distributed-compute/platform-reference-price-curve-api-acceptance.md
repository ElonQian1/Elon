---
title: 平台参考价格回退曲线管理面验收
status: current
reviewed_at: 2026-08-11
owners: ai-economy, backend, security
implementation_status: implementation_partially_verified
---

# 平台参考价格回退曲线管理面验收

## 1. 本次闭环

本次把既有 v223 领域与五账本骨架接成可认证的管理员控制面，没有创建第二套 Price Snapshot 或市场模型：

- `GET /api/admin/compute/platform-reference-price-curves`：按受限状态和数量读取批次；
- `POST /api/admin/compute/platform-reference-price-curves`：提交 exact Offer/窗口/价格批次；
- `GET /api/admin/compute/platform-reference-price-curves/:batch_id`：读取 batch、entries、review、application 与 bindings；
- `GET /api/admin/compute/platform-reference-price-curves/:batch_id/preflight`：按当前管理员身份返回复核或应用门卫；
- `POST /api/admin/compute/platform-reference-price-curves/:batch_id/review`：由不同管理员批准、退回或拒绝；
- `POST /api/admin/compute/platform-reference-price-curves/:batch_id/application`：消费 exact approval，并在同一事务登记 v171 Snapshot。

全部写入口从登录会话派生操作人 ID，只允许 `admin/owner`，拒绝请求体未知字段和客户端注入操作人身份。列表和详情返回审计回执，不返回认证令牌、私钥或节点路由。

## 2. 修复的运行时问题

既有未运行源码有两个会阻断真实生命周期的问题：

1. submit 在服务器生成 `submitted_at` 前用空字符串执行完整形状校验，导致所有真实提交失败；现使用不晚于 `valid_from` 的占位值校验不含服务器时间的 material digest，再校验实际服务器时间。
2. v223 TTL trigger 使用 `julianday` 浮点差，精确 300 秒可能被 SQLite 计算为 `300.000022...` 并误拒绝；v224 用整数 Unix 秒差替换旧触发器，同时保留纳秒 ISO 时间的严格先后和有效期约束。

v224 是追加迁移。验收会模拟已存在的旧触发器、回退迁移版本、重新打开文件 Store，并确认触发器被替换、迁移记录唯一且第二次重开幂等。

## 3. 已验证行为

定向测试覆盖 2 个 Store 生命周期、1 个进程内 Axum 管理流程和 1 个旧库升级流程：

- exact submit 幂等重放，提交人不能自审，错误 review digest 不能应用；
- approved batch 原子生成 `fallback_curve`、`sample_count=0`、`trade_id=None` 的唯一 v171 Snapshot；
- application exact 重放，详情和状态过滤列表在文件 Store 重开后保持一致；
- `changes_requested` 不能产生 application、binding 或 Snapshot，未知状态过滤失败关闭；
- 未登录返回 `401`，非管理员返回 `403`，客户端操作人字段返回 `422`；
- preflight 对提交人与独立复核人给出不同门卫，approve/application 全链成功；
- v224 可从模拟 v223 旧触发器原位升级，并在再次重开时保持幂等。

验证命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-platform-reference-price-curve -- test --manifest-path server/Cargo.toml --bin elon-server platform_reference_price_curve -- --nocapture
```

最终验证指纹：`1a23c7a39373d30c0597aca7d439d2f6fd121bd445727cb2a58aaf025fc75fd1`。验证回执 profile：`732e88238b2f3c94c16c6c9e75ef129e948027907d48b6e81e588c795141f923`。

## 4. 明确未完成

- 未验证真实 TCP、浏览器、并发压力、异常断电或生产数据库副本升级；
- 未提供 MCP 或 PC 管理入口，未部署到生产；
- 未接入真实外部价格样本、多源校验、index、mark、trade、订单簿或自动撮合；
- application 不创建 Job、不预留容量、不冻结或移动资金，也不派发 Attempt。

这些缺口不能通过重复实现 v223 DTO 或五账本解决，后续应分别进入真实价格 producer、MCP/PC 管理面和生产验证任务。
