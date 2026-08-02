# 开放商业消费者可携带数据包 V1 验收

## 已验证闭环

- 当前用户可创建、列出和读取自己的不可变数据包；同项目其他用户不能读取或列出该用户的数据包。
- 负载包含该用户的关系历史、消费者私有续期链和删除请求回执，不包含消费者账号 ID、偏好原文、订单或商户私有数据。
- 同一幂等键在来源数据变化后仍返回原数据包、原负载和原摘要；新键生成反映新状态的新快照。
- 每类来源记录最多 5000 条，序列化负载最多 5 MiB，超限失败关闭且不会静默截断。
- 服务端在创建、列表、读取和幂等重放时校验版本、来源项目和 SHA-256；数据库摘要被修改后读取失败。
- HTTP 和 MCP 共用同一领域服务；MCP 已覆盖创建、列表和详情读取。
- PC 可生成或下载历史数据包，并在浏览器重新计算负载 SHA-256；摘要不一致时停止下载。
- 商户现有关系接口和界面仍不依赖消费者用户 ID、消费者项目 ID 或内部续期字段。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -WaitTimeoutSeconds 1200 -- test --manifest-path server\Cargo.toml `
  open_commerce_portability -- --nocapture

node scripts\test-open-commerce-pc-workspace.js

Set-Location pc-frontend
npm run lint
npm run build
```

## 尚未覆盖

- 跨一龙实例或其他运营方的数据包导入、字段映射、冲突处理和迁移回滚。
- 偏好原文、联系方式、订单、支付、完整账户数据或商户外部 CRM 数据的导出。
- 用户公钥签名、加密归档、第三方托管证明、链上存证和法律合规认证。
- 自动定期导出、增量包、过期清理、外部通知和云盘同步。

因此，本批只能描述为“消费者关系与删除请求的可验证导出 V1 已实现”，不能描述为“完整消费者数据保险箱或跨平台迁移已经完成”。
