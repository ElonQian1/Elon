# 开放商业消费者关系安全续期 V1 验收

## 已验证闭环

- 消费者可对本人关系续期；新关系继承商户、范围和用途，并使用重新选择的有效期。
- 旧关系与同商户其他有效关系在同一事务内撤销，新关系生成不同的匿名标识。
- 同一来源关系的重复续期返回同一后继 ID 和匿名标识；每次继续续期必须引用当前后继。
- 商户撤回目录发布后，尚未发生的首次续期失败关闭；已成功续期的重试仍返回原结果。
- 同项目其他用户不能续期；PC 不能冒充 `mcp-client`，MCP 来源身份由入口绑定。
- 公开关系模型和商户读取不返回 `renewed_from_relationship_id`、消费者用户 ID 或消费者项目 ID。
- PC 使用明确的 14 天临期窗口，只为同一商户的最新关系提供续期，并说明旧凭证撤销和匿名标识轮换。
- HTTP、MCP 与 PC 共用同一领域服务；新审计只写入消费者项目。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce -- test --manifest-path server\Cargo.toml `
  open_commerce_relationship_renewal_tests -- --nocapture

Set-Location pc-frontend
npm run build
npm run test:open-commerce
```

## 尚未覆盖

- 短信、邮件、移动系统通知和后台自动提醒。
- 商户 CRM、会员、订单或偏好数据按新匿名标识自动迁移。
- 跨运营方关系迁移、联邦身份和公共信誉。
- 真实支付、链上关系对象、纠纷和赔付。

因此，本批只能描述为“消费者匿名关系安全续期与 PC 临期提醒 V1 已实现”，不能描述为“消费者数据或会员关系已自动迁移”。
