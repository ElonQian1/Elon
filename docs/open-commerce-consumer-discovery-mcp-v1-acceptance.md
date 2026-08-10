# 消费者 AI 完整发现 MCP V1 验收

状态：`verified_local`

## 已验证能力

- 工具清单公开完整消费者发现输入 Schema。
- 调用复用服务端消费者发现、输入规范化、排序、筛选和候选范围逻辑。
- 默认 MCP 身份降级为公开发现身份；显式 App 身份必须启用且属于当前用户和当前项目。
- MCP 返回与直接调用消费者发现领域服务的结果一致，不维护第二套筛选或排序实现。
- 返回数量按 Schema 限制为 1 至 50，未知字段失败关闭，入口参数中的 `requester_app_id` 不能覆盖 MCP 身份。
- MCP 初始化说明明确发现不会自动调用或下单。

## 本地验收

- `default_identity_discovers_public_and_requires_app_for_authorized_without_writes`：覆盖默认身份映射、公开/授权状态、内部用户与项目 ID 不泄漏及无写入。
- `owned_app_mcp_result_matches_the_shared_domain_service_and_reflects_grant`：覆盖本人 App、有效 Grant、价格/能力/类别过滤，并与同输入领域服务完整 JSON 相等。
- `explicit_identity_must_be_active_owned_and_in_the_current_project`：覆盖其他用户 App、同用户跨项目 App、停用 App 和未知 App；跨项目身份对发现、计划和授权申请均失败关闭。
- `filters_receipt_and_entry_identity_override_are_bounded_and_read_only`：覆盖动作/公开/币种/类别硬约束、透明非付费排序凭证、候选范围以及入口身份不可覆盖。
- `schema_and_domain_invalid_inputs_fail_closed_without_writes`：覆盖 0/51 返回数量、未知字段、非法币种和缺少城市偏好的硬约束。
- `definition_exposes_read_only_non_paid_discovery_contract`：覆盖工具清单只读、幂等、开放世界注解及初始化说明。

发现验证命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain open-commerce-consumer-discovery -- test --manifest-path server\Cargo.toml open_commerce_consumer_discovery_mcp::discovery_tests -- --nocapture
```

发现验证指纹：`c4e4975aa3f812cf78875a22f68d1e98e3c1b3b1ee08fec9536b8fe9e0c31c7e`。

授权申请回归指纹：`0cb3f3464d6f68dc0b4ae57cb25974b1d1ee23be138bd98fe94e4a75a0a418e6`。

## 未覆盖边界

本批使用本地 SQLite 假数据并调用真实 MCP `tools/call` 路由，未启动 HTTP 服务或 PC 页面，未配置内部同步回执来源，也未穷举全部来源筛选组合。测试不连接真实外部平台，不证明目录数据真实、全网穷尽、排序公平、真实下单、支付或生产部署。
