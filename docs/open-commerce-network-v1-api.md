---
title: AI 原生开放商业网络 V1 API 与 MCP 契约
owner: backend
reviewed_at: 2026-07-28
status: accepted
source: docs/decisions/open-commerce-network-v1-architecture.md
---

# AI 原生开放商业网络 V1 API 与 MCP 契约

## 协议标识

- HTTP schema：`open_commerce.v1`
- MCP server：`yilong-open-commerce`
- MCP protocol：`2025-03-26`
- 金额：整数微单位，禁止浮点金额
- 时间：UTC RFC 3339
- ID：服务端生成的带前缀不透明字符串

## 项目管理 API

所有项目管理接口都需要一龙 Bearer token，并校验项目成员关系。

| 方法 | 路径 | 用途 |
|---|---|---|
| `GET` | `/api/projects/:project_id/open-commerce/overview` | 节点、能力、授权、调用和计量概览 |
| `POST` | `/api/projects/:project_id/open-commerce/merchants` | 创建商户节点 |
| `PATCH` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id` | 更新或停用商户节点 |
| `POST` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/capabilities` | 创建商业能力 |
| `PATCH` | `/api/projects/:project_id/open-commerce/capabilities/:capability_id` | 更新或停用能力 |
| `POST` | `/api/projects/:project_id/open-commerce/grants` | 创建调用授权 |
| `POST` | `/api/projects/:project_id/open-commerce/grants/:grant_id/revoke` | 撤销授权 |
| `GET` | `/api/projects/:project_id/open-commerce/audit` | 读取项目审计与调用记录 |

## 发现与调用 API

| 方法 | 路径 | 用途 |
|---|---|---|
| `GET` | `/api/open-commerce/merchants` | 按文本和能力发现启用的商户 |
| `GET` | `/api/open-commerce/merchants/:merchant_id` | 读取商户公开资料和可发现能力 |
| `POST` | `/api/open-commerce/invoke` | 调用能力并记录幂等、计量和审计 |

两个 `GET` 发现接口允许匿名读取，便于任意 App 或 AI 在未加入一龙项目时发现公开商户能力。能力调用、项目管理和 MCP 仍需要 Bearer 身份；开放发现不等于匿名执行。

发现接口不返回授权数据、内部处理器配置、项目成员信息或商户内部数据。

## 调用请求

```json
{
  "merchant_id": "merchant_xxx",
  "capability_key": "store.profile.read",
  "requester_app_id": "pc-web",
  "grant_id": null,
  "idempotency_key": "demo-001",
  "input": {}
}
```

调用结果必须明确区分业务结果和资金状态：

```json
{
  "schema": "open_commerce.invocation.v1",
  "invocation_id": "invoke_xxx",
  "status": "succeeded",
  "result": {},
  "metering": {
    "units": 1,
    "unit_price_micros": 0,
    "amount_micros": 0,
    "currency": "CNY",
    "settlement_status": "recorded_not_charged"
  }
}
```

相同调用方、商户、能力和幂等键的重复调用返回原调用结果或稳定的重复结果，不重复累计金额。

## MCP 工具

| 工具 | 读写 | 用途 |
|---|---|---|
| `open_commerce_get_overview` | 读 | 查看当前项目网络状态 |
| `open_commerce_search_merchants` | 读 | 按文本或能力发现商户 |
| `open_commerce_get_merchant` | 读 | 获取单个商户与公开能力 |
| `open_commerce_create_merchant` | 写 | 创建商户节点 |
| `open_commerce_publish_capability` | 写 | 发布受控能力 |
| `open_commerce_create_grant` | 写 | 为 App 创建最小范围授权 |
| `open_commerce_revoke_grant` | 写 | 撤销授权 |
| `open_commerce_invoke` | 写 | 调用能力并生成计量和审计 |
| `open_commerce_list_audit` | 读 | 查看调用与治理证据 |

MCP 写工具遵循与 HTTP API 相同的项目角色、授权和幂等规则。MCP 不提供绕过确认的真实资金、发布或外部系统写操作。

## 第一方处理器

V1 允许：

- `merchant_profile`：返回商户公开资料；
- `static_json`：返回项目编辑者配置的静态演示数据。

V1 拒绝未知处理器和任意 URL。后续真实连接器必须单独注册、审核、限定主机、管理密钥并声明超时与失败语义。

## 错误语义

| HTTP | 含义 |
|---|---|
| `400` | 输入、schema、幂等键或状态不合法 |
| `401` | 未登录或 token 无效 |
| `403` | 不是项目成员、角色不足或授权不匹配 |
| `404` | 商户、能力、授权或项目不存在 |
| `409` | slug、能力键、幂等键语义冲突 |
| `422` | 能力存在但当前输入不满足契约 |

错误响应不得包含 token、密钥、内部处理器配置或原始敏感输入。
