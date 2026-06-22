# fb2 AI Center 验收计划

## 服务端契约测试

必须验证：

- `GET /api/external/apps/fb2` 返回 fb2 品牌、默认群和能力。
- `POST /api/external/apps/fb2/accounts/session` 能创建主项目用户会话，并默认加入官方群。
- 首次 fb2 会话能按配置发放 AI 试用额度。
- `GET /api/external/apps/fb2/chat-bootstrap` 返回聊天、ASR、TTS、WebSocket 和体验协议。
- `chat-bootstrap.voice.androidSdk.publicComponents` 包含 `VoiceComposerBootstrap`。
- `chat-bootstrap.voice.androidSdk.publicComponents` 包含 `VoiceComposerView` 和 `ChatVoiceEventSink`。
- `chat-bootstrap.voice.composer.requiredForMainProjectLikeExperience=true`。
- `chat-bootstrap.voice.composer.recommendedConfigApi=VoiceComposerBootstrap.applyFb2GroupChatConfig(...)`。
- `chat-bootstrap.voice.composer.defaultConfig.asr.serverFallbackEnabled=true`。
- `chat-bootstrap.voice.composer.defaultConfig.asr.serverConfigRequired=true`。
- `chat-bootstrap.voice.composer.defaultConfig.asr.localEngineFallbackEnabled=true` 且 `prewarmLocalEngine=true`。
- `chat-bootstrap.voice.composer.states` 包含 `SERVER_PROCESSING`，`zones` 包含 `AI_REPLY`，`callbacks` 包含 `onVoiceServerFallbackStarted`。
- `chat-bootstrap.voice.asr.localFirst=true`、`serverFallback=true`、`uploadEndpoint=/api/voice/asr`、`billing=free_auth_and_limits_only`。
- `chat-bootstrap.voice.tts.billing=free_auth_and_limits_only`。
- `chat-bootstrap.aiReply.schema=external_app.ai_reply.v1`。
- `chat-bootstrap.aiReply.externalContext.queryFields` 包含 `topic_hint`。
- `chat-bootstrap.aiReply.freePreparationSteps` 包含 `asr`、`tts`、`external_context_fetch`，且 `billableUnit=ai_reply_generation`。
- `chat-bootstrap.experience.usagePolicy.asr=free` 且 `aiReplyGeneration=billable`。
- `chat-bootstrap.experience.controls.fullWidthHoldToTalkButton=true`。
- `chat-bootstrap.billing.balanceEndpoint=/api/me/balance`。
- `chat-bootstrap.billing.gates.beforeAsr=never_check_ai_balance`。
- `chat-bootstrap.billing.gates.beforeTts=never_check_ai_balance`。
- `chat-bootstrap.billing.gates.beforeAiReplyGeneration=check_balance_or_trial_credit`。
- `GET /api/external/apps/fb2/context-contract` 返回 Context Pack 示例、质量告警、工具契约、观测指标和计费策略。
- `context-contract.answer_policy_contract` 返回引用规则和固定评测问题。
- `context-contract.answer_policy_contract.eval_scenarios` 返回六个机器可读评测场景：今日比赛、我的票、平台匿名订单摘要、群友观点、长按消息复核、来源审计。
- `context-contract.domain_context_projection_contract.source_registry.required_kinds` 只包含业务事实来源；`feedback`、`opinion_adoption` 必须位于 `quality_history_kinds`，避免 fb2 把质量闭环记录当成比赛/订单事实。
- `context-contract.domain_context_projection_contract.domain_scenario_matrix` 返回同六类真实用户问题的域数据矩阵，并声明每类问题需要的 Context Pack 小节、可自动工具、权限请求、source kinds、feedback 路由和验收信号。
- `context-contract.tool_result_envelope_contract` 返回 `fb2.tool_result_envelope.v1`、`external_app.normalized_tool_result.v1` 工具结果信封、`external_app.tool_result_grounding.v1` grounding 规则、business source kinds 和 quality history kinds。
- 默认 `scripts\smoke-fb2-ai-center.ps1` 会检查 `eval_scenarios` 的场景 id、权限边界、必需来源、必需引用和禁止输出，避免评测矩阵退化成只有标题。
- fb2 Context Pack / today-matches 响应归一化必须有服务端回归测试：HTTP 错误、非法 JSON、`success=false` 必须变成 `status=unavailable`；空 today-matches 数据必须带 `empty_matches` 质量告警；`metrics.budget_status=too_large` 必须映射为 `fb2_budget_too_large`，AI 只能说明缺口或截断风险，不能编造比赛、赔率、订单或群友观点。
- `context-contract.context_readiness_contract` 返回 required fields、prompt metadata 和 blocked/degraded/ready 判定标准。
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1` 通过，确认主项目健康、版本、实时 manifest 和主项目聊天自动工具覆盖。
- 默认 smoke 还必须直连 fb2 `/api/main-project/integration`，确认 `routing_mode=main_project_ready`、`service_token_header=X-FB2-AI-CENTER-TOKEN`、`official` 群映射，以及 `context_readiness`、`context_pack`、`tool_manifest`、`match_analysis_brief`、`group_opinion_summary`、用户订单、平台匿名摘要、质量和权限端点存在。
- 没有 `FB2_AI_CENTER_TOKEN` 时，默认 smoke 必须确认 fb2 `/context/readiness` 和 `/context/tool-manifest` 返回 401；带 token 的最终验收必须进一步验证 authenticated readiness 状态和 direct tool manifest 的必需 tool id，并与主项目 `context-contract.live_tool_manifest.tool_ids` 对齐。
- 需要验证 authenticated `chat-bootstrap` 时，脚本支持直接传 `-MainToken`，也支持传 `-Fb2Username/-Fb2Password` 或 `FB2_USER_TOKEN`，通过 fb2 `/api/main-project/session` 桥接主项目 token；这条路径无副作用，不会发送群消息。
- 需要验证 fb2 用户端 APK 是否已发布到可下载版本时，加 `-CheckFb2ApkVersion`；脚本会检查 fb2 `/api/app-version` 至少达到 `1.1.48`、`update_kind=full_apk`、checksum/size 有效，并对 `apk_url` 做 HEAD 验证。
- 需要把主项目 SDK 编译纳入同一次巡检时，加 `-CheckLocalVoiceSdkBuild`；脚本会执行 `android\gradlew.bat :chat-voice-kit:assembleDebug --quiet`。
- 需要把 fb2 真机语音链路纳入验收时，加 `-RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath <json>`；JSON 格式参考 `docs/fb2-ai-center/voice-device-evidence.example.json`，顶层必须是严格布尔值 `finalAcceptanceReady=true`，并覆盖 `VoiceComposerView`、按住说话、上滑取消、三段底部操作区、系统 ASR、云端 ASR 兜底、TTS 和 ASR/TTS 免费策略。`artifacts[].ref` 不能是示例/占位文案；本地 artifact 必须能按证据 JSON 所在目录或仓库根目录解析为真实文件，远端 artifact 必须是 `http(s)://` URL，并且至少有一条 logcat 和一条截图/视频证据。
- 采集 fb2 真机语音证据优先使用 `scripts\collect-fb2-voice-device-evidence.ps1`。默认命令只生成 `finalAcceptanceReady=false` 的半成品证据，用于保存 screenshot、UI dump、logcat、包版本、权限和系统 ASR 服务；它不是最终验收器。只有人工语音样本已经分别证明 system ASR final、系统 ASR 超时后 `/api/voice/asr` fallback 成功、server ASR 失败后 UI 恢复、TTS 播放、AI 余额为 0 时 ASR/TTS 免费，并且每个 `Observed*` 开关都有对应 logcat、截图、录屏或长期 URL 证据时，才能传 `-MarkFinalReady`。静音 ADB run 只能证明 UI/录音恢复，不能覆盖这些 ASR/TTS 最终项。
- 推荐采集命令：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\collect-fb2-voice-device-evidence.ps1 -DeviceSerial <adb_serial> -CaptureHoldGesture -OutputDir target\fb2-voice-device-evidence\<run_id>`。采集后立即运行：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath target\fb2-voice-device-evidence\<run_id>\voice-device-evidence.json`。若要作为最终交接证据，应把 artifact 复制到 `docs/fb2-ai-center/adb-evidence/<date>/` 或上传成长期 URL，避免 `target\` 被清理。
- 修改 `scripts/smoke-fb2-ai-center.ps1` 的语音证据规则后，先跑 `scripts\smoke-fb2-ai-center.ps1 -SelfTest`；该命令用离线合成 JSON 覆盖 final-ready 正例、artifact 解析变体、`finalAcceptanceReady=false`、字符串布尔值、占位 ref、缺本地文件、缺 logcat、缺截图/视频、空 artifact、低 APK 版本和缺系统 ASR 成功等失败路径。它只验证验收脚本本身，不替代真实 fb2 APK 的 `finalAcceptanceReady=true` 证据。
- 正式最终验收可加 `-RequireNoSkips`，确保缺少 token 或未覆盖的检查不会以 skip 形式被误判为完成。
- 当前 ASR/TTS 暂缓阶段，非语音数据验收使用 `-DataOnlyAcceptance`，不要用它替代 `-FinalAcceptance`。`scripts\smoke-fb2-ai-center.ps1 -DataOnlyAcceptance` 会自动打开 live fb2 数据、六类场景、平台匿名摘要、APK 版本、质量反馈、非合成 readiness、权限负向审计和 no-skip，并跳过 chat-bootstrap 里的 ASR/TTS/VoiceComposer 断言；`scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly` 不要求 `VoiceDeviceEvidencePath`，`-DataOnlyAcceptance -AllowVisibleMessages` 会把真实群聊 `@EL`、selected-message `AI回复`、总结帖、feedback coverage 和非语音质量证据绑定到同一份 summary。summary 中 `voice_status=deferred_by_user` 时，只代表语音本阶段不验收，不代表终极目标完成。
- 最终总验收优先使用 `-FinalAcceptance`；它会自动打开 `-RequireFb2Live`、`-RequireAllScenarios`、`-IncludePlatformOrderSummary`、`-CheckQuality`、`-RequireFeedbackCoverage`、`-CheckFb2ApkVersion`、`-CheckLocalVoiceSdkBuild`、`-RequireVoiceDeviceEvidence` 和 `-RequireNoSkips`，缺少主项目登录、`FB2_AI_CENTER_TOKEN` 或 `-VoiceDeviceEvidencePath` 时必须失败。
- `cd android && .\gradlew.bat :chat-voice-kit:assembleDebug` 通过，确认 fb2 可引用最新 `VoiceComposerBootstrap` 和 `VoiceComposerView`。
- 设置 `FB2_AI_CENTER_TOKEN` 后，上述巡检脚本能验证 fb2 live Context Pack、比赛分析、群观点和赛后复盘摘要；加 `-IncludePlatformOrderSummary` 后验证平台匿名订单摘要。
- 完整场景验收必须加 `-RequireAllScenarios`；此时脚本不仅要求参数存在，还会要求 Context Pack 有 `context_audit_id`、比赛/订单等数据数量达到门槛，并且关键场景有 `citation_sources`。
- 设置 `FB2_AI_CENTER_TOKEN` 后，`scripts\smoke-fb2-ai-center.ps1 -CheckQuality` 能验证 fb2 `/context/quality-summary`：`missing_context_rate`、`wrong_context_rate`、`citation_unmatched_rate` 不超过阈值，`large_context_pack_rate` 不超过预算阈值。
- 需要验证自动反馈闭环时，`scripts\smoke-fb2-ai-center.ps1 -CheckQuality -RequireFeedbackCoverage -QualitySince <RFC3339>` 必须确认最近反馈样本存在，且 `matched_cited_source_count` 达到门槛、`unmatched_cited_source_count=0`。
- 需要在没有 `FB2_AI_CENTER_TOKEN` 的情况下预检 fb2 生成的 Context Pack 格式时，先让 fb2 保存 `/api/main-project/context/pack` 完整响应 JSON，然后运行：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -InputPath <json> -Scenario today_matches_context_pack`。常用场景值：`today_matches_context_pack`、`my_ticket_context_pack`、`platform_order_context_pack`、`group_opinion_context_pack`；脚本只做离线样本格式验证，不访问网络，不替代 live `-DataOnlyAcceptance -PreflightOnly`。
- 需要验证生成后来源校验时，查看最近 generated-answer feedback：AI 回复正文显式提到的 source-like id 必须全部能匹配 Context Pack、选中消息额外来源、当前 `context_audit_id` 或 grounded/weak 成功工具结果；未匹配时应看到 `wrong_context=true`，且 `note` 包含 `source_validation=`。最终可见群聊批次仍要求 `quality_unmatched_cited_sources=0`，并通过群聊接口直读消息正文 hash，而不是截图。
- `scripts\smoke-fb2-ai-center.ps1 -RequireNonSyntheticQualityReadiness` 会用 `exclude_synthetic=true` 同时读取 fb2 `/context/feedback-summary`、`/context/quality-summary` 和 `/context/opinion-adoption-summary`，确认非合成反馈样本存在、`quality-summary` 与 `feedback-summary` 计数一致，并且群观点采纳数达到门槛。`-FinalAcceptance` 会自动启用该检查；默认阈值为 `MinNonSyntheticFeedbackCount=1`、`MinOpinionAdoptionCount=1`。最终 wrapper 的 data-only 可见群聊验收默认也保持该观点采纳门槛，只有显式 `-AllowNoNewOpinionAdoptionInShortWindow` 才允许短窗口不新增采纳。
- 需要验证权限边界时，加 `-CheckPermissionBoundaries -ExternalUserId <fb2_user_uuid>`；脚本会确认缺当前用户头的 Context Pack、`external_user_id` 与 `X-FB2-AI-CONTEXT-USER-ID` 不一致的 Context Pack、缺 platform scope 的平台摘要、缺当前用户头的用户订单工具均返回 403，并读取 `/context/permission-summary` 确认审计计数。
- 修改最终验收 wrapper 或文档映射后，先跑 `scripts\smoke-fb2-final-acceptance.ps1 -SelfTest`；该命令只验证离线合成日志的 feedback coverage、子脚本 exit code、voice/quality/permission evidence 摘录、data-only summary 的 `voice_status=deferred_by_user` 和 `success` 门槛，不需要 token，也不会发送群消息。
- 修改真实群聊可见回答策略或投注保证检测后，先跑 `scripts\smoke-fb2-visible-chat.ps1 -SelfTest`；该命令只用合成回复验证来源引用、事实/推断/风险分层、缺来源/缺风险失败、投注保证拦截、否定式表达放行，以及 selected-message/summary 引用原文时不被误判为 AI 诱导，不需要 token，也不会发送群消息。
- 只需要验证“能直接读取群聊功能，不用截图，也不写群”时，跑 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password 123qwe`。该模式只做 fb2 session bridge、`chat-bootstrap`、群成员检查和 baseline 消息读取，必须输出 `direct group message read baseline`、`read-only direct group read text fingerprint`、`writes=false`，并写出 `fb2.main_project.visible_chat_readonly.v1` summary JSON；summary 必须包含最近消息索引 `recent_messages`，每条只保存 `message_id/kind/sender/created_at/text_len/text_sha256`，用于定位 @EL / AI 回复链路且不保存正文。该模式不会发送 `@EL`、不会调用 selected-message `/ai-reply`、不会创建 summary post。截图只能作为 APK UI 辅助材料，不能替代这份 API 直读 summary。
- 需要快速交接当前进度时，跑 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center-status.ps1 -OutputPath target\fb2-ai-center\status-current.json`。该脚本只读取本地 summary、git SHA 和环境变量是否存在，输出 `blockers`、`refresh_gaps`、`next_actions` 和 `validation_scope.group_chat_evidence=api_direct_read_summary_only`；它不会访问 live fb2 Context Pack、不会写群、不会保存消息正文，也不能替代 `-DataOnlyAcceptance` 或 `-FinalAcceptance`。旧 data-only summary 如果没有 `visible_direct_read_complete`，但 `visible_direct_read_evidence` 已包含六类接口回读正文 hash，脚本应输出 `direct_read_evidence_complete=true`，并把“需要带 token 刷新新门槛字段”放入 `refresh_gaps` 而不是 `blockers`。
- 获得明确授权后，`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages` 能发送真实群聊 `@EL` 消息、调用 selected-message `/ai-reply`，并等到 `usr_elon_ai`/`gai_*` 回复；没有 `-AllowVisibleMessages` 时脚本必须拒绝写群。
- 可见群聊 smoke 不只检查消息 ID。`@EL` 和 selected-message 两类回复正文都必须包含来源标记、事实/观点/推断分层词、风险或不保证边界，且不得出现“肯定命中/稳赢/重注/包赢”等投注保证；selected-message 回复还必须明确反驳被测消息里的“肯定赢盘、重注”说法。
- 只抽样总结帖入口时，可运行 `scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage`；该脚本必须检查 summary 正文策略，有 `FB2_AI_CENTER_TOKEN` 时还必须等到 `trigger=group_summary_post` 的 fb2 feedback。
- 最终验收先跑 `scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly`，无副作用确认 `ExternalUserId`、用户订单上下文、fb2 live 数据、六类标准场景、平台匿名摘要、权限负向审计、fb2 APK 发布、主项目语音 SDK 构建、`finalAcceptanceReady=true` 的真机语音证据、no-skip 门槛，以及 no-write 群聊接口直读。预检 summary 必须包含 `preflight_evidence`、`read_only_direct_read_complete` 和 `read_only_direct_read_evidence`，直接记录 APK、语音、场景、权限审计和 `/api/me/groups/{group_id}/messages` 样本正文 hash。预检通过后，再跑 `scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages`，它会记录 `QualitySince`，发送真实群聊 `@EL`、selected-message `AI回复` 和总结帖，最后跑 `smoke-fb2-ai-center.ps1 -FinalAcceptance` 补齐质量反馈覆盖；输出的 summary 必须显示 visible chat 和 final acceptance 两个 exit code 都为 0，并包含可见消息 ID、AI 回复 ID、summary post/feedback、子脚本日志路径、feedback evidence、`feedback_coverage`、`visible_answer_policy_evidence` 和 `final_acceptance_evidence`。其中 `feedback_coverage.visible_mention`、`feedback_coverage.selected_message`、`feedback_coverage.summary_post` 必须全为 true。
- 真实群聊 summary 还必须包含 `visible_direct_read_complete=true` 和 `visible_direct_read_evidence`，机器可读地摘录 baseline 群消息读取、`@EL` seed/回复回读、selected-message seed/回复回读和 summary-post 回读；每条可见对话正文必须有 `text_len` 和 `text_sha256`。最终 wrapper 的 `success=true` 依赖该字段，避免后续用截图、人工翻日志或只有消息 ID 的记录代替接口证据。
- fb2 readiness 和 summary fallback 必须分层验收：`smoke-fb2-ai-center.ps1 -FinalAcceptance` 只接受 authenticated readiness `ready`；`-DataOnlyAcceptance` 只接受 `ready/partial`，并会拒绝 `degraded/blocked/unavailable`。`smoke-fb2-visible-chat.ps1` 默认只接受 summary post `ready`，只有最终 wrapper 在 `-DataOnlyAcceptance` 时才会传 `-AllowSummaryFallback` 允许 `ready_with_fallback`，并把 `summary_post_fallback_used` / `summary_post_ready_for_mode` 写入 summary。
- 主项目自动 feedback 的 `cited_sources` 只能回填当前 fb2 Context Pack audit registry 能匹配的来源。已执行工具返回的 `source_ids` 只有在同一个 Context Pack `citation_sources` 中存在匹配项，且工具结果为 `success=true`、`grounding.status=grounded/weak`、AI 回复正文显式提到该 ID 时，才可写入 fb2 `/context/feedback.cited_sources`；不能临时合成工具来源污染 `unmatched_cited_source_count`。工具结果里的观点记忆可继续通过 `record_opinion_adoption` 单独记录采纳。
- 主项目工具 planner 对“今天比赛/预测/赔率/我的票”优先规划 `match_analysis_brief`，并保留 `search_matches`、`search_user_orders` 等补充工具。
- 主项目工具 planner 对“群里大家怎么看/群友观点/讨论分歧/采纳建议”必须把 `group_opinion_summary` 和 `opinion_memories` 放进前 5 个自动工具；`opinion_memories` 默认按本群最近持久观点记忆读取 `{include_expired=false, limit=12}`，不得把整句用户问题作为 `query` 过度过滤。`search_group_opinions` 作为当前群聊观点来源补充工具保留。
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

