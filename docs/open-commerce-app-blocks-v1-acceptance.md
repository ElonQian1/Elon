# 开放商业商户级 App 封禁 V1 验收

验收日期：2026-08-01

## 已验证闭环

- 商户项目编辑者可按已注册 App ID 和原因创建封禁；查看者、未知 App 和共享系统入口被拒绝。
- 封禁动作在同一事务内撤销该商户的有效 Grant，并取消该商户的待审批授权申请。
- 被封 App 不能调用公开能力、不能提交新授权申请，商户也不能为其新建 Grant。
- 授权、申请和调用认领在数据库写入临界区再次检查封禁状态，封禁后不能利用检查到写入的时间窗新增对象。
- 重复封禁复用同一记录，不重置首次封禁时间，不重复计算已撤销对象。
- 解除封禁不会恢复旧 Grant；公开能力可重新调用，受限能力必须重新申请授权。
- 封禁和解除均写入项目审计，审计不保存调用原始值或 Token。
- HTTP 调用入口把类型化封禁错误映射为 `403 Forbidden`。
- 商户 PC 工作台可查看历史、执行封禁和解除，并显示撤销授权与取消申请的数量。
- 商户项目 AI 可通过 MCP 列出、封禁和解除 App。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce-app-block -- test `
  --manifest-path server\Cargo.toml open_commerce_app_block_tests

Set-Location pc-frontend
npm run build
npm run test:open-commerce
```

同时运行开放商业 Rust 回归、完整 Rust `check`、源码规模、文档模块化和差异检查。

## 尚未覆盖

- 生产 App 审核、开发者实名或组织身份互认。
- 自动风险评分、IP/设备信誉、跨商户联防和申诉处置。
- 多数据库部署下跨节点即时传播封禁状态。
- 已经认领并进入商户处理器的在途调用强制终止。
- 真实收费、退款、争议赔付和链上结算。
- 美团、抖音、京东、淘宝闪购等真实生产适配器。

因此，本批只能描述为“商户级 App 紧急封禁与授权撤销 V1 已实现”，不能描述为“开放商业生产风控已经完成”。
