# fb2 AI Center 工作台

这个目录是主项目和 fb2 子项目长期协作的统一入口。目标不是一次性把所有能力写完，而是把聊天、语音、AI 回复、业务数据上下文、评测和发布交接固定成可持续演进的工作流。

## 当前结论

- 主项目是 AI Center，负责账号互通、默认群聊、聊天协议、语音 SDK、AI 生成、计费和上下文注入。
- fb2 是业务数据提供方，负责比赛、赔率、订单、群友观点、平台汇总和审计指标。
- 第一阶段不做完整 MCP/RAG。先用 HTTP Context Pack，把 fb2 业务上下文转成模型友好的 Markdown/XML，再由主项目注入群聊 AI；后续 MCP 只能作为现有 REST Context Pack、tool manifest 和 tools/execute 的适配包装层，不能另立事实源。
- ASR、TTS、Context Pack 拉取免费；只有 AI 生成回复内容消耗 token/额度。
- fb2 不应该复制主项目内部聊天页代码。Android 原生侧优先接 `android/chat-voice-kit`，H5/WebView 侧按 `ChatVoiceInteractionContract` 还原。

## 已有主项目能力

- 外部应用注册：`server/src/external_app_registry.rs`
- 外部账号、授权、默认群和试用额度：`server/src/external_app_api.rs`、`server/src/store/external_apps.rs`
- fb2 聊天/语音启动协议：`GET /api/external/apps/fb2/chat-bootstrap`
- fb2 上下文契约：`GET /api/external/apps/fb2/context-contract`
- fb2 业务上下文拉取：`server/src/external_app_context.rs`
- Context Pack 预算和 prompt 投影：`server/src/external_app_context_budget.rs`
- 推荐工具契约：`server/src/external_app_context_tools.rs`
- 语音 SDK：`android/chat-voice-kit`
- 已有说明文档：`docs/fb2-business-context-pack.md`、`docs/fb2-chat-voice-kit-integration.md`

## 长期推进方式

1. 主项目先稳定公共契约：API、SDK、计费、prompt 投影、质量告警。
2. fb2 按契约提供 Context Pack 和业务工具，不让主项目直连 fb2 数据库。
3. 每次接入问题先补契约和测试，再考虑补代码。
4. 每轮交接都更新 `handoff.md`，多个会话按同一份状态继续。
5. 每个阶段都要有验收用例，避免“能跑一次”但长期不可维护。

## 文件说明

- 本任务的计划入口是 `docs/fb2-ai-center/PLAN.md`。仓库根目录如果出现其它 `PLAN.md`，可能属于 PC 节点、发布或其它并行任务，不代表 fb2 AI Center 当前计划。
- `contracts.md`：主项目和 fb2 之间的 HTTP、上下文、工具和 SDK 契约。
- `roadmap.md`：P0 到 P3 的执行顺序和验收目标。
- `data-tools.md`：fb2 应该提供哪些业务数据能力，以及从 Context Pack 走向 MCP/tools 的路径。
- `tool-manifest-boundary.md`：固定 live tool manifest、聊天自动工具、manifest-only 能力、integration-only 质量/权限端点和 source registry 的边界。
- `voice-sdk.md`：fb2 复用主项目 ASR/TTS 和微信式语音输入栏的落地标准。
- `billing-policy.md`：免费通道和扣费通道的固定口径。
- `test-plan.md`：端到端验收和长期评测清单。
- `final-acceptance-matrix.md`：终极目标逐项验收矩阵，明确每个接口、场景、权限和质量项需要什么证据。
- `handoff.md`：7*24 协作交接记录模板和当前状态。

