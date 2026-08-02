---
title: 开放商业消费者可携带数据包 V3
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业消费者可携带数据包 V3

## 背景

V2 已能导出消费者本人关系、续期链、删除请求、低敏偏好档案和历史披露，但本人通过不同项目和 App 完成的商业能力调用仍只能逐条下载。消费者无法用一个快照保存“我授权调用过什么、商户返回了什么、平台记录了多少未扣费技术计量”。

## 决定

1. V3 在 V2 来源之外，加入当前登录用户账户下全部终态开放商业调用凭证。范围固定为 `authenticated_user_account`，不是当前消费者项目，也不允许请求任意用户 ID。
2. 每条嵌入凭证沿用 `open_commerce.consumer_invocation_receipt.v1`，保存独立 `payload_sha256` 和服务端生成的规范 `payload_json`。负载包含商户、能力、调用 App、字段形状、终态、商户返回结果及未扣费计量，但不包含原始输入值、请求摘要、Grant、调用幂等键、内部能力 ID、项目 ID 或消费者账号 ID。
3. 关系、续期链、删除请求、偏好、披露和终态调用记录在同一个 SQLite 读事务中形成快照。每类记录最多 5000 条，完整包仍不得超过 5 MiB；任一上限超出时失败关闭，不静默截断。
4. 新导出使用 `open_commerce.consumer_portability_export.v3` 和 `open_commerce.consumer_portability_payload.v3`。读取只接受完整的 V1/V1、V2/V2 或 V3/V3 配对，版本混用失败关闭。
5. V1/V2 没有 V3 字段时仍按原字段顺序重新序列化，必须保持原字节和 SHA-256。旧版若夹带 V3 调用凭证字段则失败关闭。
6. 导出信封同时返回完整包的规范 `payload_json`，避免 Rust 与浏览器对任意商户 JSON 键顺序解释不同。服务端读取时复核规范字符串、每条调用凭证摘要和完整包摘要；PC 下载前对原始规范字符串重复执行两级 SHA-256 校验，任一级不一致都停止下载。
7. 相同幂等键仍返回首次不可变快照。后续调用只进入使用新幂等键生成的新包，不改写历史包。

## 数据边界

- 商户返回结果属于消费者已经获得的调用结果，可进入本人凭证；它不等于商户数据库中的完整订单、支付、退款、配送或履约记录。
- V3 仍不导出联系方式、消费者账号、原始调用输入、商户私有数据库、外部凭据或真实资金证明。
- 仅支持 `recorded_not_charged` 调用。出现其他结算状态时整包失败关闭，不能把未知状态改写成未扣费。
- V3 不是跨运营方导入协议、账户恢复、加密保险箱、数字签名、链上存证或法律认证。

## 实现入口

- `server/src/open_commerce_portability_model.rs`
- `server/src/store/open_commerce_consumer_portability.rs`
- `server/src/open_commerce_portability_service.rs`
- `server/src/open_commerce_portability_v3_tests.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityExports.tsx`
- `docs/open-commerce-consumer-portability-exports-v3-acceptance.md`
