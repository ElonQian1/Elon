# fb2 AI Center 交接

## 当前快照

日期：2026-06-21

主项目当前已经具备：

- fb2 外部应用注册、默认群和品牌配置。
- fb2 账号同步、会话创建、主项目授权登录 fb2。
- fb2 首次登录试用额度配置。
- `chat-bootstrap` 输出聊天、语音、ASR/TTS 和推荐体验协议。
- `chat-bootstrap` 已输出机器可读 `voice.composer` 契约，明确 fb2 应接 `VoiceComposerView`、开启录音浮层、系统 ASR 超时后走云端兜底。
- `chat-bootstrap` 已输出机器可读 `aiReply` 契约，明确 `@EL`、长按 `AI回复`、群聊总结入口都走主项目 Context Pack + AI 回复链路。
- `chat-bootstrap` 已输出机器可读 `billing` 契约，明确 `/api/me/balance`、试用额度来源和“ASR/TTS 免费、AI 回复扣费”的检查点。
- `context-contract` 输出 Context Pack 示例、质量告警、工具契约、观测指标和计费策略。
- `context-contract` 已输出 `answer_policy_contract`（`fb2.answer_policy.v1`），明确 AI 回答要区分数据事实、群友观点和 AI 推断，并带固定评测问题。
- `context-contract` 已输出 `context_readiness_contract`，用于自动判断 fb2 Context Pack 是否足够支撑 AI 回答。
- fb2 Context Pack 进入 prompt 后会附加 `<answer_rules>`，这些规则来自主项目 `answer_policy_contract.prompt_answer_rules`；归一化上下文也会带 `answer_policy` metadata。
- 群聊 AI 拉取 fb2 Context Pack 时，会把最后一次有效 @EL 用户问题作为 `topic_hint` 传给 fb2。
- 长按群消息点击 `AI回复` 时，主项目会把被选中消息作为 `topic_hint` 拉取 fb2 Context Pack。
- 群聊总结帖会把 `topic/title/instructions` 合成 `topic_hint`；Context Pack 回退到 today-matches 时也会继续传 `group_id/topic_hint`。
- 主项目会保留 fb2 Context Pack 返回的 `citation_sources`，供 AI 回答引用来源和后续质量回填使用。
- 主项目拉取 fb2 Context Pack 时，若当前主项目用户已绑定 fb2 账号，会同时传 `external_user_id` 和同值 `X-FB2-AI-CONTEXT-USER-ID`；这修复了 fb2 最新权限契约下“我的票/我的订单”上下文被 403 拦截的问题。
- 主项目只有在 `ELON_EXTERNAL_APP_FB2_PLATFORM_ORDER_CONTEXT=true` 时才会请求 `include_platform_orders=true`，并同步传 `X-FB2-AI-CONTEXT-SCOPE: platform_order_summary`；fb2 侧仍可通过 `FB2_AI_CONTEXT_PLATFORM_ORDER_SUMMARY_ENABLED` 拒绝平台摘要，避免普通群聊越权读取平台经营数据。
- 主项目按需执行 fb2 工具时，也会为用户订单工具带同值 `X-FB2-AI-CONTEXT-USER-ID`，避免工具调用绕过 fb2 的订单归属检查。
- 主项目按需执行 fb2 `platform_orders` 工具时，会同步带 `X-FB2-AI-CONTEXT-SCOPE: platform_order_summary`；线上已验证 fb2 返回 `visibility=privileged_summary`、`redaction=anonymous_aggregate_only` 和 `platform_order_summary:<date>:all` source id。
- fb2 已提供统一工具执行入口 `POST /api/main-project/tools/execute`，线上 smoke 已验证 `search_matches` 可返回比赛来源、`search_user_orders` 缺少上下文用户头会 403、带同值头只返回本人订单、不支持工具会 400。
- 群聊 `@EL` 和长按消息 `AI回复` 生成主项目 AI 回复后，会后台调用 fb2 `/api/main-project/context/feedback`，用 `context_audit_id`、主项目消息 ID、命中的引用来源和触发类型记录自动反馈样本；失败只写日志，不阻断聊天出消息。
- 当主项目工具结果包含已 grounded 的 fb2 `opinion_memories`，且 AI 回复正文显式提到对应观点记忆 source id 或原群消息 id 时，主项目会继续调用 fb2 `record_opinion_adoption`，把这次“群观点被采纳进回答”的证据写回 fb2 质量闭环；未显式引用则不自动采纳，避免把群友观点误当事实。
- 主项目工具契约、planner、grounding 和 prompt 已接入 fb2 的只读质量工具：`list_opinion_adoptions`、`opinion_adoption_summary`、`opinion_result_reviews`、`opinion_result_review_summary`；聊天 AI 不会自动触发 `refresh_opinion_result_reviews` 这类刷新/写入工具。
- 主项目上下文日志已补 `topic_hint_present`、`fallback_used`、`answer_policy_schema`、`context_quality_warning_count`、`tool_readiness_status`，用于排查 fb2 AI 为什么没用上业务数据。
- 群聊 AI 可拉取 fb2 Context Pack 并做预算裁剪。
- `android/chat-voice-kit` 已输出 `VoiceComposerView`、录音浮层、系统 ASR、云端 ASR 兜底和 TTS。

