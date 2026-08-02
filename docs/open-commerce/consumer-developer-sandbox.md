# 消费者发现与第三方应用沙盒

本文说明已实现的消费者发现、应用注册、授权审批和开发者调用闭环。沙盒边界由 `docs/decisions/open-commerce-consumer-developer-sandbox-v1.md` 决定，跨项目目录发布由 `docs/decisions/open-commerce-directory-publication-v1.md` 决定，应用与申请生命周期由 `docs/decisions/open-commerce-developer-lifecycle-v1.md` 决定，商户紧急封禁由 `docs/decisions/open-commerce-app-blocks-v1.md` 决定，授权期限由 `docs/decisions/open-commerce-grant-expiration-v1.md` 决定，消费者关系及续期由 `docs/decisions/open-commerce-consumer-relationships-v1.md` 和 `docs/decisions/open-commerce-consumer-relationship-renewal-v1.md` 决定，偏好档案与关系级披露由 `docs/decisions/open-commerce-consumer-preference-disclosures-v1.md` 决定，关联数据删除请求由 `docs/decisions/open-commerce-consumer-data-erasure-requests-v1.md` 决定，本人可验证导出由 `docs/decisions/open-commerce-consumer-portability-exports-v3.md` 决定，消费者调用凭证由 `docs/decisions/open-commerce-consumer-invocation-receipts-v1.md` 决定，开发者终态事件流由 `docs/decisions/open-commerce-developer-terminal-events-v1.md` 决定，Schema 驱动调用表单由 `docs/decisions/open-commerce-schema-driven-invocation-form-v1.md` 决定，授权总预算由 `docs/decisions/open-commerce-grant-budgets-v1.md` 决定，商户侧调用活动证据由 `docs/decisions/open-commerce-app-activity-health-v1.md` 决定。

## 使用入口

项目详情的“开放商业”区域包含：

- **消费者沙盒**：选择请求 App，按搜索词、能力、城市、标签和价格上限发现商户能力，再按商户发布的输入契约填写并调用。
- **本人调用凭证**：按账户查看跨项目终态调用摘要，读取并下载经过 SHA-256 复核的单条结果。
- **开发者**：注册沙盒 App、轮换一次性测试 Token、处理授权申请、调试能力调用并按游标取回本 App 的终态结果。
- **商户工作台**：查看外部 App 近 24 小时的成功、失败、限流、授权预算拒绝和中断恢复证据，再人工决定是否封禁。

发现结果必须展示是否存在付费排序。当前实现固定采用非付费排序；分数来自公开资料、能力匹配和显式偏好，不能通过购买排名改变。

## 最小闭环

```text
商户编辑者主动发布脱敏目录
  -> 项目编辑者注册测试 App
  -> Token 只显示一次
  -> 消费者沙盒以该 App 发现能力
  -> 对 authorized 能力提交授权申请
  -> 商户项目批准或拒绝
  -> 使用 App 身份和 Grant 调用
  -> 复用幂等、计量与审计
```

公开能力可由 `pc-web` 直接调用。受限能力必须使用独立 App；`pc-web`、未知 App 和 `owner_only` 能力都不能绕过授权。非系统 App 还必须证明 App 归当前用户所有，不能通过伪造请求头借用其他 App 的 Grant。

## 按能力契约填写

消费者在匹配结果中选择“填写并调用”后，PC 根据该能力的 `input_schema` 生成字段、枚举、列表和格式约束。未声明默认值的可选字段默认不发送；无法安全呈现的契约直接阻断，不提供手写 JSON 绕过入口。商户声明为 `action` 的能力还要求对当前表单内容明确确认，任何字段修改都会清除旧确认。

该表单只改善消费者填写体验。服务端仍会重新校验输入并执行身份、Grant、配额、预算和幂等检查；商户运行时仍负责真实报价、库存与业务写入。界面中的技术服务金额当前只记录计量、未扣真实资金，调用成功也不能单独证明订单、支付或履约完成。

## HTTP 接口

| 用途 | 方法与路径 |
|---|---|
| 注册或列出项目测试 App | `GET/POST /api/projects/{project_id}/open-commerce/developer-apps` |
| 轮换一次性测试 Token | `POST /api/projects/{project_id}/open-commerce/developer-apps/{record_id}/rotate-token` |
| 停用或重新启用 App | `POST .../developer-apps/{record_id}/disable` 或 `reactivate` |
| 查看项目授权收件箱 | `GET /api/projects/{project_id}/open-commerce/authorization-requests` |
| 批准或拒绝申请 | `POST .../authorization-requests/{request_id}/approve` 或 `reject` |
| 查看或撤回本项目发出的申请 | `GET .../outbound-authorization-requests`、`POST .../{request_id}/cancel` |
| 发布或撤回商户目录 | `PUT /api/projects/{project_id}/open-commerce/merchants/{merchant_id}/directory-publication` |
| 设置或停用商户调用配额 | `PUT .../rate-limits`、`PATCH .../rate-limits/{policy_id}/enabled` |
| 列出、封禁或解除开发者 App | `GET/PUT .../app-blocks`、`POST .../app-blocks/{block_id}/unblock` |
| 消费者发现 | `POST /api/open-commerce/sandbox/discover` |
| 提交授权申请 | `POST /api/open-commerce/authorization-requests` |
| 使用测试 Token 调用 | `POST /api/open-commerce/developer/invoke` |
| 轮询本 App 终态事件 | `GET /api/open-commerce/developer/events?cursor=&limit=` |
| 读取本 App 单条终态结果 | `GET /api/open-commerce/developer/events/{invocation_id}` |
| 列出本人调用凭证 | `GET /api/open-commerce/consumer-invocation-receipts` |
| 读取本人单条调用凭证 | `GET /api/open-commerce/consumer-invocation-receipts/{invocation_id}` |

