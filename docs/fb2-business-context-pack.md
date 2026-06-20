# fb2 Business Context Pack

fb2 是主项目的外部子项目。主项目负责 AI 调度、聊天体验、计费和 prompt 注入；fb2 负责提供可审计、可权限裁剪、可逐步索引的业务上下文。

## 协作结论

第一阶段不做 MCP。先用服务令牌保护的 HTTP Context Pack，主项目把它转成 `external_app_context` 注入群聊 AI、选中消息 AI 回复和群聊总结帖。

长期方向是：

```text
fb2 业务数据
  -> fb2 领域索引/摘要
  -> /api/main-project/context/pack
  -> 主项目 Context Pack 注入
  -> 后续按需升级为 MCP/tools
```

## 主项目职责

- 识别群聊是否属于 fb2 外部群。
- 根据当前主项目用户映射 fb2 `external_user_id`。
- 优先请求 `GET /api/main-project/context/pack`。
- pack 不可用时回退 `GET /api/main-project/context/today-matches`。
- 控制 token budget、超时、失败降级和 AI 输出安全边界。
- 将 fb2 返回的大 JSON 投影成模型友好的 XML-wrapped Markdown，不把重复原始数据无脑塞进 prompt。
- 记录上下文来源、状态、字符量、是否回退、是否包含用户订单，便于后续优化慢查询和答非所问。
- 只有 AI 生成回复扣额度；ASR、TTS、上下文拉取不扣 token。

## fb2 职责

- 不让主项目直接读 fb2 数据库。
- 返回稳定 JSON contract，并包含模型可读的 `context_pack`。
- 按用户权限裁剪订单，只返回当前用户自己的票。
- 平台订单只以匿名聚合形式进入普通群聊上下文。
- 群讨论观点必须带消息 ID，便于审计和复盘。
- 后续维护比赛、订单、群观点领域索引，减少全表扫描和联网搜索。

## 当前主项目调用

主项目通过环境变量配置 fb2：

```text
ELON_EXTERNAL_APP_FB2_BASE_URL
ELON_EXTERNAL_APP_FB2_CONTEXT_TOKEN
ELON_EXTERNAL_APP_FB2_CONTEXT_PACK_ENABLED=true
ELON_EXTERNAL_APP_FB2_MATCH_CONTEXT_LIMIT=30
ELON_EXTERNAL_APP_FB2_DISCUSSION_CONTEXT_LIMIT=80
ELON_EXTERNAL_APP_FB2_ORDER_CONTEXT_LIMIT=20
ELON_EXTERNAL_APP_FB2_PLATFORM_ORDER_CONTEXT=false
ELON_EXTERNAL_APP_CONTEXT_MAX_CHARS=16000
```

主项目请求：

```http
GET /api/main-project/context/pack
X-FB2-AI-CENTER-TOKEN: <shared-secret>
```

常用 query：

```text
group_id=official
external_user_id=<fb2-user-id-if-linked>
topic_hint=<user-question-or-summary-topic>
limit=30
discussion_limit=80
order_limit=20
lottery_type=JingCai|BeiDan
include_platform_orders=true|false
```

`include_platform_orders` 默认不开启，避免普通聊天无意拉取平台级经营数据。

## 主项目 Context Budget

主项目收到 fb2 上下文后会先做预算裁剪：

- 默认最大上下文字符数：`16000`，可用 `ELON_EXTERNAL_APP_CONTEXT_MAX_CHARS` 调整。
- 超预算时优先裁剪大数组：`group_messages`、`matches`、`user_orders`。
- `context_pack` 过长时截断，并提示后续可通过工具接口继续查询细节。
- 裁剪结果写入 `_context_budget`，包含 `before_chars`、`after_chars`、`trimmed`。

这一步是长期主义的基础：后续 fb2 能提供越来越多数据，但主项目不会让 prompt 无限膨胀。

## 主项目 Contract 归一化

主项目不会把 fb2 原始响应直接交给模型，而是先归一化为内部 `external_app_context`：

- `/context/pack` 归一化为 `fb2.context_pack.v1`。
- `/context/today-matches` 回退数据归一化为 `fb2.today_matches.v1`。
- 自动补充 `usage_policy`，明确 ASR/TTS/context fetch 免费，AI 回复扣额度。
- 自动生成 `context_quality`，记录缺失字段和数据新鲜度风险。

当前 `context_quality.warnings` 可能包含：

