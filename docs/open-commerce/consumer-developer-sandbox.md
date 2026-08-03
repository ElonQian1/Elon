# 消费者发现与第三方应用沙盒

本文说明消费者发现、应用注册、授权审批和开发者调用闭环。沙盒总边界由 `docs/decisions/open-commerce-consumer-developer-sandbox-v1.md` 决定；目录、候选范围、发现输入、排序、公开数据来源与新鲜度、内部同步回执来源绑定、声明期筛选、内部回执来源、厂商数据域、最大回执年龄、筛选建议、币种安全价格过滤、能力筛选与偏好硬约束、关系、偏好、删除请求、跟进与证明分别由 `docs/decisions/open-commerce-directory-publication-v1.md`、`docs/decisions/open-commerce-consumer-candidate-scope-v1.md`、`docs/decisions/open-commerce-consumer-discovery-inputs-v1.md`、`docs/decisions/open-commerce-pluggable-ranking-v1.md`、`docs/decisions/open-commerce-public-data-provenance-v1.md`、`docs/decisions/open-commerce-capability-source-link-v1.md`、`docs/decisions/open-commerce-consumer-freshness-filter-v1.md`、`docs/decisions/open-commerce-consumer-source-requirement-v1.md`、`docs/decisions/open-commerce-consumer-source-filters-v1.md`、`docs/decisions/open-commerce-consumer-source-age-v1.md`、`docs/decisions/open-commerce-consumer-source-filter-options-v1.md`、`docs/decisions/open-commerce-consumer-price-currency-v1.md`、`docs/decisions/open-commerce-consumer-capability-filters-v1.md`、`docs/decisions/open-commerce-consumer-preference-constraints-v1.md`、`docs/decisions/open-commerce-consumer-relationships-v1.md`、`docs/decisions/open-commerce-consumer-preference-disclosures-v1.md`、`docs/decisions/open-commerce-consumer-data-erasure-requests-v1.md`、`docs/decisions/open-commerce-consumer-data-request-followups-v1.md` 和 `docs/decisions/open-commerce-consumer-data-erasure-evidence-v1.md` 决定；当前本人导出由 `docs/decisions/open-commerce-consumer-portability-exports-v5.md` 决定。应用生命周期、动作确认、授权预算、调用凭证、终态事件和活动证据继续按 `docs/open-commerce/README.md` 所列专项 ADR 执行。

## 使用入口

项目详情的“开放商业”区域包含：

- **消费者沙盒**：选择请求 App，按搜索词、能力、城市、标签和价格上限发现商户能力，选择透明排序规则，再按商户发布的输入契约填写并调用。
- **本人调用凭证**：按账户查看跨项目终态调用摘要，读取并下载经过 SHA-256 复核的单条结果。
- **开发者**：注册沙盒 App、轮换一次性测试 Token、处理授权申请、调试能力调用并按游标取回本 App 的终态结果。
- **商户工作台**：查看外部 App 近 24 小时的成功、失败、限流、授权预算拒绝和中断恢复证据，再人工决定是否封禁。

