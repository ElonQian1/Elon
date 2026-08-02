---
title: 开放商业消费者可携带数据包 V2
status: accepted
date: 2026-08-02
owners: backend, product
superseded_by: docs/decisions/open-commerce-consumer-portability-exports-v3.md
---

# 开放商业消费者可携带数据包 V2

> V2 仍作为历史数据格式受支持；当前新建快照使用 V3。调用凭证进入数据包后的现行边界见 `docs/decisions/open-commerce-consumer-portability-exports-v3.md`。

## 背景

V1 已能把消费者本人关系、私有续期链和删除请求回执固化为可验证快照，但消费者自己保存的低敏结构化偏好及主动披露记录仍不能随包带走。这使“数据由消费者控制”在偏好层缺少可下载证据。

## 决定

1. V2 在 V1 来源之外加入当前消费者偏好档案和该用户全部历史披露快照。只包含平台已支持的类别、标签、城市、单位价格上限、公开能力偏好及消费者主动选择过的披露字段。
2. 关系、续期链、删除请求、当前偏好档案和历史披露在同一个数据库读事务中形成快照。关系、删除请求和披露各最多 5000 条，完整负载仍不超过 5 MiB；超限失败关闭，不截断。
3. 历史披露属于消费者本人记录。关系撤销或到期后，商户实时读取仍失败关闭，但消费者数据包可保留当时主动披露的快照和当时对应的匿名关系状态。
4. 新导出使用 `open_commerce.consumer_portability_export.v2` 和 `open_commerce.consumer_portability_payload.v2`。读取继续接受 V1；V2 新字段在缺省时不参与序列化，保证旧 V1 负载重新序列化后字节和 SHA-256 不变。
5. 相同幂等键仍永远返回首次快照。偏好档案更新不会改写已有包，也不会自动更新披露；生成新包或重新明确披露均须使用独立操作。
6. HTTP、MCP 和 PC 继续共用原领域服务。PC 展示偏好档案和披露计数，并在下载前对完整负载再次计算 SHA-256。

## 隐私与产品边界

- 数据包只属于当前项目中的当前用户，不向商户、项目其他成员或第三方 App 开放。
- V2 不导出消费者账号 ID、联系方式、自由文本、订单、支付、商户私有数据或外部凭据。
- V2 不是加密保险箱、账户恢复、跨运营方导入、商户 CRM 同步、数字签名、链上存证或法律认证。
- V2 导出历史披露不恢复已撤销授权，也不扩大商户当前访问权限。

## 兼容策略

V1 数据库记录无需迁移。版本标识和摘要继续保存在原表中；读取时只接受完整的 V1/V1 或 V2/V2 配对，版本混用失败关闭。

## 实现入口

- `server/src/open_commerce_portability_model.rs`
- `server/src/store/open_commerce_consumer_portability.rs`
- `server/src/open_commerce_portability_service.rs`
- `server/src/open_commerce_portability_v2_tests.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityExports.tsx`
- `docs/open-commerce-consumer-portability-exports-v2-acceptance.md`