真机语音证据先用 `scripts/collect-fb2-voice-device-evidence.ps1` 采集，再用 smoke 脚本验收。采集器会保存 screenshot、UI dump、logcat、包版本、权限、系统 ASR 服务和 `fb2.voice_device_evidence.v1` JSON；默认 `finalAcceptanceReady=false`，只适合定位和半成品证据。只有测试者已经用人工语音样本确认 system ASR final、云端 ASR fallback、server ASR 失败恢复、TTS 播放和 ASR/TTS 零余额免费，并为每个 `Observed*` 开关保留对应 artifact 时，才允许传 `-MarkFinalReady`。

当前 ASR/TTS 链路按业务安排暂缓，不作为本阶段继续推进项。非语音数据闭环使用独立 `-DataOnlyAcceptance`：它验证主项目健康、authenticated `chat-bootstrap` 的 AI 回复/计费/context fetch、live manifest、fb2 live Context Pack 六类场景、平台匿名摘要、APK 版本、权限负向审计、质量反馈、非合成 feedback 和群观点采纳；它不会要求主项目语音 SDK 构建或 `finalAcceptanceReady=true` 真机语音证据。这个模式只用于推进比赛/订单/平台摘要/群观点 AI 数据能力，不能替代最终 ASR/TTS 验收，也不能宣布终极目标完成。

真实群聊验收必须以接口直读为准：`smoke-fb2-visible-chat.ps1` 和最终 wrapper 要读取群聊 baseline、`@EL` seed/回复、selected-message seed/`AI回复`、summary post 和 feedback/quality 结果，并把消息 ID、记录数、正文长度、正文 sha256、匹配/未匹配统计写入日志和 summary。最终 wrapper 的 summary 必须包含 `visible_direct_read_complete=true` 和 `visible_direct_read_evidence`，记录 baseline 群消息读取、`@EL` seed/回复回读、selected-message seed/回复回读和 summary-post 回读；缺任一接口回读正文证据时，最终 `success` 必须为 false。截图只能辅助排查 UI，不得作为“AI 已在群聊回答、引用和反馈已闭环”的证明。

只需要确认当前账号能通过主项目群聊 API 直接读到 fb2 群消息时，先跑无写入预检：`scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password 123qwe`。它只做 session bridge、群成员检查和 baseline 消息读取，输出 `text_len`、`text_sha256` 和 `writes=false`，并写出 `fb2.main_project.visible_chat_readonly.v1` summary JSON；不会发送 `@EL`、不会触发 selected-message `AI回复`、不会创建总结帖。

readiness 和总结帖状态也要分层：full final 必须要求 fb2 authenticated readiness 为 `ready`，并要求 summary post 为模型生成 `ready`；data-only 当前允许 readiness `partial` 和 summary `ready_with_fallback`，但 summary 必须显式写出 `summary_post_fallback_used`、`summary_post_ready_for_mode` 和 readiness 允许原因。`degraded`、`blocked`、`unavailable` 不能通过 data-only 或 full final。

`-DataOnlyAcceptance -AllowVisibleMessages` 默认仍要求 `MinOpinionAdoptionCount=1`，避免真实群聊只产生 feedback 却没有观点采纳闭环。只有明确要做短窗口回归且接受“本轮不新增观点采纳”时，才允许加 `-AllowNoNewOpinionAdoptionInShortWindow`；该 opt-out 不能作为 full final 的观点采纳证据。

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>
```

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\collect-fb2-voice-device-evidence.ps1 -DeviceSerial <adb_serial> -CaptureHoldGesture -OutputDir target\fb2-voice-device-evidence\<run_id>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath target\fb2-voice-device-evidence\<run_id>\voice-device-evidence.json
```

