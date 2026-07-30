---
title: 开放商业数据接入控制面验收
owner: backend
reviewed_at: 2026-07-30
status: verified
source: docs/decisions/open-commerce-integration-control-plane.md
---

# 开放商业数据接入控制面验收

## 已验收范围

- 项目编辑者可以为项目内商户登记数据来源。
- 接入方式、授权范围和数据域使用有界枚举或标识符。
- 接入记录不接收访问令牌、Cookie 或任意外部 URL。
- 同步回执使用接入级幂等键；同键不同结果会被拒绝。
- 成功、部分成功和失败回执分别驱动连接健康状态。
- HTTP、MCP 和 PC 工作台读取同一领域事实。
- AI 开发上下文不返回处理器配置和原始经营数据。
- 所有创建、停用和同步回执事件进入开放商业审计。

## 验证命令

```powershell
$env:CARGO_TARGET_DIR='D:\rust\shared\target'
cargo test --manifest-path server/Cargo.toml open_commerce --bin elon-server
Set-Location pc-frontend
npm run typecheck
```

2026-07-30 验收结果：后端开放商业相关测试 11 项通过，PC TypeScript 类型检查通过。

## 未验收范围

- 美团、抖音、京东、淘宝闪购或微信的生产凭据和官方 API。
- POS、库存、预约或营销发布的具体厂商适配器。
- 原始经营数据存储、字段映射和冲突合并。
- 自动收费、退款、分账和 Sui 链上结算。
- 消费者侧公共发现网络和跨实现身份互认。
