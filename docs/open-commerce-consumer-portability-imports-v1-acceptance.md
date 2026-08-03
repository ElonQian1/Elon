# 开放商业消费者可携带数据包隔离导入 V1 验收

## 当前交付

- 后端已提供当前消费者项目范围内的导入、列表、详情和删除接口。
- 导入复用 V1/V2/V3 导出包规范校验，并增加完整信封 SHA-256；相同目标项目、用户和信封摘要保持幂等。
- 导入内容只保存为 `isolated_snapshot`，不会恢复关系、生成 Grant、合并偏好、写 ERP、创建订单或结算。
- 来源固定标记为 `integrity_verified_source_untrusted`，不把自报来源标签和未签名业务内容冒充为可信运营方证明。
- PC 消费者工作台可选择 JSON、填写来源、查看隔离记录、重新下载和显式删除。
- 创建和删除进入开放商业审计日志；记录按目标项目和当前用户隔离。

## 验证状态

当前批次遵循快速实现策略，只要求 Rust 和 PC 前端能够编译。专项行为测试、完整回归和生产验收统一推迟到全部功能代码完成后执行。因此本能力当前状态为 `compiled_regression_deferred`，不能表述为已完成生产验证。

## 仍未完成

- 来源运营方签名、可信密钥目录、吊销和跨运营方身份互认。
- 导入内容预览、字段映射、冲突分析、选择性采用、重新授权和回滚。
- 加密归档、外部托管证明、链上存证、定时同步和增量包。
- 完整商户订单、支付、退款、配送、售后和履约记录迁移。

## 最终统一验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain open-commerce-portability-imports -- test --manifest-path server\Cargo.toml open_commerce_portability --no-fail-fast
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
```
