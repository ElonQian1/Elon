---
title: 开放商业消费者删除请求催办与升级关注 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者删除请求催办与升级关注 V1

## 背景

删除请求 V1 已记录消费者请求、关系撤销和商户处理状态，但请求发出后缺少有界的跟进入口。消费者无法留下可审计的催办事实，商户收件箱也无法区分普通未处理请求与长期未处理请求。V1.1 增加内部运营目标、手动催办和升级关注，不把这些产品规则包装成法律期限、平台仲裁或外部删除证明。

## 决定

1. 每个请求从 `requested_at` 起计算固定 7 天内部运营目标；该时间只用于产品排队和界面提示。
2. `requested` 或 `in_progress` 请求在创建 24 小时后允许首次催办，后续催办至少间隔 24 小时，每个请求最多 3 次。
3. 请求超过内部运营目标且至少有一次催办后，消费者可追加一次 `escalate_attention`。升级关注只提高商户收件箱排序优先级。
4. 每次跟进必须携带 1 至 120 字符幂等键；同一消费者、请求和幂等键的相同操作可安全重放，不同操作或说明失败关闭。
5. 催办与升级采用 V163 独立追加式表，不覆盖删除请求状态、不恢复关系，也不修改商户处理说明或删除证明。
6. 列表响应在既有请求字段上增加可选运营字段。可携带数据包继续读取原始删除请求记录，零值运营字段不序列化，因此不会静默改变既有 V5 包内容。
7. 商户列表先展示仍未终结且已升级关注的请求，再展示超过内部目标的请求；同等优先级保留原状态与更新时间顺序。
8. 有效变化写入消费者项目审计，但审计明确记录 `legal_deadline_asserted=false` 和 `platform_adjudication_started=false`。

## 边界

- V1 不自动发送短信、邮件、Webhook 或第三方平台通知；消费者必须在 PC 手动发起跟进。
- 7 天不是任何国家或地区的法定时限判断，不构成合规结论。
- 升级关注不会创建平台客服工单、争议、仲裁、处罚、赔付、退款或信誉分。
- 催办和升级不证明商户、ERP、CRM、美团或会员系统已删除数据。
- 跟进记录当前不进入 V5 可携带包，也不跨运营方迁移。
- 当前代码未编译，未执行 V163 迁移、并发、时间边界、HTTP 或 PC 验证，状态为 `implementation_uncompiled`。

## 实现入口

- `server/src/open_commerce_data_request_followup_migration.rs`
- `server/src/store/open_commerce_data_request_followups.rs`
- `server/src/open_commerce_data_request_model.rs`
- `server/src/open_commerce_data_request_service.rs`
- `server/src/open_commerce_data_request_api.rs`
- `pc-frontend/src/features/open-commerce/ConsumerDataRequestManager.tsx`
- `pc-frontend/src/features/open-commerce/MerchantDataRequestInbox.tsx`
- `docs/open-commerce-consumer-data-request-followups-v1-acceptance.md`
