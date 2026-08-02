# 开放商业开发者终态调用事件流 V1 验收

## 已验证

- 成功调用在数据库终态转换时自动登记一次，迁移会回填历史终态调用。
- 同一 App 可按稳定游标分页读取自己的成功或失败调用；其他 App 的事件不会进入结果。
- 游标绑定 App，跨 App 和损坏游标失败关闭；空轮询保留原检查点。
- 列表最多返回 100 条，只给摘要；单条详情才返回商户结果。
- 其他 App 读取单条详情时收到未找到，不暴露事件是否存在。
- 列表不包含原始输入、请求形状、请求哈希、Grant ID、项目 ID 或用户 ID。
- 计量固定显示 `funds_moved=false`，不宣称已真实扣款。
- 定向 Rust 迁移、隔离、游标、隐私和详情测试，以及全部 `open_commerce` Rust 回归通过。

## 仍未完成

- Webhook、移动推送、外部消息队列和跨运营方事件同步。
- 回调地址所有权验证、签名密钥、重试退避、死信、SSRF 防护和投递 SLA。
- 生产 App 审核与跨运营方身份互认。
- 真实支付、退款、订单、配送或履约通知。

## 验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce_developer_event -- --nocapture
```
