---
title: 分布式算力管理 MCP 验收
status: current
reviewed_at: 2026-08-11
owners: ai-economy, backend, security
implementation_status: implementation_partially_verified
---

# 分布式算力管理 MCP 验收

## 1. 本次闭环

本次没有新增第二套 V5、onboarding、Adapter release 或价格曲线模型，而是把已经验收的 v221、v222、v223 Service 接入开放商业 MCP，使 AI 代理可以在既有权限、摘要、幂等和四眼门卫下操作管理面。

MCP 认证现在同时保留：

- 项目角色：继续供开放商业、ERP 和项目内工具授权；
- 平台角色：只用于判断全局 `admin/owner` 管理工具是否可发现、可调用；
- 当前用户与 App：继续由认证会话派生，工具参数不能伪造操作者。

普通用户的 `tools/list` 不返回 `compute_admin_*` 工具。即使绕过目录直接发送工具名，服务端也会先校验平台角色，再解析业务参数。

## 2. 工具范围

### 商户自助 v221

- `compute_submit_my_external_pool_onboarding`；
- `compute_list_my_external_pool_onboarding_requests`；
- `compute_get_my_external_pool_onboarding_request`；
- `compute_cancel_my_external_pool_onboarding_request`；
- `compute_preflight_my_external_pool_onboarding_request`。

### 平台治理 v221

- `compute_admin_list_external_pool_onboarding_requests`；
- `compute_admin_get_external_pool_onboarding_request`；
- `compute_admin_preflight_external_pool_onboarding_request`；
- `compute_admin_review_external_pool_onboarding_request`；
- `compute_admin_apply_external_pool_onboarding_request`。

### 平台治理 v222

- `compute_admin_list_external_pool_adapter_releases`；
- `compute_admin_get_external_pool_adapter_release`；
- `compute_admin_preflight_external_pool_adapter_release`；
- `compute_admin_submit_external_pool_adapter_release`；
- `compute_admin_review_external_pool_adapter_release`；
- `compute_admin_stage_external_pool_adapter_release`。

### 平台治理 v223

- `compute_admin_list_platform_reference_price_curves`；
- `compute_admin_get_platform_reference_price_curve`；
- `compute_admin_preflight_platform_reference_price_curve`；
- `compute_admin_submit_platform_reference_price_curve`；
- `compute_admin_review_platform_reference_price_curve`；
- `compute_admin_apply_platform_reference_price_curve`。

共 22 个工具。全部工具复用原 Service；MCP 层只负责工具契约、身份传递、参数解码和安全响应，不复制 Store 状态机。

## 3. 已验证行为

- 普通用户目录包含 5 个本人 onboarding 工具，不包含任何平台管理工具；
- `admin/owner` 目录在普通目录基础上增加 17 个平台管理工具；
- 普通用户直接调用 v221、v222 或 v223 管理工具时，在参数解码前失败关闭；
- owner 通过 MCP 提交 onboarding，独立管理员复核并由另一管理员 application；
- application 只生成 `external_pool/registering/self_declared` Provider，效果固定为 `provider_registered_only`；
- 管理员通过 MCP 完成 Adapter submit、独立 review 和 stage，效果固定为 `staged_admission_only`；
- 开放商业 MCP 能按平台角色路由 v223 列表工具并返回结构化内容；
- 原有不带平台角色的内部 `call_tool` 保持普通用户语义，现有调用者无需迁移；
- 写工具仍要求显式确认，所有请求继续执行原有摘要、幂等和操作者隔离规则。

## 4. 验证证据

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-management-mcp -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_mcp::management_mcp_tests -- --nocapture
```

- 结果：`CARGO_OK`；
- 测试：4 项通过；
- validation fingerprint：`93211d71a0274645d0987a482a52b4abdb2711bcd4340dd1945fd5090dde4c40`；
- validation receipt：`2dc5f04783a34ac6ea7a6d388b215a77b46833796dbdf52a592d844db33c9aca`。

## 5. 未完成边界

- 未启动真实 TCP 服务，未使用生产会话、生产数据库或 PC 页面操作；
- 未提供 PC 管理页面，也未部署或发布；
- v221 application 与 v222 staged 都只是受治理声明，不能构造 v213 route authority；
- 未解析、下载、重算或验签 Adapter artifact；
- 未验证 credential custody、verifier registry、协议 conformance 或 service actor；
- 未连接外部矿池 worker、ACK/event、Runner、真实派发、实际用量或付款；
- v223 仍是 `fallback_curve/sample_count=0`，不代表真实市场价格、指数、成交或订单簿。

下一步不能继续重复 DTO 或管理入口，应分别进入可信 artifact/verifier/credential producer、PC 管理体验、生产数据库副本升级和真实网络执行器。
