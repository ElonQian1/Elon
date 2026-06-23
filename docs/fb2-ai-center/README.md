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
- fb2 Context Pack 模板契约：`server/src/external_app_context_pack_template.rs`
- Context Pack 预算和 prompt 投影：`server/src/external_app_context_budget.rs`
- fb2 缺口提示硬保护：`server/src/external_app_context_gap_notice.rs`
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

没有 `FB2_AI_CENTER_TOKEN` 时，主项目会话不能直接读取 fb2 live Context Pack，但可以生成一份给 fb2 子会话执行的样本导出请求：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -PrintExportRequest -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d -OutputPath target\fb2-ai-center\context-pack-sample-request-current.json`。该 JSON 会列出今日比赛、我的票、平台匿名摘要、群友观点四类 `/api/main-project/context/pack` 请求模板、期望 source kinds、保存路径和离线校验命令；它不包含 token，不写群，也不保存消息正文。

fb2 子项目实现 `/api/main-project/context/pack` 时，优先读取主项目 `context-contract.context_pack_template_contract`。它是机器可读的 Markdown/XML 模板：正文必须是 `<fb2_context_pack>` 包裹的 Markdown，固定包含 `usage_boundary`、`match_facts`、`user_order_slice`、`platform_order_summary`、`group_opinion_slice`、`retrieval_evidence`、`quality_feedback` 七个小节；JSON metadata 必须包含 `context_audit_id`、`citation_sources`、`metrics`、`tool_contract`、`answer_policy` 和 `preflight_readiness` 等字段。MCP 可以以后包装这套 REST 契约，但不能绕过这份模板直接另建事实源。

真实群聊验收必须以接口直读为准：`smoke-fb2-visible-chat.ps1` 和最终 wrapper 要读取群聊 baseline、`@EL` seed/回复、selected-message seed/`AI回复`、summary post 和 feedback/quality 结果，并把消息 ID、记录数、正文长度、正文 sha256、匹配/未匹配统计写入日志和 summary。最终 wrapper 的 summary 必须包含 `visible_direct_read_complete=true` 和 `visible_direct_read_evidence`，记录 baseline 群消息读取、`@EL` seed/回复回读、selected-message seed/回复回读和 summary-post 回读；缺任一接口回读正文证据时，最终 `success` 必须为 false。截图只能辅助排查 UI，不得作为“AI 已在群聊回答、引用和反馈已闭环”的证明。

只需要确认当前账号能通过主项目群聊 API 直接读到 fb2 群消息时，先跑无写入预检：`scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password 123qwe`。它只做 session bridge、群成员检查和 baseline 消息读取，输出 `text_len`、`text_sha256` 和 `writes=false`，并写出 `fb2.main_project.visible_chat_readonly.v1` summary JSON；summary 会带最近 20 条消息的 `recent_messages` 索引，只保存消息 ID、类型、发送方、时间、正文长度和 sha256，不保存正文。该模式不会发送 `@EL`、不会触发 selected-message `AI回复`、不会创建总结帖。

fb2 回答的防编造保护分两层：prompt 内的 `context_gap_summary` 会提示模型哪些数据缺失；生成后还会经过 `external_app_context_gap_notice`。只要 fb2 外部上下文报告 readiness 阻断/降级、Context Pack 为空/过大/被截断或缺少可引用 Context Pack，最终聊天回复必须追加 `数据缺口` 行，明确不能把缺失数据编造成比赛、赔率、订单或群友观点事实。

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

常规巡检先跑无副作用脚本 `scripts/smoke-fb2-ai-center.ps1`。需要快速判断“现在差什么”时，优先跑 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1`；它会刷新 `status-refresh-current.json`，再运行 evidence freshness、gap action board、completion matrix 和 handoff prompt validator，并输出 `target\fb2-ai-center\current-state-validation-current.json`。如果只想刷新摘要而不跑总门禁，可单独运行 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1`；它会依次刷新公开契约、`status-current.json`、`goal-audit-current.*`、`handoff-current.*` 和 `handoff-prompt-current.md`，默认合并当前 worktree 与主工作区历史证据，并输出/保存 `fb2.main_project.status_refresh.v1` 摘要到 `target\fb2-ai-center\status-refresh-current.json`。该摘要里的 `owner_next_actions` 会把下一步拆成主项目、fb2 子项目和 shared 三类动作，`blocking_state` 会区分可继续做的无密钥回归和必须等 `FB2_AI_CENTER_TOKEN` 的 live 验证，`next_commands` 会给出当前状态总门禁、缺口行动板验证、回答策略证据验证、无写群直读、data-only preflight 和显式授权 visible regression 的可执行命令占位，`completion_matrix` 会列出每个最终目标 requirement 的分组、owner、状态、证据和缺口，`gap_action_board` 会把每个剩余 gap 映射为 owner、所需证据、可执行命令、是否需要密钥和是否会写群，`evidence_freshness` 会标出各 summary artifact 来自当前输出目录还是历史证据目录、最后修改时间和 age；`handoff-prompt-current.md` 则把这些字段整理成可复制给下一轮主项目或 fb2 子会话的执行提示，并自动把 token/password 参数替换成占位。修改这个总入口后先跑 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1 -SelfTest` 和 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1 -SelfTest`，这些自测跳过网络和主工作区历史证据，只验证本地编排和输出文件；修改缺口行动板后跑 `scripts\validate-fb2-ai-center-gap-action-board.ps1 -SelfTest` 和正式验证命令；修改可见群聊回答策略证据后跑 `scripts\validate-fb2-visible-answer-policy.ps1 -SelfTest`。底层状态脚本仍是 `scripts/smoke-fb2-ai-center-status.ps1 -OutputPath target\fb2-ai-center\status-current.json`，它只读取本地 summary 和环境变量是否存在，不访问 fb2、不写群、不保存消息正文，并输出 `validation_scope.group_chat_evidence=api_direct_read_summary_only`。该状态脚本会把真正阻塞项放在 `blockers`，把“缺 token 不能刷新 live 验证”“旧 summary 没有新布尔字段但已有接口回读 hash”这类事项放在 `refresh_gaps`，避免把已通过的非语音直读闭环误判为阻塞。修改主 smoke 的语音证据门槛后先跑 `scripts/smoke-fb2-ai-center.ps1 -SelfTest`，它不需要 token、不会访问 fb2、不会发送群消息，会用离线合成证据验证 `finalAcceptanceReady`、APK 版本、严格布尔字段、artifact 路径/URL、占位 ref 拒绝、logcat 和截图/视频证据门槛。最终验收 wrapper 的本地逻辑先跑 `scripts/smoke-fb2-final-acceptance.ps1 -SelfTest`，它同样不需要 token，会验证三类 feedback coverage、子脚本 exit code、voice/quality/permission evidence 摘录和最终 success 条件不会退化。真实群聊接口读取能力可先跑 `scripts/smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead`，该模式不会写群；截图只能作为 APK UI 辅助材料，不能替代群聊 API 直读 summary。只有拿到明确授权后，才运行有副作用脚本 `scripts/smoke-fb2-visible-chat.ps1 -AllowVisibleMessages`，它会向真实群聊发送可见消息。

在隔离临时 worktree 中工作时，如果历史 live 验收证据保存在主目录 `D:\rust\active-projects\elon cli\target\fb2-ai-center`，状态脚本必须带额外证据目录，例如：`scripts\smoke-fb2-ai-center-status.ps1 -EvidenceDirs "D:\rust\active-projects\elon cli\target\fb2-ai-center" -OutputPath target\fb2-ai-center\status-current.json`。脚本也支持环境变量 `FB2_AI_CENTER_SUMMARY_DIR` / `FB2_AI_CENTER_SUMMARY_DIRS`，用于多会话共享同一批 data-only summary、read-only direct-read、ai-center log 和 Context Pack 样本。

需要快速判断“主项目公开契约线上是什么状态”时，先跑无密钥脚本：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-public-contract-status.ps1 -OutputPath target\fb2-ai-center\public-contract-status-current.json`。它只访问主项目公开 `/health`、`/api/server/version` 和 `/api/external/apps/fb2/context-contract`，检查 `domain_data_blueprint_contract`、`domain_context_index_contract`、`group_chat_evidence_contract` 和 live tool manifest；不读取 fb2 订单、群消息正文或 service-token 保护接口，不能替代 `-DataOnlyAcceptance` 或 `-FinalAcceptance`。随后运行状态快照时会读取这份 summary，并输出 `latest_public_contract_status`；该字段必须显示 `domain_context_index_schema=fb2.main_project.domain_context_index.v1`、`group_chat_test_method=direct_api_read`、`screenshots_accepted=false` 和 `required_group_message_fields` 含 `message_id/text_len/text_sha256`，否则只能说明公开契约状态缺失或不完整，不能用截图补位。

