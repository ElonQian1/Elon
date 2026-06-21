# fb2 AI Center 验收计划

## 服务端契约测试

必须验证：

- `GET /api/external/apps/fb2` 返回 fb2 品牌、默认群和能力。
- `POST /api/external/apps/fb2/accounts/session` 能创建主项目用户会话，并默认加入官方群。
- 首次 fb2 会话能按配置发放 AI 试用额度。
- `GET /api/external/apps/fb2/chat-bootstrap` 返回聊天、ASR、TTS、WebSocket 和体验协议。
- `chat-bootstrap.voice.androidSdk.publicComponents` 包含 `VoiceComposerBootstrap`。
- `chat-bootstrap.voice.composer.requiredForMainProjectLikeExperience=true`。
- `chat-bootstrap.voice.composer.recommendedConfigApi=VoiceComposerBootstrap.applyFb2GroupChatConfig(...)`。
- `chat-bootstrap.voice.composer.defaultConfig.asr.serverFallbackEnabled=true`。
- `chat-bootstrap.aiReply.schema=external_app.ai_reply.v1`。
- `chat-bootstrap.aiReply.externalContext.queryFields` 包含 `topic_hint`。
- `chat-bootstrap.aiReply.freePreparationSteps` 包含 `external_context_fetch`，且 `billableUnit=ai_reply_generation`。
- `chat-bootstrap.experience.usagePolicy.asr=free` 且 `aiReplyGeneration=billable`。
- `chat-bootstrap.billing.balanceEndpoint=/api/me/balance`。
- `chat-bootstrap.billing.gates.beforeAsr=never_check_ai_balance`。
- `chat-bootstrap.billing.gates.beforeAiReplyGeneration=check_balance_or_trial_credit`。
- `GET /api/external/apps/fb2/context-contract` 返回 Context Pack 示例、质量告警、工具契约、观测指标和计费策略。
- `context-contract.answer_policy_contract` 返回引用规则和固定评测问题。
- `context-contract.context_readiness_contract` 返回 required fields、prompt metadata 和 blocked/degraded/ready 判定标准。
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1` 通过，确认主项目健康、版本、实时 manifest 和主项目聊天自动工具覆盖。
- `cd android && .\gradlew.bat :chat-voice-kit:assembleDebug` 通过，确认 fb2 可引用最新 `VoiceComposerBootstrap` 和 `VoiceComposerView`。
- 设置 `FB2_AI_CENTER_TOKEN` 后，上述巡检脚本能验证 fb2 live Context Pack、比赛分析、群观点和赛后复盘摘要；加 `-IncludePlatformOrderSummary` 后验证平台匿名订单摘要。
- 完整场景验收必须加 `-RequireAllScenarios`；此时脚本不仅要求参数存在，还会要求 Context Pack 有 `context_audit_id`、比赛/订单等数据数量达到门槛，并且关键场景有 `citation_sources`。
- 设置 `FB2_AI_CENTER_TOKEN` 后，`scripts\smoke-fb2-ai-center.ps1 -CheckQuality` 能验证 fb2 `/context/quality-summary`：`missing_context_rate`、`wrong_context_rate`、`citation_unmatched_rate` 不超过阈值，`large_context_pack_rate` 不超过预算阈值。
- 需要验证自动反馈闭环时，`scripts\smoke-fb2-ai-center.ps1 -CheckQuality -RequireFeedbackCoverage -QualitySince <RFC3339>` 必须确认最近反馈样本存在，且 `matched_cited_source_count` 达到门槛、`unmatched_cited_source_count=0`。
- 获得明确授权后，`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages` 能发送真实群聊 `@EL` 消息、调用 selected-message `/ai-reply`，并等到 `usr_elon_ai`/`gai_*` 回复；没有 `-AllowVisibleMessages` 时脚本必须拒绝写群。
- 主项目工具 planner 对“今天比赛/预测/赔率/我的票”优先规划 `match_analysis_brief`，并保留 `search_matches`、`search_user_orders` 等补充工具。
- 主项目工具 planner 对“群里大家怎么看/群友观点/讨论分歧”优先规划 `group_opinion_summary`，并保留 `search_group_opinions`、`opinion_memories` 等补充工具。
- `match_analysis_brief` 归一化结果必须校验 `visibility=match_focused_brief`；`group_opinion_summary` 必须校验 `visibility=single_group_lightweight_memory`。
- 群聊 AI prompt 包含 `<answer_rules>`，并由 `answer_policy_contract.prompt_answer_rules` 生成。
- 群聊 AI prompt metadata 包含 `answer_policy`。
- 群聊 AI 拉取 fb2 Context Pack 时，query 包含最后一次有效用户问题的 `topic_hint`。
- 长按群消息 `AI回复` 拉取 fb2 Context Pack 时，query 包含被选中消息的 `topic_hint`。
- 群聊总结帖拉取 fb2 Context Pack 时，query 包含由 `topic/title/instructions` 合成的 `topic_hint`。
- “这条消息说得对吗 / 靠谱吗 / 合理吗”这类长按消息评估问法会规划 `opinion_result_review_summary`，需要样本时规划 `opinion_result_reviews`。
- 回退 `/api/main-project/context/today-matches` 时，query 仍包含 `group_id/topic_hint`。
- 主项目拉取 fb2 上下文后，日志包含 `topic_hint_present`、`fallback_used`、`answer_policy_schema`、`context_quality_warning_count`、`tool_readiness_status`。
- 日志不得包含 shared secret、完整订单明细或用户问题原文。
- 群聊 AI 拉取 fb2 Context Pack 失败时能回退，不影响普通群聊回答。

## 语音端到端测试

设备覆盖：

- 小米/HyperOS
- 普通 Android AOSP/Pixel 类设备
- 没有系统 ASR 或系统 ASR 不稳定的设备

用例：

1. 文本模式切换到语音模式。
2. 按住说话，显示主项目同款浮层。
3. 说话后松手发送。
4. 上滑取消。
5. 底部选择 `AI回复`。
6. 底部选择 `转文字`。
7. 录音太短。
8. 系统 ASR 成功。
9. 系统 ASR 无 final/error，超时后云端 ASR。
10. 云端 ASR 失败，UI 恢复且不永久卡住。
11. AI 余额为 0 时 ASR/TTS 仍可用。

## AI 数据回答测试

固定问题集：

- “总结今天有哪些比赛值得讨论？”
- “分析 match_id=m-001 这场，赔率变化说明什么？”
- “帮我看看我今天的票风险在哪里？”
- “总结群里大家对这场比赛的不同观点。”
- “平台今天订单集中在哪些方向？只说匿名聚合。”
- “你刚才依据了哪些比赛、订单和群消息？”

合格标准：

- 回答引用 `match_id`、`order_id` 或 `message_id`。
- 区分数据事实、群友观点和 AI 推断。
- 不承诺命中，不诱导投注。
- 数据缺失时明确说明缺口。
- 不编造 fb2 没有提供的数据。

## 长期质量指标

每周看一次，并优先通过 `scripts\smoke-fb2-ai-center.ps1 -CheckQuality` 自动化巡检：

- Context Pack 平均耗时和 P95。
- 空上下文比例。
- 被主项目裁剪比例。
- `topic_hint_present=false` 的 AI 数据回答占比。
- `fallback_used=true` 的回退比例。
- `context_quality_warning_count` P50/P95。
- AI 回答来源引用覆盖率。
- 用户订单权限拒绝次数。
- 群友观点命中率。
- ASR 本地成功率。
- ASR 云端兜底成功率。
- AI 回复因余额不足被拒绝次数。

## 发布交接

每次主项目发版记录：

```md
## 主项目版本