开发者调用和事件读取只使用 `Authorization: Bearer <test-token>`，App 身份由 Token 唯一确定，不能用额外请求头切换。Token 不得进入 URL、日志、项目文档、浏览器本地存储或商户能力元数据。

批准申请时，商户可选择 7 天、30 天、90 天、1 年或长期有效，并可填写授权期内的总调用次数和总预算（人民币元）。PC 默认 30 天；长期有效必须显式选择。用尽或到期后不能由 App 自行扩容、续期，商户需要重新授权。调用失败会退回刚预留的预算，重复请求不会再次占用。批准后的实际期限和预算会同时展示给商户与申请方。

消费者还可以在发现商户后建立独立的关系凭证。关系凭证不等同于 App Grant：它只允许商户把消费者主动提供的偏好或会员标识关联到随机匿名标识。PC 默认 90 天、最长 366 天，消费者可随时撤销；到期前 14 天显示续期入口。续期会撤销旧凭证并轮换匿名标识，相同请求只返回同一个新凭证。商户看不到消费者账号、用户 ID、消费者项目 ID 或内部续期链。关系凭证不存放偏好原文、联系方式、订单和支付数据。

消费者可另行保存类别、标签、城市和价格上限等低敏结构化偏好。保存不会自动向商户披露，也不会自动改变发现请求；用户可显式带入一次发现，或针对有效且含 `preference.remember` 的关系选择字段生成快照。商户只看到仍有效关系的匿名快照；档案更新不自动同步，关系撤销、到期或续期后旧披露立即不可见。该能力不保存自由文本或敏感身份资料，也不等于完整数据保险箱。

消费者还可针对本人关系发起关联数据删除请求。创建请求会原子撤销该关系；商户只能看到匿名关系别名，可接单、拒绝或声明完成。消费者可在接单前撤回请求，但关系不会恢复。`completed` 只表示商户提交了可审计声明，平台尚未验证美团、ERP、CRM 或会员系统中的真实删除结果。

消费者可把本人的关系历史、消费者私有续期链、删除请求回执、当前低敏结构化偏好档案和历史披露快照生成不可变 JSON 数据包。相同幂等键始终返回原快照，服务端和 PC 下载前都会复核 SHA-256；历史 V1 包继续按原字节验证。该 V2 数据包不含订单、联系方式、支付或账号 ID，当前也没有导入、冲突处理、加密归档或跨运营方迁移能力。

消费者还可查看本人账户发起的终态商业调用。列表只展示摘要，单条详情才包含商户返回结果；下载前会复核服务端规范负载的 SHA-256。凭证不暴露原始请求、项目、Grant、幂等键或内部用户标识，并固定说明当前计量未扣真实资金。由于现有调用真源不保存消费者项目标识，该入口是账户级而不是项目级；MCP 工具 `open_commerce_list_my_invocation_receipts` 和 `open_commerce_get_my_invocation_receipt` 也遵循同一边界。

活动证据来自已经保存的调用记录，只显示稳定计数和关注原因。“处置”只填入紧急封禁表单；系统不会因失败次数、限流或预算拒绝自动封禁 App，也不会把这些信号解释为跨商户信誉。

## 验收

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -TargetDir D:\rust\shared\target -- test `
  --manifest-path server\Cargo.toml `
  open_commerce_client_service_tests::consumer_discovery_request_approval_and_test_token_invocation_form_a_loop

Set-Location pc-frontend
npm run test:open-commerce
```

当前已实现商户主动选择的跨项目基础目录、限时授权、消费者可撤销及安全续期的关系凭证、低敏偏好字段披露、匿名删除请求与商户履约声明、含账户级调用凭证的本人 V3 可验证导出、Schema 驱动填写、服务端短时一次性动作确认、持久化能力调用配额、近 24 小时可解释活动证据、商户级手动 App 封禁，以及沙盒 App 生命周期闭环。通过该验收仍不代表生产公共网络已经完成；生产应用审核、跨运营方身份互认、数据包导入、完整订单迁移、外部通知、自动全网滥用处置、敏感数据保险箱、外部删除适配器、支付和真实平台适配器仍是后续模块。
