---
title: 通用 ERP 开放商业商户运行时桥 V1
status: accepted
owner: merchant-erp
reviewed_at: 2026-08-16
implementation_status: implemented_locally_verified
---

# 通用 ERP 开放商业商户运行时桥 V1

## 决定

1. 继续由 `@elon/open-commerce-connector` 独占签名、重放窗口、Grant、动作确认和运行时
   信封协议；ERP SDK 不复制这些安全实现。
2. 连接器能力定义新增可选 `action` 布尔值。任何声明为动作的能力都必须在业务处理器和
   幂等占位前携带平台动作确认；历史 `order.commit` 即使未声明也继续按动作保护。
3. `@yilong/merchant-erp-kernel` 提供纯组合函数，把 ERP Provider 编译为连接器需要的
   商户 ID、能力定义和处理器，不直接依赖或启动连接器。
4. 连接器签名信封中的幂等键是运行时调用的权威键。ERP 绑定从公开 `order.create` 输入
   Schema 移除重复键，并在商户身份一致后注入 ERP 内核。
5. ERP 订单能力返回标准业务回执，使现有消费者订单闭环和 ERP/CRM 衔接链能够读取真实
   ERP 订单引用；订单仍为未付款，回执不证明资金、履约或外部平台事实。

## 兼容与回滚

- 未使用 `action` 的查询能力保持原 Manifest 形状；旧 `order.commit` 的确认要求不变。
- 商户可继续手写连接器处理器；新绑定是可选组合层，不改变现有 Provider 调用接口。
- 回滚只需停止使用 `createMerchantRuntimeBinding`；ERP 数据和连接器凭据不迁移、不改写。

## 非目标

本决定不提供 HTTP 宿主、生产幂等数据库、密钥托管、支付、部署、平台适配器或链上结算。
