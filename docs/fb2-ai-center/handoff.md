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
- fb2 Context Pack 进入 prompt 后会附加 `<answer_rules>`，这些规则来自主项目 `answer_policy_contract.prompt_answer_rules`。
- 群聊 AI 拉取 fb2 Context Pack 时，会把最后一次有效 @EL 用户问题作为 `topic_hint` 传给 fb2。
- 长按群消息点击 `AI回复` 时，主项目会把被选中消息作为 `topic_hint` 拉取 fb2 Context Pack。
- 群聊总结帖会把 `topic/title/instructions` 合成 `topic_hint`；Context Pack 回退到 today-matches 时也会继续传 `group_id/topic_hint`。
- 主项目上下文日志已补 `topic_hint_present`、`fallback_used`、`context_quality_warning_count`、`tool_readiness_status`，用于排查 fb2 AI 为什么没用上业务数据。
- 群聊 AI 可拉取 fb2 Context Pack 并做预算裁剪。
- `android/chat-voice-kit` 已输出 `VoiceComposerView`、录音浮层、系统 ASR、云端 ASR 兜底和 TTS。

当前仍需重点推进：

- fb2 真实实现 `/api/main-project/context/pack` 并返回稳定业务数据。
- fb2 接入 `VoiceComposerView` 的完整输入栏，而不是只接 ASR/TTS。
- fb2 真机验证小米/HyperOS 系统 ASR 超时后云端兜底。
- 主项目和 fb2 建立固定 AI 数据回答评测集。
- 后续把 fb2 `declared_only` 工具升级为可执行工具。

## 主项目负责人待办

- 保持 `/api/external/apps/fb2/context-contract` 与文档同步。
- 继续完善 Context Pack prompt 投影和质量告警。
- 增加 fb2 Context Pack 拉取失败、空数据、超预算的回归测试。
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
