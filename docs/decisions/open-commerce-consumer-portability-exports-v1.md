---
title: 开放商业消费者可携带数据包 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业消费者可携带数据包 V1

## 背景

消费者已经能够持有、续期和撤销匿名商户关系，也能查看关联数据删除请求，但实时列表不能充当可长期核验的个人副本。若只提供临时下载，网络重试还可能在不同时间得到不同内容，用户无法判断文件是否被意外修改。

V1 先把平台已经持有且明确属于当前消费者的关系元数据整理成不可变快照，为后续跨运营方导入协议提供可验证输入，而不提前假设偏好保险箱、订单迁移或外部商户系统已经存在。

## 决定

1. 数据包属于“消费者项目 + 当前用户”。同项目其他成员、商户和第三方 App 均不能列出或读取该用户的数据包。
2. V1 负载只包含该用户的消费者关系记录、消费者私有的关系续期链和关联数据删除请求回执，并保存来源项目、生成时间与版本。它不包含消费者账号 ID、偏好原文、联系方式、订单、支付、商户私有数据或外部凭据。
3. 关系和删除请求在同一个数据库读事务中形成快照，按稳定时间顺序输出。V1 每类最多 5000 条记录，序列化负载最多 5 MiB；超过上限失败关闭，不静默截断。
4. 调用方提供 8 至 120 个字符的幂等键。数据库对“项目 + 用户 + 幂等键”建立唯一约束；同一键永远返回首次生成的快照，即使来源数据随后变化。需要新状态时必须使用新键。
5. 平台对版本化负载的紧凑 JSON 计算小写十六进制 SHA-256，并保存不可变负载与摘要。列表、详情和幂等重放均重新计算摘要；版本、项目或摘要不一致时拒绝返回。
6. PC 创建或读取完整数据包后，在浏览器再次对负载计算 SHA-256；校验通过才触发 JSON 下载。摘要覆盖 `payload`，不覆盖导出记录的数据库 ID、幂等键或创建时间。
7. HTTP、MCP 与 PC 共用同一领域服务。只有首次创建写入消费者项目审计；幂等重放不重复写创建事件。
8. 商户原有关系读取仍不返回续期链。续期链只进入消费者本人拥有的数据包，不改变商户可见边界。

## 非目标

V1 不包含数据包导入、跨运营方迁移、账户恢复、数字签名或公钥证明、加密归档、偏好数据保险箱、订单和支付导出、商户 CRM 自动同步、法定数据副本认证、链上存证或真实结算。

## 实现入口

- 模型：`server/src/open_commerce_portability_model.rs`
- 迁移：`server/src/open_commerce_portability_migration.rs`
- 存储：`server/src/store/open_commerce_consumer_portability.rs`
- 领域服务：`server/src/open_commerce_portability_service.rs`
- HTTP：`server/src/open_commerce_portability_api.rs`
- MCP：`server/src/open_commerce_mcp.rs`、`server/src/open_commerce_mcp_tools.rs`
- PC：`pc-frontend/src/features/open-commerce/ConsumerPortabilityExports.tsx`
- 验收：`docs/open-commerce-consumer-portability-exports-v1-acceptance.md`
