---
title: Sui 适配器离线交接包 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# Sui 适配器离线交接包 V1

## 背景

标准影子凭证和纠正双腿已经能够保存为不可变 Sui 链下投影包，但两类包使用不同内部结构。未来独立网络适配器需要一个统一、可复核且不包含平台数据库访问权的输入契约，同时不能把“导出文件”误解为已签名或已广播的交易。

## 决定

1. 标准投影和纠正投影共用 `task_economy.sui_adapter_handoff.v1` 外层契约，并通过 `package_kind`、`source_id` 和 `atomic_bundle` 保留语义差异。
2. 每次导出先复用原投影服务重新计算来源摘要、投影摘要和信封。完整性冲突、阻断争议、非 `not_submitted` 状态或非零提交次数全部失败关闭。
3. 交接负载绑定项目、投影包、来源、目标网络、包 Schema、双摘要、原始信封、包创建时间和离线约束；`handoff_digest` 是负载规范序列化后的 SHA-256。
4. 纠正交接包必须保留完整冲销与替换信封并固定 `atomic_bundle=true`，不得拆成两份普通交接包。
5. 交接约束固定 `allowed_adapter_action=offline_preflight_only`，并明确签名、广播、最终性和资金移动均为 `false`。
6. 交接包按请求确定性生成，不新增交接状态表。项目编辑者可从 PC 下载 JSON；读取和导出不创建钱包、交易或网络提交记录。
7. 未来网络适配器必须另行实现机器身份、短时租约、密钥隔离、Gas 预算、幂等广播、链上最终性和失败恢复，不能直接把本交接接口改写为后台广播入口。

## 边界

- 当前没有 Sui SDK、Move Package、PTB、钱包、私钥、签名、RPC、Gas、交易摘要、对象 ID 或最终性证明。
- 交接摘要证明平台导出的链下内容一致，不证明外部文件保管安全、适配器可信或任何链上状态。
- 当前代码尚未编译、运行接口、验证摘要往返、下载文件或检查 PC 页面。

## 实现引用

- `server/src/task_settlement/sui_adapter_handoff_model.rs`
- `server/src/task_settlement/sui_adapter_handoff_service.rs`
- `server/src/task_settlement/sui_adapter_handoff_api.rs`
- `pc-frontend/src/features/open-commerce/suiAdapterHandoffDownload.ts`
- `docs/sui-adapter-offline-handoff-v1-acceptance.md`
