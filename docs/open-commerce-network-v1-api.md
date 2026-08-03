---
title: AI 原生开放商业网络 V1 API 与 MCP 契约
owner: backend
reviewed_at: 2026-08-03
status: accepted
source: docs/decisions/open-commerce-network-v1-architecture.md
---

# AI 原生开放商业网络 V1 API 与 MCP 契约

## 协议标识

- HTTP schema：`open_commerce.v1`
- MCP server：`yilong-open-commerce`
- MCP protocol：`2025-03-26`
- 金额：整数微单位，禁止浮点金额
- 时间：UTC RFC 3339
- ID：服务端生成的带前缀不透明字符串

## 项目管理 API

所有项目管理接口都需要一龙 Bearer token，并校验项目成员关系。

| 方法 | 路径 | 用途 |
|---|---|---|
| `GET` | `/api/projects/:project_id/open-commerce/overview` | 节点、能力、授权、数据接入、同步、调用和计量概览 |
| `GET` | `/api/projects/:project_id/open-commerce/development-context` | 供 AI 开发代理读取的脱敏能力与接入上下文 |
| `POST` | `/api/projects/:project_id/open-commerce/merchants` | 创建商户节点 |
| `PATCH` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id` | 更新或停用商户节点 |
| `POST` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/capabilities` | 创建商业能力 |
| `PATCH` | `/api/projects/:project_id/open-commerce/capabilities/:capability_id` | 更新或停用能力 |
| `PUT` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/runtime` | 配置受控商户运行绑定，不接收明文密钥 |
| `POST` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/runtime/verify` | 执行签名健康检查并核对 Manifest |
| `GET` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/business-evidence` | 读取商户终态能力调用证据、结果摘要和可选业务回执 |
| `GET` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/business-evidence/:invocation_id` | 读取单条商户业务证据及当时结果 |
| `POST` | `/api/projects/:project_id/open-commerce/business-handoff-receipts` | 项目编辑者明确记录 ERP/CRM 对指定业务证据的处理结果 |
| `GET` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/business-handoff-receipts` | 读取指定商户的业务衔接回执 |
| `GET` | `/api/projects/:project_id/open-commerce/merchants/:merchant_id/business-handoff-queue` | 读取由业务证据和最新回执派生的待处理或需重试队列 |
| `POST` | `/api/projects/:project_id/open-commerce/grants` | 创建调用授权 |
| `POST` | `/api/projects/:project_id/open-commerce/grants/:grant_id/revoke` | 撤销授权 |
| `GET` | `/api/projects/:project_id/open-commerce/audit` | 读取项目审计与调用记录 |
| `POST` | `/api/projects/:project_id/open-commerce/integrations` | 登记商户数据来源、授权范围和数据域 |
| `PATCH` | `/api/projects/:project_id/open-commerce/integrations/:integration_id/enabled` | 停用或重新启用数据接入 |
| `GET` | `/api/projects/:project_id/open-commerce/adapter-credentials` | 列出不含明文 Token 的接入器机器凭据元数据 |
| `POST` | `/api/projects/:project_id/open-commerce/integrations/:integration_id/adapter-credential/rotate` | 经明确确认并指定 1–366 天有效期后签发或轮换一次性机器 Token |
| `POST` | `/api/projects/:project_id/open-commerce/adapter-credentials/:credential_id/revoke` | 经明确确认后撤销机器凭据 |
| `GET` | `/api/projects/:project_id/open-commerce/adapter-handoff-claims` | 项目成员查看最近接入器任务租约，不返回密钥 |
| `POST` | `/api/projects/:project_id/open-commerce/adapter-handoff-claims/:claim_id/resume` | 项目编辑者明确确认后重新排队第 6 次拒绝而暂停的当前任务 |
| `POST` | `/api/projects/:project_id/open-commerce/sync-receipts` | 由适配器记录幂等同步或健康检查回执 |
| `PUT` | `/api/projects/:project_id/open-commerce/rate-limits` | 按能力和指定 App/全部 App 创建或更新调用配额 |
| `PATCH` | `/api/projects/:project_id/open-commerce/rate-limits/:policy_id/enabled` | 停用或重新启用调用配额 |
| `GET/PUT` | `/api/projects/:project_id/open-commerce/app-blocks` | 列出记录或封禁已注册开发者 App |
| `POST` | `/api/projects/:project_id/open-commerce/app-blocks/:block_id/unblock` | 解除封禁；不会恢复旧授权 |

## 发现与调用 API

| 方法 | 路径 | 用途 |
|---|---|---|
| `GET` | `/api/open-commerce/merchants` | 按文本和能力发现启用的商户 |
| `GET` | `/api/open-commerce/merchants/:merchant_id` | 读取商户公开资料和可发现能力 |
| `POST` | `/api/open-commerce/invoke` | 调用能力并记录幂等、计量和审计 |
| `POST` | `/api/open-commerce/action-confirmations` | 为当前用户和 App 的动作请求准备 5 分钟输入绑定确认 |
| `POST` | `/api/open-commerce/action-confirmations/:confirmation_id/confirm` | 独立确认待执行动作，不直接执行能力 |
| `POST` | `/api/open-commerce/developer/action-confirmations` | 使用开发者测试 Token 准备沙盒动作确认 |
| `POST` | `/api/open-commerce/developer/action-confirmations/:confirmation_id/confirm` | 使用同一测试 Token 确认沙盒动作 |
| `GET` | `/api/open-commerce/developer/events` | 使用测试 Token 按游标读取当前 App 的终态调用摘要 |
| `GET` | `/api/open-commerce/developer/events/:invocation_id` | 使用测试 Token 读取当前 App 的单条终态结果 |
| `GET` | `/api/open-commerce/consumer-invocation-receipts` | 当前账户列出本人的终态调用凭证摘要 |
| `GET` | `/api/open-commerce/consumer-invocation-receipts/:invocation_id` | 当前账户读取并复核本人的单条调用凭证 |
| `POST` | `/api/open-commerce/adapter/business-handoff-receipts` | 使用受限接入器 Bearer Token 提交机器衔接回执 |
| `POST` | `/api/open-commerce/adapter/business-handoff-claims` | 使用显式扩权的机器凭据领取一条短时待衔接任务 |
| `POST` | `/api/open-commerce/adapter/business-handoff-claims/:claim_id/complete` | 使用机器凭据与一次性租约密钥原子完成任务回执 |
| `POST` | `/api/open-commerce/adapter/business-handoff-claims/:claim_id/release` | 主动释放未完成租约并立即允许安全重试，不创建业务回执 |
| `POST` | `/api/open-commerce/adapter/business-handoff-claims/:claim_id/renew` | 使用当前租约密钥续租 60–900 秒，但不超过首次领取后 1 小时 |

两个 `GET` 发现接口允许匿名读取，便于任意 App 或 AI 在未加入一龙项目时发现公开商户能力。能力调用、项目管理和 MCP 仍需要 Bearer 身份；开放发现不等于匿名执行。

发现接口不返回授权数据、内部处理器配置、项目成员信息或商户内部数据。

## 调用请求

```json
{
  "merchant_id": "merchant_xxx",
  "capability_key": "store.profile.read",
  "requester_app_id": "pc-web",
  "grant_id": null,
  "idempotency_key": "demo-001",
  "input": {}
}
```

调用结果必须明确区分业务结果和资金状态：

```json
{
  "schema": "open_commerce.invocation.v1",
  "invocation_id": "invoke_xxx",
  "contract_validation": {
    "profile": "open_commerce.capability_schema.v1",
    "input_validated": true,
    "output_validated": true
  },
  "status": "succeeded",
  "result": {},
  "metering": {
    "units": 1,
    "unit_price_micros": 0,
    "amount_micros": 0,
    "currency": "CNY",
    "settlement_status": "recorded_not_charged"
  },
  "settlement_receipt": {
    "schema": "open_commerce.settlement_receipt.v1",
    "receipt_id": "invoke_xxx",
    "billable_units": 1,
    "amount_micros": 0,
    "currency": "CNY",
    "status": "recorded_not_charged",
    "funds_moved": false
  }
}
```

相同调用方、商户、能力和幂等键的重复调用不重复累计金额；历史成功结果仍满足能力当前输出契约时返回原结果，否则以 `422` 拒绝本次重放且不改写历史调用。

`action` 调用必须先向确认接口提交完全相同的商户、能力、Grant、幂等键和输入。服务端校验后返回 5 分钟有效的确认 ID；独立确认后，调用请求在顶层增加 `action_confirmation_id`。Invocation 创建和确认消费在同一事务中完成。已消费确认只允许重放所绑定的同一 Invocation，不能创建第二次动作。准备接口按精确请求幂等复用，因此动作成功但网络响应丢失后仍可通过同一幂等键恢复原确认和 Invocation；同键更换输入或 Grant 会失败。每个用户与 App 同时最多保留 20 份活动确认，创建超过 7 天且没有 Invocation 的过期确认会在后续准备时清理。`query` 不需要也不接受该字段。

能力创建和更新只接受平台可以实际执行的有限 Schema 配置。调用输入在 Invocation、限流和 Grant 预算预留前校验；处理器输出在成功计量前校验。输入违例返回 `422` 且不创建调用；输出违例保存为 `output_schema_violation` 零金额失败调用并释放预算。错误与审计只返回字段路径和规则代码，不返回业务值。当前不支持 `$ref`、组合或条件 Schema，也不把结构校验解释为业务真实性证明。

## 消费者调用凭证

消费者调用凭证是已有 Invocation 的账户级只读投影，不是第二套订单或结算记录。列表仅包含 `succeeded` 和 `failed` 的摘要，不返回商户结果；详情只允许调用发起账户读取，其他账户统一收到未找到。

凭证不返回用户 ID、项目 ID、能力内部 ID、Grant ID、幂等键、请求哈希或原始输入。请求侧只给出字段数量、序列化字节数和 `contains_raw_values=false`；本人详情可包含商户当时返回的结果。V1 只接受 `recorded_not_charged` 并明确标记未移动真实资金，遇到未来资金状态时失败关闭。

服务端返回规范 `payload_json` 和其 SHA-256。PC 下载前会重新计算摘要、解析该字符串并核对外层 `payload`；摘要只证明所下载字节与服务端响应一致，不是商户签名、外部时间戳、链上证明或支付凭证。现有 Invocation 没有消费者项目标识，因此 HTTP 和 MCP 都按当前登录账户读取跨项目历史。

## 开发者终态事件流

开发者 App 可使用自身测试 Token 轮询终态调用。列表按只增终态序号升序返回 `succeeded` 或 `failed` 摘要，默认 20 条、最多 100 条；游标绑定当前 App，不能跨 App 复用。列表不返回结果正文、原始输入、请求摘要、Grant 或内部身份，单条详情只允许同一 App 读取其原本已获得的结果。

客户端应持久保存每次响应的 `next_cursor`，处理完本页后再请求下一页。空轮询会保留原检查点；`has_more=true` 时应立即继续读取。该接口是可恢复的拉取事件流，不是 Webhook 或外部主动推送，也不证明真实支付、订单或履约完成。

## 商户业务调用证据

商户证据是终态 Invocation 的项目内只读投影。列表按终态序号返回调用状态、结果摘要、计量、可选标准业务回执和当前 ERP 实例关联；详情可返回商户当时的结果。证据不包含原始调用输入，也不会自动写入 ERP。

商户运行时可在结果中加入 `_yilong_business_receipt`，版本固定为 `open_commerce.merchant_business_receipt.v1`。新响应中的回执必须使用小写业务类型和状态、商户侧引用、RFC 3339 时间；金额采用非负整数最小货币单位，且必须与大写币种同时出现。平台不从任意业务 JSON 猜测订单字段。

Invocation 只证明平台完成调用，标准业务回执只证明商户运行时作出声明，真实订单、库存、财务、支付和履约仍以商户 ERP 为准。MCP 工具为 `open_commerce_list_merchant_business_evidence` 和 `open_commerce_get_merchant_business_evidence`。

## ERP/CRM 业务衔接回执

项目编辑者在真实接入器已经处理业务证据后，可显式记录 `applied`、`ignored` 或 `rejected`。请求必须绑定商户、Invocation、同商户接入器、幂等键和当前结果 SHA-256，并设置 `confirmed_by_user=true`。

`applied` 只接受成功且带有效标准业务回执的 Invocation，同时必须提供外部目标记录号；服务端只保存目标记录号 SHA-256。`ignored` 和 `rejected` 不能提供目标记录号，必须提供结果代码。停用接入器、跨商户接入器、摘要不匹配、非编辑者或同键改写均失败关闭。

人工入口的回执权威为 `project_editor_asserted`。接入器也可使用只显示一次、服务端仅保存 SHA-256 的专用 Bearer Token 向机器入口提交；项目、商户和接入器全部从凭据派生。凭据始终包含 `business_handoff.write`，任务领取权限 `business_handoff.claim` 默认关闭，只能在用户明确轮换时加入。机器回执标记 `adapter_token_authenticated`、`confirmed_by_user=false`，并固化凭据 ID 和提交时版本。轮换、撤销、到期或停用接入后旧身份立即失败关闭。

显式获得 `business_handoff.claim` 的接入器可调用领取入口。请求只接受 `lease_seconds`，范围 60–900 秒；响应最多包含一条由凭据边界派生的任务、一次性 `lease_token` 和该任务结果。服务端不提供任意商户数据读取。接入器可用同一租约密钥续租 60–900 秒，但 `lease_deadline_at` 在首次领取时固定为 1 小时后，不能无限延期。完成入口不接受客户端重写 Invocation、商户或接入器，只接受租约密钥和处理结果；回执与租约完成状态在同一数据库事务中提交，回执保存 `adapter_claim_id`。同一 Invocation 同时只有一个活动租约，超时、主动释放或 `rejected` 后方可产生下一次尝试。`rejected` 按尝试次数进入 30–900 秒退避，候选任务按最久未尝试优先；第 6 次拒绝后暂停自动领取，只有项目编辑者明确确认恢复当前最新尝试后才能继续。主动释放只接受租约密钥与受限原因代码，不创建回执，也不表示外部任务已经处理。

签发或轮换请求还必须提交 `expires_in_days`，允许 1–366。凭据元数据返回绝对 `expires_at` 与服务端派生的 `is_expired`；到期后鉴权失败且不自动续期。升级前已有凭据由迁移补 90 天期限。

两类回执均固定 `funds_moved=false`，不创建平台订单，也不证明外部 ERP 数据真实、支付、履约或退款。机器凭据不是外部平台签名；具体生产适配器、官方授权和外部系统回读仍需逐项实现。

项目内 AI 可通过 `open_commerce_list_adapter_handoff_claims` 读取与 PC 相同的脱敏租约状态；`open_commerce_resume_adapter_handoff_claim` 只恢复第 6 次拒绝后暂停的当前任务，要求项目编辑权限和 `confirmed_by_user=true`。机器领取、续租、完成和释放仍只走专用 Bearer HTTP 入口，不把机器 Token 暴露给项目 MCP。

## Grant 生命周期预算

创建 Grant 或批准授权申请时，可选设置 RFC 3339 格式的 `expires_at`、`max_invocations`、`max_amount_micros` 和 `budget_currency`。期限必须晚于当前服务器时间；未提供期限表示长期有效。PC 新授权默认 30 天，长期有效必须显式选择。返回值同时包含期限、`used_invocations` 和 `used_amount_micros`；批准后的授权申请还回读实际 Grant 条件，供商户与申请方核对。

预算在新调用进入处理器前原子预留，成功后确认，处理器失败时释放。幂等重放不重复占用。达到次数或金额上限的新调用记录为 `failed/grant_budget_exceeded`、单位与金额为 0，并返回 `403`。该金额只限制当前链外计量，不移动真实资金。

Grant 到期不删除或改写历史，也不会自动续期。消费者发现不再把它视为有效授权，调用必须重新申请新 Grant。

## 消费者关系凭证

消费者关系与商户到 App 的 Grant 是两类不同对象。Grant 控制 App 能否调用商户能力；消费者关系凭证只证明消费者允许指定商户在期限内把主动提供的偏好或会员标识关联到一个匿名关系标识，不授予读取商户私有数据的权限。

- `GET/POST /api/projects/:project_id/open-commerce/consumer-relationships`：当前用户读取或创建自己的关系凭证。
- `POST /api/projects/:project_id/open-commerce/consumer-relationships/:relationship_id/revoke`：当前用户幂等撤销自己的关系。
- `POST /api/projects/:project_id/open-commerce/consumer-relationships/:relationship_id/renew`：当前用户安全续期；撤销旧关系、轮换匿名标识并返回幂等后继。
- `GET /api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-relationships`：商户项目读取指向本商户的脱敏关系历史。

创建请求包含 `merchant_id`、`source_app_id`、固定 `scopes`、`purpose` 和 RFC 3339 `expires_at`。关系只能指向已发布商户，期限必须在未来且不超过 366 天。商户响应不含消费者账号、用户 ID 或消费者项目 ID；重新创建会撤销旧关系并生成新的 `subject_alias`。续期请求包含 `source_app_id` 和新的 `expires_at`，继承原范围和用途；同一来源关系重复续期返回同一后继。内部续期链不进入公开响应。

MCP 对应工具为 `open_commerce_list_consumer_relationships`、`open_commerce_list_merchant_relationships`、`open_commerce_create_consumer_relationship`、`open_commerce_revoke_consumer_relationship` 和 `open_commerce_renew_consumer_relationship`。MCP 来源身份由入口绑定，不能通过参数冒充其他 App。

## 消费者偏好档案与关系级披露

偏好档案属于当前项目中的当前用户，保存档案不自动授权商户，也不自动改变发现请求。V1 只接受类别、标签、城市、单位调用价格上限和公开能力偏好；不接受自由文本和敏感身份资料。

- `GET/PUT/DELETE /api/projects/:project_id/open-commerce/consumer-preference-profile`：读取、保存或删除本人档案。删除同时移除本人在该项目生成的披露快照。
- `GET /api/projects/:project_id/open-commerce/consumer-preference-disclosures`：读取本人披露历史，包括关系已经失效的审计视图。
- `GET/PUT/DELETE /api/projects/:project_id/open-commerce/consumer-relationships/:relationship_id/preference-disclosure`：读取、更新或撤回本人指定关系的披露快照。
- `GET /api/projects/:project_id/open-commerce/merchants/:merchant_id/preference-disclosures`：商户项目只读取仍有效关系的匿名披露。

披露请求使用固定 `shared_fields` 白名单，只能绑定有效且含 `preference.remember` 的关系。返回值包含匿名 `subject_alias`、字段快照和来源档案修订号，不含消费者账号、用户 ID 或消费者项目。档案后续更新不自动同步，关系撤销、到期、删除请求或续期都会使旧披露对商户失败关闭。

MCP 提供档案读取、保存、删除，本人披露列表、单关系披露读取/更新/撤回，以及商户有效披露列表工具。工具定义集中在 `server/src/open_commerce_consumer_preference_mcp.rs`，共用同一领域服务。当前低敏结构化档案、历史披露和账户级终态调用凭证可进入消费者本人 V3 数据包，但这仍不是端到端加密或跨运营方消费者数据保险箱。

## 消费者关联数据删除请求

删除请求与关系撤销也是不同对象：撤销关系停止未来授权，删除请求则要求商户处理此前按该匿名关系关联的数据。创建删除请求会在同一事务内撤销关系，但平台不保存待删除的数据，也不能仅凭请求状态证明商户外部系统已经删除数据。

- `GET/POST /api/projects/:project_id/open-commerce/consumer-data-requests`：当前用户读取或针对本人关系创建删除请求。
- `POST /api/projects/:project_id/open-commerce/consumer-data-requests/:request_id/withdraw`：在商户接单前撤回；不恢复关系。
- `GET /api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-data-requests`：商户项目读取指向本商户的匿名请求。
- `POST /api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-data-requests/:request_id/decision`：商户编辑者执行 `accept`、`complete` 或 `reject`。

状态机为 `requested -> in_progress -> completed/rejected`，消费者可把尚未接单的 `requested` 改为 `withdrawn`。完成和拒绝必须填写说明；`completed` 的 `resolution_kind` 固定为 `merchant_attested_completed`，只代表商户声明。响应不含消费者账号、用户 ID 或消费者项目 ID。

MCP 对应工具为 `open_commerce_list_consumer_data_requests`、`open_commerce_list_merchant_data_requests`、`open_commerce_create_consumer_data_erasure_request`、`open_commerce_withdraw_consumer_data_request` 和 `open_commerce_decide_consumer_data_request`。

## 消费者可携带数据包

可携带数据包是当前用户拥有的不可变快照，不是商户公开目录，也不是完整账户备份。V3 包含关系历史、消费者私有续期链、删除请求回执、当前低敏结构化偏好档案、历史披露快照和本人账户级终态调用凭证。调用凭证可包含消费者已收到的商户结果，但不包含原始输入、内部 Grant 或调用幂等键，也不是商户数据库完整订单、支付或履约证明。披露记录是消费者当时主动固化的快照，即使关系后来失效仍可由本人导出，但商户实时读取仍按关系状态失败关闭。

- `GET/POST /api/projects/:project_id/open-commerce/consumer-portability-exports`：列出当前用户的数据包摘要，或用幂等键创建新快照。
- `GET /api/projects/:project_id/open-commerce/consumer-portability-exports/:export_id`：读取当前用户拥有的数据包，重新校验总包及每条调用凭证的 SHA-256。

创建参数为 `idempotency_key`。同一用户在同一项目重复使用相同键时永远返回首次快照；要反映后续关系或请求变化必须使用新键。负载最多 5 MiB，关系和请求各最多 5000 条，超限不会截断。

MCP 对应工具为 `open_commerce_list_consumer_portability_exports`、`open_commerce_get_consumer_portability_export` 和 `open_commerce_create_consumer_portability_export`。V2 新包使用新版本标识；读取同时接受原 V1 包，并通过缺省字段不参与重新序列化保持旧 SHA-256。当前仍只导出，不提供导入、跨运营方迁移、加密归档或敏感偏好保险箱。

## 调用配额

商户可以为每项能力配置固定时间窗调用上限。指定 App 策略优先于全部 App 策略；全部 App 策略按调用主体分别计数。没有策略时保持现有允许行为，项目编辑者在本项目内调试不占额度。

幂等重放不重复占用限流配额，但历史成功结果必须先通过能力当前输出契约；契约变化导致旧结果不再匹配时返回 `422`，原始历史调用状态和金额不被改写。超过配额的新调用不会进入处理器，记录为 `failed/rate_limited`，单位和金额均为 0，并返回 `429` 与重试时间。项目总览同时返回 `rate_limit_policies` 和当前时间窗 `rate_limit_usage`。

当前计数持久化在一龙主数据库中，适用于共用该数据库的服务实例；它不等于跨数据库、跨地域的全网限流。

## App 紧急封禁

商户项目编辑者可手动封禁具体的已注册 App。封禁在同一事务内激活记录、撤销该商户授予该 App 的有效 Grant，并取消待审批授权申请。被封 App 不能继续调用公开或受限能力，也不能提交新申请。解除封禁不会恢复旧 Grant；受限能力必须重新申请授权。

## MCP 工具

| 工具 | 读写 | 用途 |
|---|---|---|
| `open_commerce_get_overview` | 读 | 查看当前项目网络状态 |
| `open_commerce_get_development_context` | 读 | 获取不含密钥和原始经营数据的开发上下文 |
| `open_commerce_search_merchants` | 读 | 按文本或能力发现商户 |
| `open_commerce_discover_for_consumer` | 读 | 按消费者偏好和硬约束透明发现商户能力，并返回候选范围、来源和授权状态；不自动调用或下单 |
| `open_commerce_plan_consumer_capability` | 读 | 校验拟调用输入并返回调用、注册 App、授权申请、等待审批或动作确认的下一步；不创建副作用 |
| `open_commerce_get_merchant` | 读 | 获取单个商户与公开能力 |
| `open_commerce_create_merchant` | 写 | 创建商户节点 |
| `open_commerce_publish_capability` | 写 | 发布受控能力 |
| `open_commerce_upsert_runtime` | 写 | 配置商户运行绑定的地址、服务端凭据引用和 Manifest 摘要 |
| `open_commerce_verify_runtime` | 写 | 通过签名健康检查激活运行绑定 |
| `open_commerce_create_grant` | 写 | 为 App 创建最小范围授权，可附加总调用与总计量预算 |
| `open_commerce_upsert_rate_limit` | 写 | 按商户能力和 App 创建或更新固定时间窗配额 |
| `open_commerce_set_rate_limit_enabled` | 写 | 停用或重新启用调用配额 |
| `open_commerce_list_app_blocks` | 读 | 查看当前项目的 App 封禁与解除记录 |
| `open_commerce_block_app` | 写 | 封禁 App，并撤销授权、取消待审批申请 |
| `open_commerce_unblock_app` | 写 | 解除封禁但不恢复旧授权 |
| `open_commerce_create_integration` | 写 | 登记商户数据来源 |
| `open_commerce_set_integration_enabled` | 写 | 停用或重新启用接入 |
| `open_commerce_record_sync_receipt` | 写 | 记录有界、幂等的适配器回执 |
| `open_commerce_revoke_grant` | 写 | 撤销授权 |
| `open_commerce_prepare_action_confirmation` | 写 | 校验并准备或幂等复用短时、输入绑定的动作确认，不执行能力 |
| `open_commerce_confirm_action_confirmation` | 写 | 仅在用户已明确同意后确认准备结果，不执行能力 |
| `open_commerce_invoke` | 写 | 调用能力并生成计量和审计；动作必须携带已确认 ID |
| `open_commerce_list_my_invocation_receipts` | 读 | 按当前账户列出本人终态调用凭证摘要 |
| `open_commerce_get_my_invocation_receipt` | 读 | 按当前账户读取并复核本人单条调用凭证 |
| `open_commerce_list_merchant_business_evidence` | 读 | 按当前项目列出指定商户的终态调用证据和可选业务回执 |
| `open_commerce_get_merchant_business_evidence` | 读 | 读取单条商户业务证据和当时结果，不自动写入 ERP |
| `open_commerce_list_business_handoff_receipts` | 读 | 读取指定商户的 ERP/CRM 显式衔接回执 |
| `open_commerce_list_business_handoff_queue` | 读 | 读取指定商户待处理或需重试的 ERP/CRM 衔接证据 |
| `open_commerce_record_business_handoff_receipt` | 写 | 用户确认后幂等记录接入器对业务证据的处理结果，不移动资金 |
| `open_commerce_list_audit` | 读 | 查看调用与治理证据 |

MCP 写工具遵循与 HTTP API 相同的项目角色、授权、动作确认和幂等规则。MCP 不提供绕过确认的真实资金、发布或外部系统写操作。

## 数据接入与同步回执

数据接入记录只包含平台标识、接入方式、授权范围、数据域和健康状态，不接受凭据、Cookie 或任意远程 URL。`configured` 表示已登记但尚无成功证据；`connected` 和 `degraded` 由同步回执驱动；停用后拒绝新回执。

同步回执以 `integration_id + receipt_key` 幂等。同键不同结果返回冲突；回执只包含记录数量、游标摘要、错误代码和时间，不包含原始订单、客户、财务或库存值。

## 受控处理器

V1 允许：

- `merchant_profile`：返回商户公开资料；
- `static_json`：返回项目编辑者配置的静态演示数据；
- `merchant_runtime`：把调用转交给当前商户已验证的运行绑定。

V1 仍拒绝未知处理器，也不允许在能力配置中填写任意 URL 或密钥。`merchant_runtime` 的地址和凭据引用独立保存；生产地址必须使用 HTTPS 并命中 `OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS`，密钥只从服务端环境变量解析。平台使用 HMAC-SHA256 对时间戳和原始 JSON 请求体签名，签名健康检查核对商户身份与 Manifest 摘要，失败时绑定进入降级状态。详细契约见 `docs/open-commerce/merchant-runtime.md`。

## 错误语义

| HTTP | 含义 |
|---|---|
| `400` | 输入、schema、幂等键或状态不合法 |
| `401` | 未登录或 token 无效 |
| `403` | 不是项目成员、角色不足、授权不匹配、Grant 预算用尽或 App 已被商户封禁 |
| `404` | 商户、能力、授权或项目不存在 |
| `409` | slug、能力键、幂等键语义冲突 |
| `422` | 当前输入、处理器输出或历史重放结果不满足能力契约 |
| `429` | 商户配置的能力调用配额已达到上限 |

错误响应不得包含 token、密钥、内部处理器配置或原始敏感输入。
