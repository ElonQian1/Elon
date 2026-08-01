---
title: 开放商业商户级 App 封禁与紧急撤销 V1
status: accepted
date: 2026-08-01
owners: backend, product
---

# 开放商业商户级 App 封禁与紧急撤销 V1

## 背景

商户可设置固定时间窗调用配额，但配额只控制请求速度，不能终止已经失去信任的开发者 App。若 App 出现安全事件、违反商户规则或持续异常调用，商户必须能立即停止它访问自己的全部能力，并同时清理尚未完成的授权关系。

V1 只建设商户可解释的人工安全开关，不用缺乏证据的自动风险评分替商户做封禁决定。

## 决定

1. 封禁边界为“商户 + 已注册开发者 App”，不影响该 App 与其他商户的关系。
2. 只有商户项目编辑者可以封禁或解除封禁，项目查看者只能读取记录。
3. 封禁在同一数据库事务内完成三件事：激活封禁记录、撤销该商户授予该 App 的全部有效 Grant、取消该商户尚未处理的授权申请。
4. 被封 App 不能调用公开或受限能力，不能新建授权申请，商户也不能误为其创建新 Grant。
5. 重复封禁保持同一记录，已经撤销或取消的对象不会重复计数。
6. 解除封禁只恢复重新建立信任的可能，不恢复旧 Grant，也不恢复旧申请；App 必须重新申请授权。
7. `pc-web` 和 `mcp-client` 是共享系统入口，不能整体按 App 封禁。商户应撤回目录、停用具体能力或设置调用配额。
8. 审计记录封禁、解除、原因代码和撤销数量，不记录 Token、调用正文或用户经营数据。

授权创建、申请创建和调用认领在各自数据库写入临界区再次检查封禁状态。若封禁先取得数据库锁，新写入被拒绝；若调用已经认领成功，则视为在途请求，V1 不强制中断正在执行的商户处理器。

## 非目标

V1 不包含在途调用强制终止、自动封禁、IP 或设备信誉、验证码、生产 App 审核、跨运营方黑名单、申诉工单、DDoS 防护和全网动态风控。它是商户自己的紧急安全控制，不是公共网络治理已经完成的证明。

## 实现入口

- Schema：`server/src/open_commerce_app_block_migration.rs`
- 原子封禁与解除：`server/src/store/open_commerce_app_blocks.rs`
- 领域规则：`server/src/open_commerce_app_block_service.rs`
- HTTP：`server/src/open_commerce_app_block_api.rs`
- MCP：`server/src/open_commerce_mcp.rs`、`server/src/open_commerce_mcp_tools.rs`
- PC：`pc-frontend/src/features/open-commerce/OpenCommerceAppBlockManager.tsx`
- 验收：`docs/open-commerce-app-blocks-v1-acceptance.md`
