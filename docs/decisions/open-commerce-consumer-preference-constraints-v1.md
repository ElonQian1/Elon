---
title: 开放商业消费者偏好硬约束 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者偏好硬约束 V1

## 背景

消费者的城市、经营类别和标签原本只参与排序加分。自动决策场景中，“偏好吉安”和“必须在吉安”语义不同；如果没有显式硬约束，AI 可能返回得分较高但违反用户必要条件的商户。

## 决定

1. 消费者发现请求增加 `require_city_match`、`require_category_match` 和 `require_all_tags_match`，默认均为 `false`。
2. 城市硬约束要求商户公开资料城市与单个偏好城市忽略 ASCII 大小写后相等。
3. 类别硬约束要求商户单个公开类别命中消费者类别列表中的任意一项。
4. 全部标签硬约束要求每个消费者标签都能在商户公开标签中找到忽略 ASCII 大小写的匹配项。
5. 启用约束却没有对应偏好值时失败关闭；商户资料字段缺失、类型错误或不匹配时排除该商户。
6. 三个开关通过 `preference_constraints` 回显，并进入匹配原因、请求指纹和排序凭证规范负载。
7. 未启用的城市、类别和标签继续作为原有软偏好参与评分，不改变旧请求行为。

## 信任边界

- 匹配基于商户项目公开资料，不证明地址、类别或标签经过外部机构验证。
- 标签匹配不自动推断同义词、过敏原、法律资格或服务质量。
- 硬约束只影响发现候选，不代表授权、库存、下单或履约完成。
- 当前代码未编译，未执行接口、资料缺失、组合筛选、凭证、浏览器或 UI 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_consumer_constraints.rs`
- `server/src/open_commerce_consumer_model.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPreferenceConstraintFields.tsx`
- `docs/open-commerce-consumer-preference-constraints-v1-acceptance.md`
