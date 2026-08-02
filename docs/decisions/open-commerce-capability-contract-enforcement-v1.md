---
title: 开放商业能力契约强制执行 V1
status: accepted
owner: backend
reviewed_at: 2026-08-02
---

# 开放商业能力契约强制执行 V1

## 背景

商户发布的 `input_schema` 和 `output_schema` 会被消费者 App、AI 与 MCP 用来理解能力，但此前平台只检查调用输入是否为 JSON 对象，没有按声明验证字段，也没有验证商户返回值。这样会出现两类风险：AI 发送商户未声明的参数；商户运行时返回与目录契约不一致的结果，却仍被计量为成功。

## 决定

1. 平台采用名为 `open_commerce.capability_schema.v1` 的有限 JSON Schema 配置，不宣称支持完整 JSON Schema 标准。
2. 能力创建或更新时校验 Schema。V1 支持对象、数组、字符串、整数、数字、布尔和空值，以及 `properties`、`required`、布尔型 `additionalProperties`、`items`、数量与长度上下限、数值上下限、`enum`、`const` 和 `uuid/date-time/uri` 格式。
3. 空对象 Schema 继续表示兼容旧能力的宽松契约。输入根节点若声明 `type`，必须是 `object`，与现有调用信封保持一致。
4. `$ref`、组合 Schema、条件分支、正则表达式和其他未实现关键字在发布时失败关闭；已有不受支持的 Schema 在调用时也以 `invalid_schema` 失败关闭。
5. 输入必须在创建 Invocation、占用限流和预留 Grant 预算前通过校验。无效输入返回 `422`，不产生调用和计量记录。
6. 处理器返回值必须在成功记账前通过输出校验。无效输出把 Invocation 标记为 `output_schema_violation`，金额为零并原子释放 Grant 预算；商户运行时同时降级，等待重新验证。
7. 幂等重放历史成功调用时，按当前输出契约重新验证已保存结果；契约已变化且结果不再匹配时拒绝重放，但不篡改原始历史调用状态。
8. 错误与审计只记录字段路径、规则代码、商户和能力标识，不记录输入值或返回值。
9. 发现结果和调用回执明确返回契约配置名称，PC 消费者沙盒显示“契约校验”。

## 边界

- 本决定不把能力契约变成业务真实性证明。Schema 通过不代表价格、库存、评价或商户身份真实。
- 本决定不执行远程 `$ref`，也不允许 Schema 拉取外部资源。
- 本决定不完成消费者自动下单、支付、退款、生产 App 审核或跨运营方治理。
- 输出校验失败只阻止平台把本次调用记为成功，不会回滚商户后端已经发生的外部副作用。写能力仍必须在商户端使用幂等、报价、用户确认和事务边界。

## 实现引用

- `server/src/open_commerce_capability_schema.rs`
- `server/src/open_commerce_capability_contract_service.rs`
- `server/src/store/open_commerce_capabilities.rs`
- `server/src/open_commerce_service.rs`
- `server/src/open_commerce_invocation_protocol.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
