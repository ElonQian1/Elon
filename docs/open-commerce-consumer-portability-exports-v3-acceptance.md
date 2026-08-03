# 开放商业消费者可携带数据包 V3 验收

## 已验证闭环

- 新快照使用 V3 版本，并包含当前登录用户账户级终态调用凭证；调用来源不受创建数据包的消费者项目限制。
- 关系、续期链、删除请求、偏好、披露和调用记录在同一个 SQLite 读事务中读取，调用记录同样受 5000 条硬上限约束。
- 每条凭证和总包都返回服务端规范 `payload_json` 与独立 SHA-256；服务端先验证规范表示和内层凭证再验证总包，PC 对原字符串重复两级校验，避免任意结果键顺序造成误报。
- 凭证可包含本人已经收到的商户返回结果，但不包含原始输入、请求摘要、Grant、调用幂等键、内部能力 ID 或消费者账号 ID。
- 仅纳入 `succeeded` 和 `failed` 终态且结算状态为 `recorded_not_charged` 的调用；进行中或未知结算状态失败关闭。
- 相同导出幂等键不会吸收后续调用；使用新键才生成包含新调用的新快照。
- 历史 V1 和 V2 JSON 重新序列化后保持原字节与摘要，V1/V2/V3 混合版本失败关闭。
- HTTP、MCP 和 PC 继续共用同一领域服务，列表显示调用凭证数量。

## 仍未完成

- 商户数据库中的完整订单、支付、退款、配送、售后和履约记录迁移。
- 接收方已能验证并隔离保存外部数据包；来源身份互认、字段映射、冲突解决、重新授权和回滚仍未完成。
- 加密归档、用户公钥签名、第三方托管证明、链上存证和法律认证。
- 自动定期导出、增量包、外部通知、云盘同步和超过单包上限的分页归档。

## 验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain open-commerce-portability-v3 -- test --manifest-path server\Cargo.toml open_commerce_portability --no-fail-fast
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
```
