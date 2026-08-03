---
title: 开放商业 Webhook 运行健康 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业 Webhook 运行健康 V1

## 背景

订阅、投递、重试、死信和生产资格已有各自真源，但运营者需要逐条翻查才能判断当前 App 是否积压、是否存在待处理死信，以及生产通知为什么不能启用。健康能力必须提供统一视图，同时不能复制队列状态或自动改变订阅。

## 决定

1. 健康摘要是只读派生视图，不新增健康表、不保存第二套状态，也不自动启停、重试、处罚或赔付。
2. 每个 App 固定返回沙箱和生产两个环境，分别聚合订阅总数、活动数、已验证数、待发、投递中、重试、死信、最早排队时间、最近成功时间和最近错误码。
3. 环境状态只允许 `idle`、`healthy`、`processing`、`attention` 和 `action_required`。死信或活动生产订阅失去生产就绪条件时要求处置，重试需要关注，正常在途任务标记处理中。
4. 生产就绪度同时公开两个功能开关、当前生产凭据资格、综合就绪结果和稳定阻断码。管理界面不直接展示密钥、Token 摘要、主体声明或内部数据库错误。
5. 健康 API 复用 Webhook 管理的 App 所有者和项目访问边界；PC 将摘要显示在订阅管理区，并在订阅或投递列表刷新后重新读取。
6. 指标是查询时快照，不是 SLA、外部送达证明、支付结果、ERP 入库结果或跨运营方信誉评分。

## 边界

- 当前没有时间序列、阈值配置、外部告警、通知升级、自动修复、投递延迟分位数或运营方全局总览。
- 历史死信会持续计入环境摘要，直到原记录被合法重试或后续版本引入明确的人工归档语义。
- 当前代码尚未编译、运行查询、验证大数据量性能或检查 PC 页面。

## 实现引用

- `server/src/open_commerce_webhook_health_model.rs`
- `server/src/open_commerce_webhook_health_service.rs`
- `server/src/store/open_commerce_developer_webhook_health.rs`
- `server/src/open_commerce_webhook_api.rs`
- `pc-frontend/src/features/open-commerce/DeveloperWebhookHealthSummary.tsx`
- `docs/open-commerce-webhook-operational-health-v1-acceptance.md`