需要把默认 smoke 的 console 检查固化成可交接证据时，加 `-SummaryPath target\fb2-ai-center\contract-smoke-summary-current.json`。该 summary schema 为 `fb2.main_project.contract_smoke_summary.v1`，会把 chat-bootstrap、ASR/TTS 免费契约、AI 回复计费、live manifest、domain contract、fb2 dynamic discovery、service-token 401 边界和 fb2 live 数据状态压成 `latest_contract_smoke_summary`；缺 `FB2_AI_CENTER_TOKEN` 时，默认允许 `fb2_live_data_status=skipped_missing_FB2_AI_CENTER_TOKEN`，但 `-RequireFb2Live` 或 `-RequireNoSkips` 仍会让缺口变成失败。

需要把当前状态交给 fb2 子会话或下一轮主项目会话时，先刷新 public/status，再生成 handoff 报告：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-handoff-report.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\handoff-current.json -MarkdownPath target\fb2-ai-center\handoff-current.md`。报告 schema 为 `fb2.main_project.handoff_report.v1`，只汇总当前本地 summary 的完成项、缺口、证据策略和安全命令；如果当前 worktree 没有带 token 的 data-only summary/log，它会如实显示七类用户场景未全量完成，而不是从历史文档里推断完成。

需要回答“现在离最终目标差多少”时，在 status 之后生成目标审计报告：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-goal-audit-report.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\goal-audit-current.json -MarkdownPath target\fb2-ai-center\goal-audit-current.md`。报告 schema 为 `fb2.main_project.goal_audit_report.v1`，逐项列出 Context Pack 契约、主项目 contract smoke、今日比赛、我的票、平台匿名摘要、群观点、长按 `AI回复`、总结帖、来源审计、权限、feedback/quality、群聊接口直读和语音 final evidence；`data_goal_complete=true` 只代表非语音数据/聊天/权限/反馈目标完成，`full_final_complete=true` 还必须有 status 中的 `latest_final_acceptance` 同批 `visible_final_acceptance` / `full_final_acceptance` summary、`voice_status=required`、两个子脚本 exit code 为 0、feedback/direct-read 完整和 ASR/TTS final-ready 真机证据。