当前仍需重点推进：

- 用真实群聊消息和真实用户票据继续扩充联调样本，确认“我的票/群观点”在不同账号权限下都返回期望数据；平台订单风险工具的匿名聚合正向 smoke 已通过。
- 发布后抽样验证主项目群聊链路里 `user_order_context_present=true` 的日志，确认用户订单上下文已经从 fb2 进入 prompt；平台摘要仍应只在双端开关和 scope 同时开启时出现。
- 用主项目真实群聊入口触发 `@EL` 和长按消息 `AI回复`，确认主项目工具执行结果进入 prompt 后，AI 回答能显式区分比赛事实、本人订单、群观点和 AI 推断。
- 用已完赛比赛样本验证 `opinion_result_review_summary` 和 `opinion_result_reviews` 在主项目真实群聊回答中只被描述为历史复盘/样本统计，不被写成未来命中承诺。
- fb2 接入 `VoiceComposerView` 的完整输入栏，而不是只接 ASR/TTS。
- fb2 真机验证小米/HyperOS 系统 ASR 超时后云端兜底。
- 主项目和 fb2 建立固定 AI 数据回答评测集。
- 后续把 fb2 工具执行从当前的 Context Pack + 轻量工具调用继续升级为更细粒度的可评测工具链。

## 主项目负责人待办

- 保持 `/api/external/apps/fb2/context-contract` 与文档同步。
- 继续完善 Context Pack prompt 投影和质量告警。
- 增加 fb2 Context Pack 拉取失败、空数据、超预算的回归测试。
- 观察 `auto_generated_answer_feedback` 和 `record_opinion_adoption` 样本，后续如果 AI 回答未显式引用 source id，要继续强化 prompt 或前端引用展示。
- 维护 `android/chat-voice-kit` 的公共 API，不让 fb2 复制主 App 内部代码。
- 明确发布 commit 和 fb2 重新编译要求。

## fb2 负责人待办

- 按 `contracts.md` 实现 Context Pack。
- 给比赛、赔率、订单、群观点都补 source id。
- 接 `VoiceComposerView`，不要再使用临时 Web 浮层作为长期方案。
- 把 AI 回复问题链路打通：群消息进入主项目、触发 `@EL`、模型回复、TTS 播放。
- 回传真机日志、接口响应和 APK 版本。

## 交接模板

```md
# 交接记录

## 当前目标

## 本班完成

## 代码提交

## 已验证

## 未验证

## 当前阻塞

## 下班继续做什么

## 风险
```

## 会话协作规则

- 主项目会话只改主项目 SDK、服务端、契约和文档。
- fb2 会话只改 fb2 客户端、fb2 服务端和业务数据接口。
- 两边不能同时改同一个文件或同一接口实现。
- fb2 改完接口后，主项目先读最新契约和返回样例，再调整 prompt/质量告警。
- 主项目 SDK 改完后，fb2 重新引用主项目 commit 并重打 APK。
