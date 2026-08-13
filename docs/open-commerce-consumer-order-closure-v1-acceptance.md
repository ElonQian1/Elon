---
version_status: current
status: verified
owner: open-commerce
updated: 2026-08-13
---

# 消费者订单闭环视图 V1 验收

## 本批完成

- 新增只读 HTTP：`GET /api/open-commerce/consumer-order-closures/:invocation_id`。
- 新增只读 MCP：`open_commerce_get_my_order_closure`。
- 两个入口共用同一服务，只聚合当前消费者拥有的终态 Invocation、商户标准订单回执与最新 ERP 衔接回执。
- 派生 `merchant_confirmed_erp_pending`、`erp_recorded`、`erp_retry_required`、`erp_ignored` 四种解释状态，不创建第二套订单状态机。
- ERP 投影只返回目标记录 SHA-256 和最小结果字段，不返回项目、Integration、接入器凭据、Claim、租约密钥、内部用户、请求哈希或原始 ERP 记录号。
- 商户订单金额保持 `amount_minor`，平台调用计量保持 `amount_micros`；所有层级 `funds_moved=false`。

## 验证证据

执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain server -- test --manifest-path server/Cargo.toml open_commerce_consumer_order_closure -- --nocapture
```

结果：

- Rust server 二进制完成真实编译。
- 定向 Rust/SQLite：`4 passed; 0 failed`。
- 覆盖消费者隔离、非终态/失败/缺失/非订单/无效回执失败关闭、四种闭环状态、内部字段脱敏及 MCP 与领域服务同投影。
- 验证指纹：`88e72aa1655816c88b59491f6dbe167792492c085238842160871f79ceb7e18f`。
- 验证回执：`d1660d2ba88302a486ec835ec333e4a80164a9791715f0c877d1e92f25ee8b48`。

## 明确边界

- 本批没有新增订单表、支付表或 ERP 状态机。
- 商户订单回执仍是商户运行时声明，ERP 衔接结果仍是目标接入器声明。
- 本批不证明真实支付、配送、履约、退款、生产 ERP 写入、外部平台授权、公网 TLS 或链上结算。
- HTTP 路由随 server 编译接线；本批没有再启动独立真实 TCP 服务做重复验收。
