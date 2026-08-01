# 开放商业调用配额 V1 验收

验收日期：2026-08-01

## 已验证闭环

- 商户编辑者可按能力和指定 App/全部 App 创建或更新固定时间窗配额。
- 指定 App 的外部首次调用成功，超过上限的新调用返回类型化限流错误。
- 相同幂等键重放原成功结果，不重复占用配额。
- 超限调用保存为 `failed`、`error_code=rate_limited`，单位与金额均为 0。
- 超限事件写入审计，且不保存原始调用值。
- 停用策略后外部调用恢复；项目编辑者在本项目内调试不占额度。
- 项目总览和 PC 商户工作台展示策略、当前时间窗用量和近期拒绝数。
- 商户项目 AI 可通过 MCP 创建、更新、停用和重新启用同一份配额策略。
- HTTP 商户入口和开发者 Token 入口均把类型化限流错误映射为 `429`。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce-rate-limit -- test `
  --manifest-path server\Cargo.toml open_commerce_rate_limit

Set-Location pc-frontend
npm run build
npm run test:open-commerce
```

Rust 目标测试、全部开放商业回归、完整 Rust `check`、PC 生产构建、开放商业 PC 契约、源码大小和文档模块化门禁均已通过。

## 尚未覆盖

- 全网 IP、设备、机器人信誉和动态反滥用模型。
- 多数据库或多地域部署下的共享限流计数。
- 生产应用审核、开发者信用分级和自动封禁申诉流程。
- 真实收费、退款、争议处理或链上结算。
- 美团、抖音、京东、淘宝闪购等真实生产适配器。

因此，本批可描述为“商户可控的持久化调用配额 V1 已实现”，不能描述为“公共商业网络生产风控已完成”。