发现结果必须展示是否存在付费排序。当前代码提供偏好匹配、最低调用价、公开能力优先、最近更新和商户名称五种内置策略，均固定为非付费排序；消费者可显式选择，省略时使用偏好匹配。未知策略失败关闭，不能通过购买排名改变结果。排序只作用于目录查询返回的最多 100 个商户候选，最终最多返回 50 条，不代表全网穷举或客观唯一最优。普通响应会显示候选上限、目录候选数、合格数、返回数和截断状态，且固定声明非全网穷尽。每项能力还显示商户项目声明的来源和新鲜度；商户可把当前能力版本显式关联到合格的项目内部同步回执，此时新鲜度改按回执完成时间派生。来源始终未外部核验，消费者可独立要求只看声明期内能力或只看内部回执来源能力，也可按一个厂商标识、一个数据域和最长内部回执年龄精确筛选；PC 会从当前候选窗口提供非穷尽建议，但建议不保证最终命中。无效或未来回执时间失败关闭，默认不替用户隐藏其他商户声明。价格条件先匹配三位币种代码再比较微单位，不能跨币种比较；能力条件可限定查询/动作与公开/授权，但不改变后续授权和动作确认。城市、类别和标签默认仍是软偏好，只有消费者显式勾选后才成为硬约束。消费者可显式生成单次排序凭证；搜索词、偏好和来源过滤只进入输入摘要，PC 复核负载摘要后下载。凭证固定未签名，不证明运营方身份、数据真实性、外部平台背书或算法公平。

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
| 绑定或移除能力内部来源回执 | `PUT/DELETE /api/projects/{project_id}/open-commerce/capabilities/{capability_id}/source-link` |
| 设置或停用商户调用配额 | `PUT .../rate-limits`、`PATCH .../rate-limits/{policy_id}/enabled` |
| 列出、封禁或解除开发者 App | `GET/PUT .../app-blocks`、`POST .../app-blocks/{block_id}/unblock` |
| 消费者发现 | `POST /api/open-commerce/sandbox/discover` |
| 提交授权申请 | `POST /api/open-commerce/authorization-requests` |
| 使用测试 Token 调用 | `POST /api/open-commerce/developer/invoke` |
| 轮询本 App 终态事件 | `GET /api/open-commerce/developer/events?cursor=&limit=` |
| 读取本 App 单条终态结果 | `GET /api/open-commerce/developer/events/{invocation_id}` |
| 列出本人调用凭证 | `GET /api/open-commerce/consumer-invocation-receipts` |
| 读取本人单条调用凭证 | `GET /api/open-commerce/consumer-invocation-receipts/{invocation_id}` |

MCP 消费者 AI 可使用 `open_commerce_discover_for_consumer` 获取与上述消费者发现同源的排序、来源、筛选、候选范围和授权状态。默认 MCP 身份只按 `pc-web` 做公开发现；显式 `x-elon-app-id` 必须属于当前用户。该工具只读，不会自动申请授权、调用能力、创建订单或结算。

需要选择独立 App 身份时，AI 可先调用 `open_commerce_list_my_consumer_apps`。该工具只列当前项目中本人拥有的 App，并明确当前 MCP 身份；响应不包含测试 Token、Token 提示或生产凭据。创建 App 和管理 Token 仍在开发者门户完成。

选择能力后，AI 可把实际拟调用输入交给 `open_commerce_plan_consumer_capability`。计划只读返回调用、注册 App、申请授权、等待审批或动作确认的有序步骤；动作能力固定要求准备、用户明确同意、确认和调用。计划不会占用 Grant 次数或预算，也不会创建确认、调用、订单或结算记录。

计划会用能力当前单位价格和币种检查该 App 的全部有效 Grant，优先选择任意能覆盖下一次调用的一张。若全部授权的次数、金额或币种均不满足，发现页显示“授权额度不足”，AI 返回 `grant_refresh_required` 和具体 `grant_budget_status`；用户可提交新的最小授权申请。该过程不会自动扩容或撤销旧 Grant，新额度仍须商户批准，真正调用时还会在事务内再次预留预算。

动作确认准备后，AI 可在用户同意前调用 `open_commerce_get_my_action_confirmation` 重新核对当前用户和当前 App 持有的确认状态、商户、能力、输入字段形状、失效时间和下一步。服务端不会返回原始输入值，因此 AI 或宿主仍必须从当前安全会话向用户展示本次真实参数；读取本身不会确认、续期、占用预算或执行动作。

计划要求授权时，消费者 AI 只有在用户明确同意、MCP 使用本人已注册 App 身份并提供固定确认短语后，才能用 `open_commerce_request_consumer_authorization` 提交单能力申请。成功只表示等待商户决定；批准、拒绝、期限、总次数和总预算仍由商户控制，AI 不能自批或扩权。

提交后可用 `open_commerce_list_my_consumer_authorization_requests` 按状态查看本人申请和批准后的 Grant 条件。查询同时按当前用户和当前项目本人 App 隔离，不返回团队成员或审核人内部身份；它只读，不会重提或调用。项目编辑者在用户明确同意并提供固定确认短语后，可用 `open_commerce_cancel_my_consumer_authorization_request` 撤回本人 pending 请求；非 pending 保持原状态，该工具不会撤销已批准 Grant。历史 `approved` 也不保证 Grant 仍有效，实际调用前仍需重新执行计划。

