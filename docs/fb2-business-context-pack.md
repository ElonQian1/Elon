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
