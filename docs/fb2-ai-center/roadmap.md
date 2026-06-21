# fb2 AI Center 路线图

## P0：先让链路稳定

目标：fb2 用户可以用主项目账号体系、默认群聊、语音输入和 AI 回复。

主项目：

- 保持 `chat-bootstrap` 和 `context-contract` 稳定。
- 确保 ASR/TTS 免费，AI 回复才检查额度。
- 确保 fb2 用户首次会话能拿到试用额度。
- 维护 `android/chat-voice-kit` 的完整输入栏和语音兜底链路。

fb2：

- 后端创建主项目会话。
- 客户端进入默认群。
- 语音输入使用 `VoiceComposerView`。
- 群消息能触发主项目 AI 回复。

验收：

- 同一台手机上，fb2 能完成文本聊天、按住说话、转文字、AI 回复、TTS 播放。
- 没有 AI 额度时，ASR/TTS 仍可用。
- 有试用额度时，AI 回复可用。

## P1：让 AI 读懂 fb2 业务数据

目标：AI 能基于比赛、赔率、用户订单和群友观点回答。

主项目：

- 群聊 AI 拉取 fb2 Context Pack。
- 预算裁剪并注入 prompt。
- 输出 `context_quality` 和工具 readiness。
- 回答中要求引用 source id。

fb2：

- 实现 `/api/main-project/context/pack`。
- 返回 `context_pack`、`matches`、`user_orders`、`group_messages`、`metrics`、`tool_contract`。
- 用户订单只返回当前用户自己的数据。
- 平台订单只返回匿名聚合。

验收：

- AI 能回答“今天比赛”“我的票风险”“群友观点分歧”。
- 回答能引用 `match_id`、`order_id`、`message_id`。
- 数据缺失时不编造。

## P2：从一次性上下文升级到可调用工具

目标：AI 在上下文不足时能按需查 fb2 细节。

主项目：

- 从 `declared_only` 升级到受控工具执行。
- 记录工具调用审计。
- 把工具结果合入回答引用。

fb2：

- 实现推荐工具：`search_matches`、`get_match_detail`、`search_user_orders`、`get_order_detail`、`search_group_opinions`、`get_context_audit`。
- 所有工具返回 source id、权限状态、更新时间。

验收：

- 用户追问单场比赛时，不需要重新塞完整 Context Pack。
- 用户追问订单时，只能查自己的订单。
- 排障时能通过 `context_audit_id` 回查来源和裁剪状态。

## P3：长期评测和领域索引

目标：让 AI 分析能力长期变好，而不是每次重新搜索互联网。

主项目：

- 维护固定评测集。
- 统计回答引用覆盖率、空上下文率、裁剪率。
- 根据评测结果调整 prompt、budget 和工具选择。

fb2：

- 建比赛、赔率、订单、群观点和审计索引。
- 做每日/每周复盘摘要。
- 把用户观点、命中情况和失败原因变成可检索经验。

验收：

- 常见问题回答更快。
- 回答引用更准。
- 空上下文和超大上下文比例下降。
- 历史观点和复盘能被后续分析使用。
