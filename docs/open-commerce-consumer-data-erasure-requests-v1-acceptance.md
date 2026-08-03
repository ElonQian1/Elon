# 开放商业消费者关联数据删除请求 V1 验收

## 已验证闭环

- 消费者只能对本人关系创建删除请求，创建与关系撤销在同一数据库事务内完成。
- 同一关系最多一个未完成请求；重复创建、重复撤回和重复同向商户决定保持幂等。
- 同项目其他成员不能读取或撤回该用户的请求。
- 商户只能读取指向本商户的匿名请求；响应与 PC 界面均不依赖消费者用户 ID 或消费者项目 ID。
- 商户查看不要求编辑权，但接单、完成和拒绝要求商户项目编辑权。
- 完成与拒绝必须填写处理说明；消费者只能在商户接单前撤回，撤回不会恢复关系。
- `completed` 保存为 `merchant_attested_completed`，PC 两端均明确该状态不是平台验证的外部删除证明。
- HTTP、MCP 与 PC 共用同一状态机；MCP 已验证创建、读取与撤回真实调用闭环。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce -- test `
  --manifest-path server\Cargo.toml open_commerce_data_request -- --nocapture

Set-Location pc-frontend
npm run build
npm run test:open-commerce
```

## 尚未覆盖

- 消费者偏好数据保险箱、字段级授权、导出和可移植数据包。
- 美团、抖音、ERP、CRM 或会员系统的真实删除适配器与回执核验。
- 法定期限判断、自动通知、平台工单、争议处理、处罚与赔付。有限手动催办与升级关注代码已形成但尚未编译，后置验证见 `docs/open-commerce-consumer-data-request-followups-v1-acceptance.md`。
- 跨运营方身份、工单迁移、密码学删除证明和公共信誉。
- 真实支付、链上请求对象和赔付。

因此，本批只能描述为“消费者关联数据删除请求与商户履约声明 V1 已实现”，不能描述为“消费者数据保险箱或外部数据自动删除已经完成”。