状态快照还会输出 `latest_context_pack_sample_request`。当主项目没有 `FB2_AI_CENTER_TOKEN` 时，如果该字段 `complete=true`，说明可以让 fb2 子会话按 `context-pack-sample-request-current.json` 导出 live Context Pack 样本；如果该字段缺失或不完整，先运行 `validate-fb2-context-pack.ps1 -PrintExportRequest` 生成请求。

fb2 子会话导出样本后，用 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\context-pack-samples-validation-current.json` 一次性校验四类样本。输出 summary 只保留场景、audit id、source kinds、citation source 数量、Context Pack 长度和 sha256，不保存订单或群聊正文；状态快照会读取 `latest_context_pack_sample_set`，用于判断样本是否已经离线通过。

状态快照还会从样本集推导 `latest_context_answer_readiness`。该字段按四类真实用户问题检查必需来源覆盖：今日比赛需要 `match/odds/context_audit`，我的票需要 `user_order/ticket/context_audit`，平台订单风险需要 `platform_order_summary/context_audit`，群观点需要 `group_message/opinion_memory/context_audit`。它只证明样本能支撑回答输入，不代表模型已生成回复；最终仍要用 live token 跑权限、质量和 feedback 验证。

状态快照还会输出 `coordination schema=fb2.main_project.coordination.v1`，这是主项目和 fb2 子会话的机器可读交接字段。它固定 owner split、禁止项、当前 data-only summary/read-only summary/ai-center log 路径、`official -> ext_fb2_official` 群映射、`@EL`/selected-message/summary-post 的接口回读 ID 和 hash、当前 context projection 状态、安全命令，以及双方下一步动作。后续检查 fb2 对话时优先读这个字段：`coordination.direct_read_policy.screenshots_accepted=false` 且 `coordination.safe_commands.no_write_direct_read` 不会写真实群；只有 `coordination.safe_commands.visible_regression_requires_authorization` 明确带 `-AllowVisibleMessages`，它才是有副作用回归。

状态快照还会输出 `goal_gap_audit schema=fb2.main_project.goal_gap_audit.v1`。这是当前终极目标的差距总表：`completed` 表示已有机器证据覆盖的能力，`missing` 表示仍缺的 live token、语音 final evidence 或 full final 同批验收，`deferred_by_user` 会明确列出本阶段暂停的 ASR/TTS。`completed` 应包含 `domain_context_index_contract` 和 `main_project_contract_smoke`，并在 `current_flags.domain_context_index_contract_complete`、`current_flags.main_project_contract_smoke_complete`、`evidence_refs.domain_context_index_*`、`evidence_refs.contract_smoke_*` 中保留索引契约和默认 smoke 证据；群聊对话相关项必须满足 `direct_read_policy.screenshots_accepted=false`，并通过 `/api/me/groups/{group_id}/messages` 或 summary-post 接口回读拿到 `text_len/text_sha256`；截图、录屏或人工看见 UI 只能辅助定位，不能作为 AI 已在群聊读取/回复/引用/反馈闭环的验收依据。

状态快照还会输出 `latest_user_scenario_audit schema=fb2.main_project.user_scenario_audit.v1`。该字段把用户真实问题映射为可审计产品场景：今日比赛、我的票、平台订单风险、群观点、长按消息复核、总结帖和来源审计。它同时固定当前上下文路线：`context_format=xml_wrapped_markdown_context_pack_with_json_metadata`，`mcp_status=not_first_phase_use_rest_context_pack_and_tool_manifest_first`；也就是说，fb2 先提供 REST Context Pack + tool manifest + tools/execute 闭环，MCP 以后只作为包装层或增强层，不作为第一阶段事实源。

状态快照还会输出 `latest_domain_data_blueprint schema=fb2.main_project.domain_data_blueprint.v1`。该字段回答“fb2 长期到底要给主项目 AI 什么数据工具”：比赛赔率、当前用户票据、平台匿名摘要、群观点、观点学习闭环、质量反馈审计 6 条 lane；每条 lane 都有 Context Pack 小节、source kinds、主工具、权限 scope、回答分层、禁止输出和未来索引。它明确主项目不复制 fb2 业务数据，第一阶段仍走 REST Context Pack + tool manifest + tools/execute，MCP 以后只做权限和审计不变的包装。

同一口径也会通过主项目接口公开：`GET /api/external/apps/fb2/context-contract` 返回 `domain_data_blueprint_contract`。fb2 子会话和未来子项目不需要读取主项目本地脚本，就能从接口拿到长期 lane 蓝图和格式边界。

长期领域索引口径也会通过主项目接口公开：`GET /api/external/apps/fb2/context-contract` 返回 `domain_context_index_contract schema=fb2.main_project.domain_context_index.v1`。它固定 fb2 后端内部至少维护比赛、赔率、当前用户票据、平台匿名风险、群观点、观点记忆、上下文审计、反馈质量 8 类索引；主项目只消费 `retrieval_evidence`、`citation_sources`、metrics 和工具结果，不复制 fb2 业务数据，也不接收 raw embedding dump。

真实群聊直读口径也会通过主项目接口公开：`GET /api/external/apps/fb2/context-contract` 返回 `group_chat_evidence_contract`。它声明 `group_chat_test_method=direct_api_read`、`screenshots_accepted=false`，并要求 `message_id`、`text_len`、`text_sha256` 等字段；后续测试 fb2 对话优先用该契约和群聊 API，而不是截图。

状态快照还会输出 `live_preflight_request schema=fb2.main_project.live_preflight_request.v1`。该字段不包含任何 secret，只说明当前是否已经具备“拿到 `FB2_AI_CENTER_TOKEN` 后立即刷新 live 权限/质量/feedback”的条件，并给出无写群 `-DataOnlyAcceptance -PreflightOnly` 命令、目标用户、目标群和验收门槛。它的 `evidence_policy` 固定为 `group_chat_test_method=direct_api_read`、`screenshots_accepted=false`，群聊验证必须有消息 ID、正文长度和正文 hash。当前如果 `ready_without_token=true` 且 `missing=FB2_AI_CENTER_TOKEN`，说明主项目侧下一步不是改格式，也不是看截图，而是补 token 跑 live preflight。

最终验收使用 `scripts/smoke-fb2-final-acceptance.ps1`。先用 `-PreflightOnly` 做无副作用预检：解析 `ExternalUserId`、确认该用户有订单上下文，并在不发送群消息的前提下强制验证 fb2 live 数据、六类标准场景、平台匿名摘要、fb2 APK 发布、主项目语音 SDK 构建、`finalAcceptanceReady=true` 的真机语音证据和 no-skip 门槛；同时会自动跑 `smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead`，把 `read_only_direct_read_complete`、只读 summary path 和样本消息正文 hash 写入总 summary。真机语音证据的 artifact 不能是占位 ref；本地文件必须存在，远端证据必须是 URL，且至少包含 logcat 和截图/视频。预检通过后，再用 `-AllowVisibleMessages` 把真实群聊可见触发、总结帖、summary-post feedback、非合成质量 readiness 和 `smoke-fb2-ai-center.ps1 -FinalAcceptance` 绑定到同一批证据，并输出机器可读 summary；summary 会记录子脚本日志路径、`@EL` 消息 ID、AI 回复 ID、长按 `AI回复` 消息 ID、回复正文策略证据 `visible_answer_policy_evidence`、feedback evidence、`feedback_coverage`、`visible_direct_read_complete`、summary post fallback 状态、非合成反馈数、群观点采纳数和引用记忆数。最终 `success` 必须证明 `visible_mention`、`selected_message`、`summary_post` 三类 feedback 都覆盖，接口直读完整，summary post 对当前模式可接受，并且 `exclude_synthetic=true` 的 feedback/adoption readiness 达到阈值。传 `-Fb2Username/-Fb2Password` 时会自动解析 `ExternalUserId`；缺 `FB2_AI_CENTER_TOKEN`、无法解析或手工提供有订单的 `ExternalUserId`、final-ready 真机语音证据或显式写群授权时必须失败。
