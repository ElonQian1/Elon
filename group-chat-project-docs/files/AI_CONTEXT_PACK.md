# 群聊 Context Pack 规则

每次 AI 生成总结帖前，系统必须先生成 Context Pack。AI 只能基于 Context Pack 和本目录文档生成总结。

## 必备字段

- `group_id`
- `task`
- `source_window`
- `selected_messages`
- `retrieval_strategy`
- `source_message_count`
- `group_ai_docs`
- `output_contract`

## 输出约束

- 总结帖必须引用 `selected_messages` 中的消息 ID。
- 如果 Context Pack 消息不足 2 条，必须提示证据不足。
- 如果成员之间没有明确结论，写“未形成明确结论”。
- 如果发现多个议题混在一起，应建议拆分，而不是强行合并。

## 可审计性

Context Pack 必须保存到系统中，并和总结帖版本绑定。用户之后查看总结帖时，可以看到当时 AI 依据了哪些消息和规则。