常规巡检先跑无副作用脚本 `scripts/smoke-fb2-ai-center.ps1`。需要快速判断“现在差什么”时，先跑 `scripts/smoke-fb2-ai-center-status.ps1 -OutputPath target\fb2-ai-center\status-current.json`，它只读取本地 summary 和环境变量是否存在，不访问 fb2、不写群、不保存消息正文，并输出 `validation_scope.group_chat_evidence=api_direct_read_summary_only`。该状态脚本会把真正阻塞项放在 `blockers`，把“缺 token 不能刷新 live 验证”“旧 summary 没有新布尔字段但已有接口回读 hash”这类事项放在 `refresh_gaps`，避免把已通过的非语音直读闭环误判为阻塞。修改主 smoke 的语音证据门槛后先跑 `scripts/smoke-fb2-ai-center.ps1 -SelfTest`，它不需要 token、不会访问 fb2、不会发送群消息，会用离线合成证据验证 `finalAcceptanceReady`、APK 版本、严格布尔字段、artifact 路径/URL、占位 ref 拒绝、logcat 和截图/视频证据门槛。最终验收 wrapper 的本地逻辑先跑 `scripts/smoke-fb2-final-acceptance.ps1 -SelfTest`，它同样不需要 token，会验证三类 feedback coverage、子脚本 exit code、voice/quality/permission evidence 摘录和最终 success 条件不会退化。真实群聊接口读取能力可先跑 `scripts/smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead`，该模式不会写群；截图只能作为 APK UI 辅助材料，不能替代群聊 API 直读 summary。只有拿到明确授权后，才运行有副作用脚本 `scripts/smoke-fb2-visible-chat.ps1 -AllowVisibleMessages`，它会向真实群聊发送可见消息。

状态快照还会输出 `coordination schema=fb2.main_project.coordination.v1`，这是主项目和 fb2 子会话的机器可读交接字段。它固定 owner split、禁止项、当前 data-only summary/read-only summary/ai-center log 路径、`official -> ext_fb2_official` 群映射、`@EL`/selected-message/summary-post 的接口回读 ID 和 hash、当前 context projection 状态、安全命令，以及双方下一步动作。后续检查 fb2 对话时优先读这个字段：`coordination.direct_read_policy.screenshots_accepted=false` 且 `coordination.safe_commands.no_write_direct_read` 不会写真实群；只有 `coordination.safe_commands.visible_regression_requires_authorization` 明确带 `-AllowVisibleMessages`，它才是有副作用回归。

最终验收使用 `scripts/smoke-fb2-final-acceptance.ps1`。先用 `-PreflightOnly` 做无副作用预检：解析 `ExternalUserId`、确认该用户有订单上下文，并在不发送群消息的前提下强制验证 fb2 live 数据、六类标准场景、平台匿名摘要、fb2 APK 发布、主项目语音 SDK 构建、`finalAcceptanceReady=true` 的真机语音证据和 no-skip 门槛；同时会自动跑 `smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead`，把 `read_only_direct_read_complete`、只读 summary path 和样本消息正文 hash 写入总 summary。真机语音证据的 artifact 不能是占位 ref；本地文件必须存在，远端证据必须是 URL，且至少包含 logcat 和截图/视频。预检通过后，再用 `-AllowVisibleMessages` 把真实群聊可见触发、总结帖、summary-post feedback、非合成质量 readiness 和 `smoke-fb2-ai-center.ps1 -FinalAcceptance` 绑定到同一批证据，并输出机器可读 summary；summary 会记录子脚本日志路径、`@EL` 消息 ID、AI 回复 ID、长按 `AI回复` 消息 ID、回复正文策略证据 `visible_answer_policy_evidence`、feedback evidence、`feedback_coverage`、`visible_direct_read_complete`、summary post fallback 状态、非合成反馈数、群观点采纳数和引用记忆数。最终 `success` 必须证明 `visible_mention`、`selected_message`、`summary_post` 三类 feedback 都覆盖，接口直读完整，summary post 对当前模式可接受，并且 `exclude_synthetic=true` 的 feedback/adoption readiness 达到阈值。传 `-Fb2Username/-Fb2Password` 时会自动解析 `ExternalUserId`；缺 `FB2_AI_CENTER_TOKEN`、无法解析或手工提供有订单的 `ExternalUserId`、final-ready 真机语音证据或显式写群授权时必须失败。
