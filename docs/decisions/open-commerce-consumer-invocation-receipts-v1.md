---
title: 开放商业消费者调用凭证 V1
status: accepted
owner: backend
reviewed_at: 2026-08-02
---

# 开放商业消费者调用凭证 V1

## 背景

消费者或消费者 AI 已能调用商户能力，但调用结果此前主要存在于项目审计和单次响应中。消费者缺少一个独立、可复核且只属于本人账户的历史入口，无法在更换项目或客户端后确认“我调用了什么、结果是什么、平台是否真的扣款”。同时，直接复用商户审计视图会暴露项目、授权、幂等键和请求摘要等内部标识。

## 决定

1. 消费者调用凭证是已有 `OpenCommerceInvocation` 的只读投影，不创建第二套订单、支付或调用状态。
2. V1 按一龙登录账户归属，而不是按项目归属。现有调用记录没有消费者项目标识，因此同一账户从不同项目的 MCP 入口看到相同的本人凭证；PC 必须明确标注“账户级”。
3. 只投影 `succeeded` 和 `failed` 终态。进行中或缺少完成时间的记录不生成凭证。
4. 列表只返回摘要，不返回商户结果；详情只允许 `requester_user_id` 对应的本人读取，其他用户统一返回未找到，避免泄露记录是否存在。
5. 凭证公开商户与能力的稳定外部标识、调用状态、错误代码、时间、计量单位、币种与资金状态，但不公开用户 ID、项目 ID、能力内部 ID、Grant ID、幂等键、请求哈希或原始输入。
6. V1 只支持 `recorded_not_charged`。若未来出现真实资金状态，当前投影失败关闭，不能把真实扣款误标成“未扣款”。
7. 请求侧只重建字段数量、序列化字节数和“不包含原始值”的安全形状；商户返回结果只出现在本人详情中。
8. 服务端生成规范 `payload_json`，以其 UTF-8 字节计算 SHA-256。PC 下载前重新计算摘要、解析负载并核对外层对象，防止传输或客户端处理造成静默变化。
9. 摘要只是完整性指纹，不是外部时间戳、区块链证明、商户签名、支付凭证或欺诈裁决。
10. HTTP 与 MCP 共用同一领域服务和归属校验；MCP 不接受用户 ID 参数，调用身份由入口绑定。

## 边界

- 本决定不新增自动下单、支付、退款、配送、订单状态机或外部商户通知。
- 本决定不把能力返回值解释为真实成交，也不验证商户返回内容的业务真实性。
- 本决定发布时未把调用凭证加入消费者可携带数据包 V1；后续 V3 已纳入本人账户级调用凭证，见 `docs/decisions/open-commerce-consumer-portability-exports-v3.md`。跨运营方导入、冲突合并和完整订单迁移仍未实现。
- 商户返回结果可能包含消费者业务数据。下载文件由消费者自行保护，平台不宣称端到端加密或外部不可抵赖。
- 本决定不增加 Sui SDK、Move 合约、网络提交或可转让资产。

## 实现引用

- `server/src/open_commerce_consumer_receipt_service.rs`
- `server/src/store/open_commerce_consumer_receipts.rs`
- `server/src/open_commerce_consumer_receipt_api.rs`
- `server/src/open_commerce_consumer_receipt_mcp.rs`
- `pc-frontend/src/features/open-commerce/ConsumerInvocationReceipts.tsx`
