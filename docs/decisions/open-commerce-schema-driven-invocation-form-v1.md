---
title: 开放商业 Schema 驱动调用表单 V1
status: accepted
owner: pc-frontend
reviewed_at: 2026-08-02
---

# 开放商业 Schema 驱动调用表单 V1

> 后续服务端确认边界见 `docs/decisions/open-commerce-server-action-confirmation-v1.md`。本文件中的页面确认现已作为交互层，不能单独授权动作执行。

## 背景

消费者沙盒此前可以发现并调用商户能力，但 PC 对所有能力都提交空对象。能力输入输出契约开始由服务端强制执行后，需要报价编号、商品明细、预约时间等字段的能力会被正确拒绝。要求普通消费者手写 JSON 既不符合产品定位，也容易把未经约束的数据发送给商户运行时。

## 决定

1. PC 从发现结果中的 `input_schema` 生成非技术表单，不维护第二份能力字段定义。
2. V1 支持受限的对象、列表、文本、整数、数值、开关、空值、简单枚举、固定值、默认值、长度、数量、数值范围以及 `uuid`、`date-time`、`uri` 格式。输入根节点必须是对象。
3. 表单最多递归 12 层、单个列表最多填写 50 项。复杂枚举、缺少项目结构的列表、没有字段定义的必填项和超出呈现上限的契约失败关闭，不提供原始 JSON 绕过入口。
4. 未声明默认值的可选字段默认不进入请求；可选开关、空值和固定值需要显式选择，避免把“未提供”误写成“否”、`null` 或固定值。带 `default` 的字段显示并提交商户给出的默认值；必填固定值由契约生成且不允许消费者篡改。
5. 客户端校验只提供即时反馈；服务端能力契约仍是最终权威，调用继续经过 App 身份、Grant、配额、预算、幂等、计量、审计和商户运行时边界。
6. 商户声明为 `action` 的能力必须由消费者对当前表单内容再次明确确认。任何字段修改都会清除确认；`query` 能力不要求该确认。
7. `kind` 来自商户发布契约，因此动作确认是用户体验防线，不证明商户把能力分类正确，也不替代商户运行时自己的报价、库存、写操作确认或风控。
8. 表单显示的平台技术服务计量仍为 `recorded_not_charged`。商户返回结果不是订单、支付、履约或链上交易证明。
9. 同一份输入的失败或网络不确定重试复用同一个幂等键；字段变化或切换能力后才生成新键。成功后关闭表单并废弃当前确认，避免把旧确认用于第二次动作。

## 边界

- 本决定不增加或改变 HTTP、MCP、能力 Schema、授权或调用协议。
- 本决定不支持完整 JSON Schema、任意附加属性、条件 Schema、联合类型、文件上传或自由 JSON 编辑。
- 本决定不创建支付、退款、配送、订单状态机、真实平台连接器或链上提交。
- AI 和第三方应用仍可按现有协议提交结构化输入；本决定只补齐 PC 消费者的非技术填写路径。

## 实现引用

- `pc-frontend/src/features/open-commerce/capabilityInvocationSchema.ts`
- `pc-frontend/src/features/open-commerce/CapabilitySchemaField.tsx`
- `pc-frontend/src/features/open-commerce/CapabilityInvocationComposer.tsx`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
- `pc-frontend/scripts/test-capability-invocation-schema.cjs`
