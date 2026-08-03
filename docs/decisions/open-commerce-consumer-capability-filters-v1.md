---
title: 开放商业消费者能力类型与访问级别筛选 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者能力类型与访问级别筛选 V1

## 背景

消费者 AI 需要区分信息查询与经营操作，也需要按公开调用或需授权调用寻找能力。只在结果返回后由客户端自行过滤，会使请求指纹、排序凭证和服务端候选范围不一致。

## 决定

1. 消费者发现请求增加可选 `capability_kind` 和 `access_level`。
2. 能力类型只接受 `query` 或 `action`；访问级别只接受 `public` 或 `authorized`。
3. 空白按未设置处理，未知值失败关闭；`owner_only` 不允许进入消费者公开发现。
4. 服务端在价格、来源和排序之前执行精确筛选。
5. 规范化条件通过 `capability_filter` 回显，并进入匹配原因、请求指纹和排序凭证规范负载。
6. PC 使用固定选项菜单，空值表示不限，结果区域显示最终采用的条件。
7. 本批不改变授权审批、Grant、动作确认、调用配额或结算规则。

## 信任边界

- `action` 只表示商户项目声明的能力类型，不证明操作已经执行。
- `authorized` 只表示能力要求授权，不代表当前应用已经获批。
- 筛选不能绕过短时动作确认、人工确认、授权范围或预算限制。
- 当前代码未编译，未执行接口、组合筛选、凭证、兼容性、浏览器或 UI 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_model.rs`
- `server/src/open_commerce_consumer.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCapabilityFilterFields.tsx`
- `docs/open-commerce-consumer-capability-filters-v1-acceptance.md`
