---
title: 开放商业 App 调用活动证据 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业 App 调用活动证据 V1

## 背景

商户已经可以设置调用配额和 Grant 总预算，也可以手动封禁失去信任的开发者 App。但如果控制面只展示封禁表单，商户仍需自行翻查调用记录，无法快速判断哪个 App 最近出现了重复失败、持续限流、预算拒绝或中断恢复。

V1 为商户提供可解释的活动证据，不建立黑箱风险分数，也不替商户自动作出处罚决定。

## 决定

1. 证据按“项目 + 商户 + 外部 App”聚合，只读取最近 24 小时已持久化的开放商业调用。
2. `pc-web` 和 `mcp-client` 属于共享系统入口，不进入外部 App 活动统计。
3. 控制面返回调用总数、成功数、失败数、限流数、Grant 预算拒绝数、中断恢复数和最近调用时间。
4. 关注原因使用稳定代码，不生成不可解释的综合分数：
   - `recovered_invocation`：至少一次调用因服务重启或租约过期被恢复流程失败关闭；
   - `repeated_failures`：24 小时内至少三次失败；
   - `rate_limit_pressure`：24 小时内至少三次触发限流；
   - `grant_budget_pressure`：至少一次被 Grant 总预算拒绝。
5. 关注状态只用于提醒。读取证据不得创建封禁、撤销 Grant、取消授权申请、扣款、赔付或改变调用权限。
6. 商户点击“处置”只把目标 App 填入现有紧急封禁表单；仍需人工选择原因并确认提交。
7. 证据只显示调用状态和计数，不返回 Token、原始请求、处理结果或消费者经营数据。

## 非目标

V1 不包含生产 App 审核、IP 或设备信誉、跨商户黑名单、机器学习风险评分、自动封禁、申诉工单、SLA 裁决、自动赔付和全网动态风控。24 小时证据也不代表 App 的长期信誉。

## 实现入口

- 聚合模型：`server/src/open_commerce_app_activity_health_model.rs`
- 只读证据查询：`server/src/store/open_commerce_app_activity_health.rs`
- 商户总览：`server/src/open_commerce_service.rs`
- PC 人工处置入口：`pc-frontend/src/features/open-commerce/OpenCommerceAppBlockManager.tsx`
- 领域测试：`server/src/open_commerce_app_activity_health_tests.rs`
- 验收：`docs/open-commerce-app-activity-health-v1-acceptance.md`
