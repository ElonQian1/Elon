# 开放商业 App 调用活动证据 V1 验收

## 已验证闭环

- 商户项目总览按商户和外部 App 返回最近 24 小时的调用总数、成功数、失败数及最近调用时间。
- 限流、Grant 预算拒绝和中断恢复使用独立计数，并映射为稳定的可解释关注原因。
- 三次失败和三次限流的阈值由代码明确表达，不生成综合风险分数。
- `pc-web`、`mcp-client` 和超过 24 小时的记录不进入外部 App 活动证据。
- 普通成功 App 保持 `normal`；命中关注原因的 App 标记为 `attention`。
- 读取活动证据不会自动创建商户 App 封禁，也不会撤销授权或改变调用权限。
- PC 工作台展示具体计数；“处置”只填入现有人工封禁表单，不自动提交。
- 统计不读取或展示 Token、原始请求值、处理结果和消费者数据。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce-app-activity-health -- test `
  --manifest-path server\Cargo.toml open_commerce_app_activity_health -- --nocapture

node scripts\test-open-commerce-pc-workspace.js
```

## 尚未覆盖

- 生产开发者身份审核、跨运营方身份互认和跨商户联防。
- 动态时间窗、自定义阈值、IP 或设备信誉和机器学习异常检测。
- 自动封禁、自动赔付、申诉工单和 DDoS 防护。
- 多数据库部署下的跨节点聚合及近实时告警推送。

因此，本批只能描述为“商户侧 App 调用活动证据和人工处置入口已实现”，不能描述为“开放商业生产风控已经完成”。
