---
title: 开放商业数据接入控制面决策
owner: product-platform
reviewed_at: 2026-07-30
status: accepted
source: docs/open-commerce/integration-architecture.md
---

# 开放商业数据接入控制面决策

## 决策

在接入任何美团、抖音、京东、淘宝闪购、微信或收银系统适配器之前，先建立厂商无关的数据接入控制面。控制面只管理连接事实，不保存第三方平台令牌或原始经营数据。

每个接入记录必须绑定项目和商户，并声明：

- 稳定接入键与平台标识；
- `official_api`、`merchant_export`、`local_adapter` 或 `manual_import` 接入方式；
- 最小授权范围和可用数据域；
- `configured`、`connected`、`degraded` 或 `disabled` 健康状态；
- 最近验证和同步时间。

真实适配器运行后提交有幂等键的同步回执。回执只记录同步类型、成功状态、扫描与变更数量、游标摘要、错误代码和时间，不保存订单、客户、库存或财务原始值。

## 为什么先做控制面

大型平台的授权范围、字段和稳定性不同，不能通过一个“已接入”开关掩盖差异。统一控制面使 AI、PC 工作台和后续适配器共享同一组事实：

1. 哪个商户通过什么方式连接了哪个数据来源；
2. 当前允许读取或执行什么；
3. 哪些数据域可用于应用开发；
4. 最近一次同步是否有可审计证据；
5. 连接异常时哪些自动化任务必须停止。

## AI 开发上下文

开放商业 MCP 和 HTTP 提供项目级开发上下文。它可以被现有 Matter/Assignment 开发链路读取，包含商户、能力契约、数据接入状态和有界同步证据，但排除：

- 第三方平台访问令牌和密钥；
- 商业能力处理器私有配置；
- 原始订单、客户、财务与库存值；
- 对尚未获得的平台权限作出的推断。

这使开发代理能够区分“代码可以实现”“数据已经可用”和“仍需平台授权或适配器”。

## 边界

- 登记 `provider_key=meituan` 不代表美团已开放或授权相关 API。
- 商户项目编辑者可以登记和停用接入，但只有真实适配器结果才应提交成功回执。
- 控制面不调用任意外部 URL，不承担密钥保险箱职责。
- 每个平台适配器必须单独审核主机、认证、限流、字段、失败语义和写操作确认。
- 多平台聚合仍属于部分实现，只有通过回执验收的适配器才可升级为“已接通”。

## 实现证据

- 模型：`server/src/open_commerce_integration_model.rs`
- 数据库：`server/src/open_commerce_integration_migration.rs`
- 存储与幂等：`server/src/store/open_commerce_integrations.rs`
- HTTP：`server/src/open_commerce_api.rs`
- MCP：`server/src/open_commerce_mcp.rs`
- PC 管理：`pc-frontend/src/features/open-commerce/OpenCommerceIntegrationManager.tsx`
- 端到端验收：`server/src/open_commerce_service_tests.rs`
