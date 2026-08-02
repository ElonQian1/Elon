# 开放商业消费者可携带数据包 V2 验收

## 已验证闭环

- 新快照使用 V2 版本，并包含当前用户低敏结构化偏好档案和历史披露快照。
- 关系、续期链、删除请求、偏好档案和披露在同一个数据库读事务中读取；披露与其他列表一样有 5000 条硬上限。
- 数据包及 PC 界面不依赖或返回消费者用户 ID、消费者项目 ID、联系方式、订单或支付。
- 同一幂等键在偏好更新后仍返回原快照；新幂等键才反映新的档案修订。
- 档案更新不会暗中重写旧披露，数据包可区分当前档案和历史披露的修订号。
- 历史 V1 JSON 在 V2 代码中读取后，重新序列化字节与原 SHA-256 保持一致；V1/V2 版本混用失败关闭。
- HTTP、MCP 和 PC 继续共用同一领域服务；PC 显示偏好档案和披露计数，并在下载前重新计算 SHA-256。

## 仍未完成

- 订单、支付、联系方式、自由文本、敏感身份信息和商户外部 CRM 数据导出。
- 跨运营方导入、字段映射、冲突解决、迁移回滚和账户恢复。
- 加密归档、用户公钥签名、第三方托管证明、链上存证和法律认证。
- 自动定期导出、增量包、外部通知和云盘同步。

## 验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain open-commerce-portability-v2 -- test --manifest-path server\Cargo.toml open_commerce_portability --no-fail-fast
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
```