- `missing_generated_at`：fb2 没有返回生成时间，AI 必须提示数据新鲜度不足。
- `missing_context_pack`：fb2 没有返回模型友好的 Markdown/XML 包，主项目只能退回结构化 JSON。
- `missing_context_pack_version`：fb2 没有声明 pack 版本，后续 contract 演进难以追踪。
- `empty_matches`：本次上下文没有比赛数据，AI 不能假设今日有可分析比赛。

这使 fb2 可以渐进式接入：接口先可用，再通过 warnings 不断补齐数据质量，而不是让模型静默使用残缺上下文。

## 主项目 Prompt 投影

群聊 AI 不直接读取完整原始 JSON，而是优先读取：

```text
context_pack
usage_policy
context_quality
_context_budget
source/status/generated_at
```

并在 prompt 中强制：

- 区分「数据事实」「群友观点」「AI推断」。
- 涉及比赛预测时说明不确定性，不承诺命中，不诱导投注。
- 引用比赛尽量带 `match id`。
- 引用订单/票据尽量带 `order id` 或 `ticket id`。
- 引用群友观点必须带 `message id`。
- 上下文缺少来源或更新时间时，必须说明信息不足，不能编造。
- `context_quality.warnings` 非空时，必须在回答中说明相关数据缺口或新鲜度风险。

群聊总结帖仍会把预算后的 `external_app_context` 放进 Context Pack，方便总结帖保留可审计源数据。

## 主项目观测日志

每次拉取 fb2 上下文会记录：

- `app_id`
- `group_id`
- `external_group_id`
- `user_id`
- `status`
- `source`
- `context_chars`
- `has_external_user_id`

后续 P5 评测会在此基础上扩展为结构化指标：延迟、回退原因、引用命中率、过期数据次数、权限拒绝次数。

## 返回要求

fb2 返回外层：

```json
{
  "success": true,
  "data": {
    "context_pack_version": "fb2-chat-pack-v1",
    "generated_at": "2026-06-20T12:00:00+08:00",
    "context_pack": "<fb2_context_pack>...</fb2_context_pack>",
    "matches": [],
    "user_orders": [],
    "group_messages": [],
    "platform_order_summary": {},
    "tool_contract": {},
    "usage_policy": {}
  }
}
```

`context_pack` 推荐使用 XML-wrapped Markdown：

```xml
<fb2_context_pack version="1.0" project="fb2">

## 使用边界

- 只作为比赛讨论和订单剖析参考。
- 不承诺命中，不诱导投注。
- 必须区分数据事实、群友观点和 AI 推断。

## 今日/近期比赛与赔率

...

## 当前用户订单/票据

...

## 群讨论观点

...

## 平台/店铺订单摘要

...

</fb2_context_pack>
```

## 分阶段演进

### P0: 可用业务包

- 比赛与赔率。
- 当前用户订单。
- 群讨论消息。
- 平台订单匿名摘要。
- 主项目优先消费 `/context/pack`，失败回退 `/today-matches`。

### P1: 工具化接口

fb2 增加：

- `search_matches`
- `get_match_detail`
- `search_user_orders`
- `get_order_detail`
- `search_group_opinions`

主项目根据用户问题选择具体工具，不再每次拉完整包。

### P2: 领域索引

fb2 建设：

- `ai_match_context_index`
- `ai_order_context_index`
- `ai_group_opinion_index`
- `ai_context_source_log`

目标是让 AI 快速召回有用信息，不每次扫表或联网搜索。

### P3: 观点记忆和复盘

- 记录用户观点、群体分歧、AI 采纳记录。
- 赛后复盘观点质量。
- 所有观点必须引用源消息 ID。

### P4: RAG / 向量检索

向量检索只作为候选召回，最终仍要回查原始数据和 source id。Context Pack 只放精选证据，不放全量向量命中文本。

### P5: 评测闭环

持续记录：

- context pack 生成耗时。
- 返回源数据数量。
- token/字符长度。
- 回答是否引用真实源。
- 过期赔率/比赛数据次数。
- 权限拦截次数。

## 两个 Codex 会话如何沟通

fb2 会话每新增数据能力，先更新 fb2 文档和接口示例，再把能力名称、路径、字段、权限、失败行为发给主项目会话。

主项目会话只依赖 `/api/main-project/context/*` contract，不耦合 fb2 内部表名。主项目如需更细粒度检索，先给出 tool schema，由 fb2 在同一接口族下实现。
