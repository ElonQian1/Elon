# 开放商业授权生命周期预算 V1 验收

验收日期：2026-08-01

## 已验证闭环

- 商户直接创建 Grant 或批准授权申请时，可选设置总调用次数和总预算；留空保持原有无限额行为。
- 新调用在处理器前原子预留次数和当前能力单价，成功后确认预留。
- 处理器失败时，调用保存为零金额失败，预留次数和金额原子退回。
- 相同幂等键重放原结果，不重复消耗 Grant 预算。
- 次数或金额达到上限后，新调用不进入处理器，保存为 `failed/grant_budget_exceeded` 并返回类型化 `403`。
- Grant 返回已用次数和金额；PC 商户工作台展示已用/上限，审批入口可设置同一预算。
- MCP 创建 Grant 与 HTTP 使用同一预算字段和领域规则。
- 预算事件审计不包含原始调用值。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce-grant-budget -- test `
  --manifest-path server\Cargo.toml open_commerce_grant_budget

Set-Location pc-frontend
npm run build
npm run test:open-commerce
```

## 尚未覆盖

- 进程在预留后崩溃时的自动超时对账与人工释放工具。
- 跨数据库、跨地域部署下的分布式原子预算。
- 真实支付、退款、争议裁决、链上结算或收入权益。
- 商户对现有 Grant 原地增加预算；V1 必须撤销并重新授权。

因此，本批可描述为“商户可控的 Grant 生命周期调用与计量预算 V1 已实现”，不能描述为“真实资金托管或链上限额已完成”。
