# 开放商业能力契约强制执行 V1 验收

## 已验证

- 能力发布拒绝未支持的 `$ref` 和非对象输入根类型。
- 报价类嵌套数组可校验必填字段、UUID、整数范围、长度和多余字段。
- 无效输入在创建 Invocation 前返回，不占调用记录、计量或 Grant 预算。
- 无效输出保存为零金额失败调用，错误代码为 `output_schema_violation`，并释放已预留的 Grant 次数与金额。
- 调整输出契约后，前一次失败释放的预算可被后续成功调用正常使用。
- 历史成功调用在幂等重放前会按当前输出契约重新校验；契约变化造成不兼容时拒绝重放，原始历史记录保持不变。
- 契约错误和审计只包含路径及规则代码，不包含测试输入值或商户返回值。
- HTTP 能把契约违例映射为 `422 Unprocessable Entity`。
- 发现响应和调用回执标明 `open_commerce.capability_schema.v1`，PC 消费者沙盒显示契约校验状态。
- Rust 编译检查、定向契约测试、全部 `open_commerce` 回归、开放商业与 ERP 的 PC 静态契约、PC 类型检查、定向 ESLint 和生产构建通过。

## 仍未完成

- 完整 JSON Schema 2020-12、远程引用、组合 Schema、条件 Schema 和正则表达式。
- 业务数据真实性、数据新鲜度、商户身份、评价可信度和欺诈治理。
- 已发生外部副作用的自动补偿；写能力仍依赖商户运行时事务和幂等设计。
- 生产 App 审核、真实支付、退款、跨运营方互认和公共网络治理。

## 验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce_capability_ -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce -- --nocapture
npm --prefix pc-frontend run test:open-commerce
cd pc-frontend
npm run build
npx eslint src/features/open-commerce/ConsumerCommerceSandbox.tsx src/features/open-commerce/openCommerceClientTypes.ts --max-warnings 0
```