需要判断当前还能使用哪些授权时，AI 应调用 `open_commerce_list_my_active_grants`，而不是只查看历史申请。该工具只列当前项目中本人 App 仍有效的 Grant，并返回期限、已用和剩余调用次数、已用和剩余计量预算；预算耗尽会明确标记。它不预留预算、不调用能力，也不代替 `open_commerce_plan_consumer_capability` 对实际输入和当前封禁状态的最终复核。

开发者调用和事件读取只使用 `Authorization: Bearer <test-token>`，App 身份由 Token 唯一确定，不能用额外请求头切换。Token 不得进入 URL、日志、项目文档、浏览器本地存储或商户能力元数据。

批准申请时，商户可选择 7 天、30 天、90 天、1 年或长期有效，并可填写授权期内的总调用次数和总预算（人民币元）。PC 默认 30 天；长期有效必须显式选择。用尽或到期后不能由 App 自行扩容、续期，商户需要重新授权。调用失败会退回刚预留的预算，重复请求不会再次占用。批准后的实际期限和预算会同时展示给商户与申请方。

消费者还可以在发现商户后建立独立的关系凭证。关系凭证不等同于 App Grant：它只允许商户把消费者主动提供的偏好或会员标识关联到随机匿名标识。PC 默认 90 天、最长 366 天，消费者可随时撤销；到期前 14 天显示续期入口。续期会撤销旧凭证并轮换匿名标识，相同请求只返回同一个新凭证。商户看不到消费者账号、用户 ID、消费者项目 ID 或内部续期链。关系凭证不存放偏好原文、联系方式、订单和支付数据。

消费者可另行保存类别、标签、城市和价格上限等低敏结构化偏好。保存不会自动向商户披露，也不会自动改变发现请求；用户可显式带入一次发现，或针对有效且含 `preference.remember` 的关系选择字段生成快照。商户只看到仍有效关系的匿名快照；档案更新不自动同步，关系撤销、到期或续期后旧披露立即不可见。该能力不保存自由文本或敏感身份资料，也不等于完整数据保险箱。

敏感自由文本由独立的数据保险箱处理。PC 在本机使用用户口令加密，服务端只托管不透明密文及标签、类型、大小、摘要和修订等最小元数据；列表不返回密文，只有本人可读取单项密文并在本机解锁。平台没有口令或找回能力，保险箱内容不会自动用于发现、披露、商户调用或 AI 执行。

消费者还可针对本人关系发起关联数据删除请求。创建请求会原子撤销该关系；商户只能看到匿名关系别名，可接单、拒绝或声明完成。消费者可在接单前撤回请求，但关系不会恢复。跟进代码按 7 天内部运营目标，在 24 小时后允许首次催办、每 24 小时一次且最多三次，超时并至少催办一次后可升级关注；该批尚未编译，且不会自动通知第三方、仲裁或处罚。`completed` 只表示商户提交了可审计声明。完成后，商户可按外部系统追加回执编号、原始回执 SHA-256 和摘要，消费者可查看；这些记录固定标记为商户提供且平台未核验，仍不能证明美团、ERP、CRM 或会员系统真实完成删除。

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

当前已形成商户主动发布目录、候选范围摘要、服务端发现输入规范化、商户声明来源与新鲜度、能力与内部同步回执来源绑定、消费者主动声明期筛选、内部回执来源筛选、厂商数据域与最大回执年龄过滤、非穷尽来源筛选建议、币种安全价格过滤、能力类型与访问级别筛选、偏好硬约束、五种用户可选透明非付费排序器、可复核未签名单次排序凭证、限时授权、消费者可撤销关系、低敏偏好字段披露、客户端加密数据保险箱、匿名删除请求、有界催办与升级关注、商户未核验证明、含证明与调用凭证的本人 V5 可验证导出、隔离导入、Schema 驱动填写、短时动作确认、持久化调用配额、活动证据、商户级手动 App 封禁和沙盒 App 生命周期闭环。输入规范化、来源绑定、新鲜度、来源过滤、排序器、凭证、跟进、保险箱与 V5 等新增批次尚未编译或回归；外部平台签名与官方回读证明、运营方签名凭证、第三方排序器 SDK、生产应用审核、完整订单迁移、自动外部通知、平台仲裁、商户或 AI 授权解密、真实删除适配器、支付和真实平台适配器仍是后续模块。