真机证据必须沉淀为 `fb2.voice_device_evidence.v1` JSON，顶层 `finalAcceptanceReady=true` 必须是 JSON 布尔值，不接受字符串 `"true"`，并至少附一条真实可访问 logcat artifact 和一条截图/录屏 artifact。主项目最终验收脚本只接受机器可读证据，避免用口头描述、占位路径或静音半成品证据替代小米/HyperOS 等设备上的实际验证。

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
- 真机语音链路：fb2 `1.1.48(96)` 在小米/HyperOS 上已经通过 ADB 抽样确认文本/语音切换、`按住 说话`、上滑取消、直接发 3 秒语音消息和系统 ASR `empty_asr` 后 UI 回收；仍需要按 `docs/fb2-ai-center/voice-device-evidence.example.json` 同格式回传 `finalAcceptanceReady=true` 的人工语音样本、系统 ASR final、云端 ASR 兜底、server ASR 失败恢复、AI 回复区、TTS 和 ASR/TTS 免费证据。

当前可见群聊脚本：

- `scripts/smoke-fb2-visible-chat.ps1` 是有副作用 smoke，只能在明确授权后传 `-AllowVisibleMessages`。
- `scripts/smoke-fb2-final-acceptance.ps1` 是最终总验收入口；本地 wrapper 回归传 `-SelfTest`，无副作用 live 阶段传 `-PreflightOnly`，可见验收阶段传 `-AllowVisibleMessages`。除 `-SelfTest` 外，其它两种模式都必须提供 `FB2_AI_CENTER_TOKEN` 和 `finalAcceptanceReady=true` 的真机语音证据；`ExternalUserId` 可由 `-Fb2Username/-Fb2Password` 自动解析，只有无法解析时才需要手工传。
- 脚本支持直接用 fb2 用户账号桥接主项目 session，也支持传 `ELON_MAIN_TOKEN`。
- 验证通过不等于 APK UI 已通过；APK UI 仍需真机确认长按菜单和消息刷新显示。
