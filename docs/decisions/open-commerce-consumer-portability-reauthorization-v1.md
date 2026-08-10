---
title: 开放商业消费者关系人工映射与重新授权 V1
status: accepted
date: 2026-08-03
owners: backend, product
reviewed_at: 2026-08-10
implementation_status: verified_rust_http_pc_build
---

# 开放商业消费者关系人工映射与重新授权 V1

## 背景

可携带数据包中的商户 ID 和关系 ID 属于来源运营环境，不能直接作为目标环境身份。自动复制旧关系或 Grant 会绕过目标商户审批，也可能把两个不同商户错误合并。首个迁移路径必须把身份判断和新授权明确拆开。

## 决定

1. 消费者从本人隔离导入中选择一条来源关系，并在 PC 端公开目录中按名称搜索、选择目标商户。目标商户必须在当前网络中处于有效且已发布状态。搜索结果不构成身份认证，不代替消费者确认。
2. 创建映射必须由消费者明确确认“来源与目标是同一业务主体”。映射权威固定为 `consumer_confirmed`，不冒充平台认证、工商认证或密码学证明。
3. 同一导入包和来源关系同一时间只允许一个有效映射。改映射前必须撤销旧记录；撤销不删除历史审计。
4. 映射本身不创建关系、Grant、订单或 ERP 数据。消费者必须选择自己拥有的独立开发者 App，并向目标商户提交一笔全新的授权申请。
5. 新申请范围必须是来源关系范围的子集，同时继续经过目标商户当前能力、发布状态、App 封禁和授权规则校验。
6. 旧 Grant 固定 `old_grant_restored=false`。目标商户仍可批准或拒绝新申请，迁移不能绕过目标商户决定。
7. 映射创建、撤销和重新授权申请都写入审计日志；映射按当前用户和消费者项目隔离。
8. 对签名可信的 V4 包，若来源与目标商户具有相同的有效持有证明指纹，映射记录保存 `trusted_operator_key_match` 和匹配键。该证据不代替消费者确认。

## 验证证据

- 4 项 Rust/SQLite 与进程内 Axum 专项覆盖明确确认、同目标幂等、冲突映射、撤销、范围子集、本人 App、目标发布状态、旧 Grant 不恢复，以及登录、项目和同项目用户隔离。
- `cargo check --tests` 与目标测试 `portability_reauthorization` 已通过；目标测试结果为 4 通过、0 失败。
- PC 已通过严格类型检查、开放商业契约回归和 Vite 生产构建。
- 上述证据只验证本机 SQLite 和进程内 HTTP 路由，不等同于真实 TCP、浏览器交互、跨运营方身份互认或生产部署。

## 边界

- 当前没有自动商户身份联邦、域名证明、工商证明、DID、证书链或跨运营方商户签名。
- 人工映射可能判断错误；平台只记录谁在何时确认，不为业务主体同一性背书。
- 撤销映射不会自动撤销已经提交或批准的新授权，授权仍使用现有独立生命周期。
- 当前只提供公开目录文本检索，没有批量映射、候选智能匹配或商户主动认领流程。

## 实现入口

- `server/src/open_commerce_portability_reauthorization_model.rs`
- `server/src/open_commerce_portability_reauthorization_service.rs`
- `server/src/store/open_commerce_portability_reauthorization.rs`
- `server/src/open_commerce_portability_reauthorization_api.rs`
- `server/src/open_commerce_portability_reauthorization_migration.rs`
- `server/src/open_commerce_portability_reauthorization_service_tests.rs`
- `server/src/open_commerce_portability_reauthorization_api_tests.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityReauthorization.tsx`