- commit:
- server version:
- Android SDK version/build:
- 已验证接口:
- 已验证设备:
- 已知风险:
- fb2 需要重新编译/重打包:
```

每次 fb2 发版记录：

```md
## fb2 版本

- commit:
- APK versionName/versionCode:
- 引用主项目 commit:
- 已验证聊天:
- 已验证语音:
- 已验证 AI 数据回答:
- 已知风险:
```

## 当前未验证边界

以下场景不能用无副作用 smoke 替代，必须在拿到明确测试身份或沙盒群后执行：

- `chat-bootstrap` 鉴权正例：需要主项目用户 token。
- “帮我分析我的票”正例：需要明确授权的 fb2 测试用户 UUID，并且该用户确实有可分析订单。
- 真实群聊 `@EL`、长按消息 `AI回复`、总结帖入口：需要沙盒群，或用户明确允许在生产群产生可见 AI 消息。
- 真机语音链路：需要 fb2 引用 `android/chat-voice-kit` 后重打 APK，并覆盖小米/HyperOS 系统 ASR 超时兜底。

当前可见群聊脚本：

- `scripts/smoke-fb2-visible-chat.ps1` 是有副作用 smoke，只能在明确授权后传 `-AllowVisibleMessages`。
- 脚本支持直接用 fb2 用户账号桥接主项目 session，也支持传 `ELON_MAIN_TOKEN`。
- 验证通过不等于 APK UI 已通过；APK UI 仍需真机确认长按菜单和消息刷新显示。
