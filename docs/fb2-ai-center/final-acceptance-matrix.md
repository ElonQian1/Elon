# fb2 AI Center 最终验收矩阵

## 上下文格式

| 要求 | 目标形态 | 当前证据 | 完成口径 |
|---|---|---|---|
| fb2 业务数据给主项目 AI 使用 | XML-wrapped Markdown Context Pack 作为正文，JSON metadata 和 `citation_sources` 作为机器字段 | `contracts.md` 已固定 `<fb2_context_pack>`、`context_audit_id`、`matches`、`user_orders`、`platform_order_summary`、`citation_sources` | live `/context/pack` 返回非空 `context_pack`、`context_audit_id`、业务数组和引用来源 |
| 不先上完整 MCP/RAG | REST Context Pack + tool manifest + tools/execute，MCP 只作为后续包装层 | `PLAN.md`、`contracts.md`、`smoke-fb2-ai-center.ps1` | smoke 能验证 REST 契约和工具执行策略；不要求 MCP 才算完成 |

## 主项目必须提供

| 要求 | 验收证据 | 当前状态 |
|---|---|---|
| `chat-bootstrap` | `smoke-fb2-ai-center.ps1 -MainToken` 或 `-Fb2Username/-Fb2Password` 检查默认群、语音 composer、AI 回复、计费策略 | 最新 data-only summary 已用 `123qwe/123qwe` 通过 fb2 session bridge 验证 authenticated bootstrap、AI 回复入口和 billing；full final 仍需语音证据 |
| `context-contract` | 默认 smoke 检查 answer policy、六类评测场景、live manifest execution policy | 已覆盖 |
| Context Pack 拉取 | `-RequireFb2Live -RequireAllScenarios -ExternalUserId <uuid>` | 已在 `data-only-acceptance-20260622T103826Z` 用 service token 验证：today pack、my-ticket pack、平台摘要、群观点和 citation sources 均通过 |
| tool manifest 读取 | 默认 smoke 检查 `live_tool_manifest.status=ready`、必需 tool ids、无 missing allowed tool | 已覆盖并新增必需工具清单 |
| fb2 dynamic discovery | 默认 smoke 直连 fb2 `/integration`，检查路由就绪、token header、关键端点和官方群映射；无 token 时 readiness/tool-manifest 必须 401；带 token 时验证 authenticated readiness 和 direct manifest tool id | 最新 data-only summary 已验证 `/integration`、authenticated readiness、direct manifest tool ids 与主项目 contract 对齐 |
| readiness 运行时使用 | 主项目拉 Context Pack 前读取 fb2 `/context/readiness`，把结果写入 `preflight_readiness`，并把非 ready 状态提升为 `context_quality.warnings` 和 `context_fact_summary.preflight_readiness`；`blocked` 时工具执行记录 skipped，prompt 里要出现 `<tool_gap_summary>` 数据缺口 | live readiness 已带 token 读取并记录 `status=partial`；Context Pack 内容级 live 复核已通过。`blocked` 场景仍作为后续退化回归，不是当前 data-only 阻塞 |
| 工具执行和审计 | `-RequireAllScenarios` + `-CheckPermissionBoundaries` + visible chat final acceptance 的 feedback/audit evidence 和 `feedback_coverage` | 已在 `data-only-acceptance-20260622T103826Z` 同一批 `QualitySince` 验证权限负向、质量汇总和三类 feedback 覆盖 `3/3` |
| answer policy | 默认 smoke 检查 `fb2.answer_policy.v1` 和 6 个 canonical eval scenarios | 已覆盖 |
| 非语音 data-only 验收 | `smoke-fb2-ai-center.ps1 -DataOnlyAcceptance` 或最终 wrapper `-DataOnlyAcceptance` | 已通过：`target\fb2-ai-center\data-only-acceptance-20260622T103826Z.json`，`success=true`、`voice_status=deferred_by_user`。该项不能替代最终语音验收 |
| 真机语音证据采集 | `collect-fb2-voice-device-evidence.ps1 -CaptureHoldGesture` 生成 `fb2.voice_device_evidence.v1` JSON 和 screenshot/UI dump/logcat artifact，再由 `smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence` 验证 | 已新增采集器；默认输出 `finalAcceptanceReady=false`，只作为证据采集路径。最终完成仍必须人工确认所有 UI/ASR/TTS/免费策略 checks 并用长期 artifact 支撑 |
| 语音证据脚本离线回归 | `smoke-fb2-ai-center.ps1 -SelfTest` 检查 final-ready 正例、严格布尔字段、artifact 解析/占位拒绝、logcat/视觉证据、低 APK 和 ASR/TTS 必需项失败路径 | 已补本地自测；它只证明主 smoke 的语音证据门槛不会退化，不替代真实 fb2 APK 的 `finalAcceptanceReady=true` 真机证据 |
| 最终验收 wrapper 离线回归 | `smoke-fb2-final-acceptance.ps1 -SelfTest` 检查三类 feedback coverage、子脚本 exit code、voice/quality/permission evidence 摘录和 summary success 门槛 | 已补本地自测；它只证明 wrapper 逻辑未退化，不替代 live token、真实群聊或真机语音最终证据 |
| 可见群聊回答正文策略 | `smoke-fb2-visible-chat.ps1 -AllowVisibleMessages` 检查 `@EL` 和长按 `AI回复` 回复正文含来源、事实/观点/推断分层、风险边界，且拒绝“肯定赢盘/重注”类说法；最终 summary 输出 `visible_answer_policy_evidence` | 已在 data-only final wrapper 同批通过：`@EL` 回复 `gai_ccbd283a451a4629b8cc028c85b32b22`，selected-message 回复 `gai_d93a114f0a284a25973a7fa8b7ca93ad`，均有 direct group read 和正文策略证据 |
| 群总结帖回答策略 | `smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage` 创建 summary post，并检查 summary source references、事实/观点/推断/风险分层和禁止投注保证；有 `FB2_AI_CENTER_TOKEN` 时还等待 `trigger=group_summary_post` 的 fb2 feedback | 已在 data-only final wrapper 同批通过：summary post `gsp_014861b660a64f0aaaf69cc8c05c4546 status=ready`，`trigger=group_summary_post` feedback 已回写，matched=3、unmatched=0 |
| billing policy | `chat-bootstrap.billing` 验证 ASR/TTS/context fetch 免费，AI 回复生成前扣费 | 最新 data-only summary 已验证 `chat-bootstrap billing`、AI reply gate 和 `external_context_fetch` 免费；语音免费策略仍需 final-ready 真机语音证据配合验证 |
| observability | `PROGRESS.md`/`handoff.md` 记录 server version、summary JSON、log path、feedback evidence、`feedback_coverage`、voice artifact refs、`preflight_evidence`、`final_acceptance_evidence` | 非语音 observability 已绑定到 `data-only-acceptance-20260622T103826Z`；full final 仍缺 voice artifact refs |

