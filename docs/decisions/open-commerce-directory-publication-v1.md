---
title: "开放商业商户主动发布与脱敏目录 V1"
status: accepted
reviewed_at: 2026-08-01
---

# 开放商业商户主动发布与脱敏目录 V1

## 背景

开放商业 V1 已具备跨项目查询所需的商户、能力、授权和调用主干，但原实现把“商户节点可运行”和“商户愿意被外部发现”混为同一状态。只要商户处于 `active`，就可能进入全局搜索；发现响应还复用了内部领域对象，包含项目、所有者和处理器类型等不应进入公共目录的字段。普通登录调用也可以自报 App ID，无法充分证明调用者有权代表该应用。

## 决定

1. 新商户默认不进入开放目录。商户项目编辑者必须单独执行“发布到目录”，且至少存在一项有效的 `public` 或 `authorized` 能力。
2. 目录发布状态与商户运行状态分离，并记录发布人、修订号、发布时间、撤回时间和审计事件。
3. 公开发现只返回专用脱敏目录对象：商户 ID、slug、展示资料、公开资料、目录修订及可发现能力契约。
4. 目录不返回项目 ID、所有者 ID、节点模式、能力内部 ID、处理器类型、处理器配置、运行地址、密钥引用、授权记录或原始经营数据。
5. `owner_only` 能力永不进入目录；`authorized` 能力只公开调用契约，实际调用仍需独立 App、商户批准的 Grant 和幂等键。
6. 商户撤回目录后，外部发现立即消失，外部 App 即使持有尚未撤销的 Grant 也不能继续调用；商户项目编辑者仍可在项目内调试。
7. 非系统 App 的每次调用都必须验证 App 已注册、状态有效且归当前用户所有，不能只相信请求头中的 App ID。
8. 排序继续只使用商户公开资料、能力属性和消费者显式偏好，不引入付费排名。

## 非目标

- 本决定不建立生产 App 审核、跨运营方身份互认、公开注册、限流套餐或滥用处置中心。
- 本决定不实现支付、自动扣费、链上结算、订单履约或任何大厂生产适配器。
- 本决定不把商户数据公开下载；目录公开的是商户主动选择的能力契约。

## 结果

项目获得了“默认私有 -> 商户主动发布 -> 跨项目脱敏发现 -> 授权调用 -> 可随时撤回”的基础闭环。它可以作为后续消费者 AI 网络的目录层，但不能宣传为已经完成生产级公共商业网络。

## 实现证据

- `server/src/open_commerce_directory_service.rs`
- `server/src/store/open_commerce_directory.rs`
- `server/src/open_commerce_directory_model.rs`
- `pc-frontend/src/features/open-commerce/OpenCommerceDirectoryPublisher.tsx`
- `contracts/open-commerce/directory-merchant-v1.schema.json`
- `server/src/open_commerce_client_service_tests.rs`
