# 开放商业开发者终态调用事件流 V1 验收

## 当前状态

`verified_rust_sqlite_service`

## 已验证

- 成功调用在数据库终态转换时自动登记一次，迁移会回填历史终态调用。
- 同一 App 可按稳定游标分页读取自己的成功或失败沙箱调用；其他 App 和同 App 的平台调用不会进入结果。
- V2 游标绑定 App 与凭据环境，跨 App 和损坏游标失败关闭；空轮询保留原检查点。
- 列表最多返回 100 条，只给摘要；单条详情才返回商户结果。
- 其他 App 读取单条详情时收到未找到，不暴露事件是否存在。
- 列表不包含原始输入、请求形状、请求哈希、Grant ID、项目 ID 或用户 ID。
- 计量固定显示 `funds_moved=false`，不宣称已真实扣款。
- 开发者事件模块两项 Rust/SQLite 服务测试通过，覆盖沙箱调用、平台事件隔离、跨 App 游标、详情越权和隐私字段。
- 全部 `open_commerce` Rust 过滤回归及服务器 Rust 全目标检查通过。
- PC 开发者门户可从当前检查点刷新、继续分页和读取单条结果；测试 Token 只保存在组件内存中。
- PC 开放商业静态契约、定向 ESLint、生产构建和硬性包体预算通过；总 JS/CSS 保留 3 条既有软预算告警。

## 仍未完成

- 本批未运行真实 HTTP 实例、测试 Token 鉴权、生产凭据事件读取、PC 浏览器或窄屏交互。
- 已形成的 Webhook、回调地址验证、签名、重试、死信和 SSRF 防护没有在本批执行真实 DNS、TLS、网络或工作器验证。
- 生产 App 的外部组织核验与跨运营方身份互认。
- 真实支付、退款、订单、配送或履约通知。

## 验证证据

- 修复前精确复现：`9e9408d5c0d0fc425b4a072ac20eb54314325e37b20ee4ef20638bcb7fe91db3`。
- 开发者事件模块两项测试：`d78571bc0de87a1793a812d1026b88ac98b5ad702b0219a34f9f6b461e1c0669`。
- 全部 `open_commerce` Rust 过滤回归：`5eb216be17647b54bcdc8de86b6ab0d83047bcf6a09b631fe63caae60c3fd551`。
- Rust 全目标检查：`0e9944e06e29ce76becda8cf38a77300c0a2f112fbbf870511cf6a8a7a865e56`。

## 验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce_developer_event_tests
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- check --manifest-path server\Cargo.toml --all-targets
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
npm --prefix pc-frontend run check:bundle-budget
```
