# 开放商业 Grant 限时授权 V1 验收

## 已验证闭环

- 商户直接创建 Grant 和批准第三方 App 申请时均可选择 7/30/90/365 天或长期有效。
- PC 新授权默认 30 天；长期授权必须显式选择，API 未提供期限时仍保持向后兼容。
- 授权审批把 `expires_at`、次数上限、金额上限和币种传入统一 Grant 创建流程。
- 批准后的商户收件箱和申请方收件箱从实际 Grant 回读相同期限与预算条件。
- 服务端拒绝已过期 Grant；消费者发现不再把它识别为有效授权。
- PC 标记过期 Grant，并从能力调用选择器中排除，不删除历史记录。
- 批准审计包含期限和预算边界，不包含 Token、调用输入或处理结果。
- 没有自动续期、自动延期或因 UI 状态绕过服务端校验的路径。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce-authorization-expiry -- test `
  --manifest-path server\Cargo.toml open_commerce_authorization_expiry -- --nocapture

Set-Location pc-frontend
npm run build
npm run test:open-commerce
```

## 尚未覆盖

- 到期前通知、人工续期向导、批量续期和宽限期。
- 生产开发者身份审核和跨运营方授权互认。
- 多数据库部署下的统一时间源和时钟偏差监控。
- 真实资金、退款、链上授权对象和争议处理。

因此，本批可描述为“商户可设置并向申请方披露的 Grant 限时授权 V1 已实现”，不能描述为“生产身份与授权治理已经完成”。
