---
title: 开放商业消费者可携带数据包隔离导入 V1
status: accepted
date: 2026-08-03
owners: backend, product
---

# 开放商业消费者可携带数据包隔离导入 V1

## 背景

V3 已允许消费者导出本人关系、续期链、数据请求、低敏偏好披露和账户级调用凭证，但数据包只能在来源项目内读取和下载。消费者把数据带到另一运营环境后，接收方缺少统一的完整性验证、归属隔离和删除入口，数据所有权仍停留在“可以下载”，还没有形成可迁移的接收路径。

## 决定

1. 消费者项目可上传 V1、V2 或 V3 可携带数据包，并填写来源运营方或来源环境标签。服务端复用原导出校验，验证版本配对、规范 `payload_json`、总负载 SHA-256，以及 V3 每条调用凭证的规范表示和摘要。
2. 服务端还对完整导出信封计算 SHA-256，并按目标项目、当前用户和信封摘要幂等保存。相同信封重复上传返回首次导入记录，不复制快照。
3. 无签名导入记录固定为 `integrity_verified_source_untrusted`：摘要通过只证明接收内容与上传信封一致。独立的签名信任 V1 允许消费者登记运营方公钥，并把通过签名的记录标记为 `trusted_operator_signature_verified`。
4. 导入记录固定为 `isolated_snapshot`：不创建或恢复消费者关系，不生成 Grant，不合并偏好，不写入商户 ERP/CRM，不创建订单，不移动资金，也不触发外部适配器。
5. 记录严格绑定当前用户和目标消费者项目。用户可列出、读取、重新下载或删除自己的隔离快照，不能读取其他用户或项目的导入记录；创建和删除都写入审计日志。
6. HTTP 上传上限为 6 MiB，内部规范负载仍沿用 5 MiB 上限。超限、混合版本、字段不一致或摘要失败均拒绝导入，不保存部分数据。

## 后续边界

- 来源运营方签名、用户信任公钥和撤销已形成 V1 代码；全网身份认证、证书链、透明日志和自动轮换尚未实现。
- 账户映射、关系重新授权、字段冲突预演、选择性合并和一键回滚尚未实现。
- 商户完整订单、支付、退款、履约和敏感身份数据不属于当前可携带包。
- 当前导入不是法律认证、链上证明、云保险箱或灾难恢复。

## 实现入口

- `server/src/open_commerce_portability_import_model.rs`
- `server/src/open_commerce_portability_import_service.rs`
- `server/src/store/open_commerce_consumer_portability_imports.rs`
- `server/src/open_commerce_portability_import_migration.rs`
- `server/src/open_commerce_portability_api.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityImports.tsx`
- `docs/open-commerce-consumer-portability-imports-v1-acceptance.md`
