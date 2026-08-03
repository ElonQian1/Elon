---
title: Sui 离线预检适配器身份与报告 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# Sui 离线预检适配器身份与报告 V1

## 背景

平台已经能把标准投影和纠正双腿投影导出为统一、确定性的离线交接包，但此前没有独立机器身份，也无法接收外部工具的预检结论。直接把下载接口扩展成后台签名或广播入口，会混淆链下内容复核、适配器可信度和链上终局三种不同事实。

## 决定

1. 每个项目可以创建多个离线预检适配器档案，分别限制允许的 `devnet/testnet/mainnet` 和 `standard/correction`。档案不是钱包、Sui 地址或链上对象。
2. 适配器凭据明文只在签发或轮换时返回一次，服务端只保存 SHA-256、末尾提示、凭据版本和最长 366 天有效期。停用后立即替换摘要并使旧凭据失效，历史报告不删除。
3. 机器报告入口由 `ELON_SUI_OFFLINE_PREFLIGHT_ENABLED` 控制并默认关闭。项目成员可在入口关闭时管理档案和查看历史，但机器不能提交新报告。
4. 机器只能提交 `passed` 或 `rejected`。服务端按投影类型从数据库只读重建当前交接包，要求项目、包 ID、目标网络、权限和 `handoff_digest` 全部匹配后才保存报告。
5. 报告是追加式证据，绑定适配器及凭据版本、投影包、网络、交接摘要、投影摘要、工具版本、结论和说明。同一适配器与幂等键的完全相同请求返回原报告，不同内容失败关闭。
6. 机器报告链路不得调用会改写投影完整性或验证时间的复核操作；它不改变 `submission_readiness`、`network_submission`、提交次数或争议状态。
7. `passed` 只表示该外部工具对确定性交接内容完成离线预检；`rejected` 只保存其拒绝理由。两者都不授权签名、PTB 构建、RPC 广播、Gas 支付、终局确认或资金移动。

## 边界

- 当前没有 Sui SDK、Move Package、钱包、私钥、签名服务、RPC 客户端、Gas 预算、交易摘要、对象 ID 或最终性监听器。
- 机器凭据证明请求来自已登记适配器，不证明适配器软件、运行主机或预检算法正确。
- 报告不能替代平台投影完整性复核、争议处理、财务审计或链上证明。
- 当前实现尚未编译、执行迁移、运行接口、验证并发幂等或检查 PC 页面。

## 实现引用

- `server/src/task_settlement/sui_preflight_*.rs`
- `server/src/store/task_sui_preflight_*.rs`
- `server/src/task_sui_preflight_migration.rs`
- `pc-frontend/src/features/open-commerce/SuiPreflightAdaptersPanel.tsx`
- `docs/sui-offline-preflight-adapters-v1-acceptance.md`