## fb2 必须提供

| fb2 能力 | live tool id / endpoint | smoke 证据 |
|---|---|---|
| Context Pack | `context_pack` / `/context/pack` | `scenario: today matches context pack`、`scenario: my ticket context pack` |
| 今日比赛 | `today_matches` | live manifest 必需 tool id；Context Pack 和 match brief 有比赛数据 |
| 比赛分析简报 | `match_analysis_brief` | `scenario: match analysis brief` |
| 群观点摘要 | `group_opinion_summary` | `scenario: group opinions summary` |
| 用户订单 | `search_user_orders`、`user_orders` | `scenario: my ticket has user orders`，必须带 `external_user_id` 和同值用户头 |
| 平台匿名摘要 | `platform_orders` | `scenario: platform order risk`，必须带 platform scope |
| 反馈写入 | `record_context_feedback` | visible final acceptance 的 generated-answer feedback |
| 反馈查询 | `list_context_feedbacks` | quality feedback samples |
| 质量汇总 | `/context/quality-summary` 集成端点、`context_feedback_summary`、`context_audit_summary` | `-CheckQuality` |
| 非合成质量 readiness | `/context/feedback-summary?exclude_synthetic=true`、`/context/quality-summary?exclude_synthetic=true`、`/context/opinion-adoption-summary?exclude_synthetic=true` | `-RequireNonSyntheticQualityReadiness`；`-FinalAcceptance` 自动启用，默认要求非合成反馈 >= 1、群观点采纳 >= 1 |
| 权限审计 | `/context/permission-summary` 集成端点、`context_audit_summary` | `-CheckPermissionBoundaries` 会触发缺用户头、用户头不匹配、缺平台 scope、用户订单工具缺头等 403 负向请求，并读取 permission summary |
| 工具 manifest | `tool_manifest` | 默认 smoke 和 live data smoke |

## 用户场景

| 场景 | 必需数据 | 验收方式 |
|---|---|---|
| 今天比赛怎么看 | match、odds、context audit、citation source | live Context Pack + `match_analysis_brief` |
| 帮我分析我的票 | 当前用户订单、关联比赛、引用来源 | `ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d` 或其它有订单用户；不能返回他人订单 |
| 平台今天订单风险怎么样 | 匿名聚合平台摘要 | `-IncludePlatformOrderSummary` 且带 platform scope；禁止单用户明细 |
| 群里大家怎么看这场 | group message、opinion memory、AI 推断分层 | `group_opinion_summary` + visible chat feedback |
| 这条消息说得对吗 | selected message、match/odds、context audit | `smoke-fb2-visible-chat.ps1` selected-message `/ai-reply` |
| 总结今天群聊讨论 | group messages、topic hint、context audit、citation sources、summary-post feedback | `smoke-fb2-visible-chat.ps1 -SkipMention -SkipSelectedMessage` summary post；必须区分数据事实、群友观点、AI 推断和风险边界，并在有 fb2 service token 时验证 `group_summary_post` feedback |

## 还不能宣布完成的证据缺口

- 真实 `FB2_AI_CENTER_TOKEN` 已用于最新 data-only 验收，并完成 authenticated readiness/direct manifest、live Context Pack、平台匿名摘要、权限、质量汇总和三类 feedback 样本验证；token 仍不应写入仓库或文档。
- ASR/TTS 当前按安排暂停；`-DataOnlyAcceptance` 已沉淀非语音数据闭环证据，但终极完成口径仍必须在恢复语音后补齐 `-FinalAcceptance`。
- 缺 `finalAcceptanceReady=true` 的真实 fb2 真机语音证据 JSON，不能证明小米/HyperOS 等设备上主项目 `VoiceComposerView`、系统 ASR、云端兜底、TTS 和 ASR/TTS 免费策略已完整可用；最终证据还必须提供真实可访问的 logcat 和截图/视频 artifact。当前 ADB 半成品证据只能证明 UI/录音浮层没有静音卡死。
- full final 默认 `MinOpinionAdoptionCount=1`；data-only visible 验收默认同样保持该门槛，不再自动为短窗口真实群 smoke 降为 0。只有显式传 `-AllowNoNewOpinionAdoptionInShortWindow` 时，才允许本轮不新增真实 non-synthetic opinion adoption，且该 opt-out 不能作为 full final 证据。
- 真实群聊可见消息、AI 回复、总结帖、summary-post feedback、正文策略检查、source references、`feedback_coverage`、`visible_answer_policy_evidence` 和 `final_acceptance_evidence` 已在 data-only wrapper 同批绑定；full final 仍需把同类证据与 `finalAcceptanceReady=true` 语音证据同批绑定。
- 带真实 token 的权限负向已在最新 data-only 验收中通过：缺当前用户头、用户头不匹配、缺平台 scope、用户订单工具缺头均返回 403，并记录 permission summary。
