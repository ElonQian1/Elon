# fb2 AI Center 最终验收矩阵

## 上下文格式

| 要求 | 目标形态 | 当前证据 | 完成口径 |
|---|---|---|---|
| fb2 业务数据给主项目 AI 使用 | XML-wrapped Markdown Context Pack 作为正文，JSON metadata 和 `citation_sources` 作为机器字段 | `contracts.md` 已固定 `<fb2_context_pack>`、`context_audit_id`、`matches`、`user_orders`、`platform_order_summary`、`citation_sources` | live `/context/pack` 返回非空 `context_pack`、`context_audit_id`、业务数组和引用来源 |
| 不先上完整 MCP/RAG | REST Context Pack + tool manifest + tools/execute，MCP 只作为后续包装层 | `PLAN.md`、`contracts.md`、`smoke-fb2-ai-center.ps1` | smoke 能验证 REST 契约和工具执行策略；不要求 MCP 才算完成 |

## 主项目必须提供

| 要求 | 验收证据 | 当前状态 |
|---|---|---|
| `chat-bootstrap` | `smoke-fb2-ai-center.ps1 -MainToken` 或 `-Fb2Username/-Fb2Password` 检查默认群、语音 composer、AI 回复、计费策略 | 已用 `123qwe/123qwe` 验证 fb2 session bridge 和 authenticated bootstrap；最终验收仍需与 live Context Pack、语音证据、真实群聊写入绑定 |
| `context-contract` | 默认 smoke 检查 answer policy、六类评测场景、live manifest execution policy | 已覆盖 |
| Context Pack 拉取 | `-RequireFb2Live -RequireAllScenarios -ExternalUserId <uuid>` | 需要真实 `FB2_AI_CENTER_TOKEN` 完成最终验证 |
| tool manifest 读取 | 默认 smoke 检查 `live_tool_manifest.status=ready`、必需 tool ids、无 missing allowed tool | 已覆盖并新增必需工具清单 |
| 工具执行和审计 | `-RequireAllScenarios` + `-CheckPermissionBoundaries` + visible chat final acceptance 的 feedback/audit evidence 和 `feedback_coverage` | 需要最终验收绑定同一批 `QualitySince`，且 `visible_mention`、`selected_message`、`summary_post` 三类 feedback 覆盖完整 |
| answer policy | 默认 smoke 检查 `fb2.answer_policy.v1` 和 6 个 canonical eval scenarios | 已覆盖 |
| 最终验收 wrapper 离线回归 | `smoke-fb2-final-acceptance.ps1 -SelfTest` 检查三类 feedback coverage、子脚本 exit code、voice/quality/permission evidence 摘录和 summary success 门槛 | 已补本地自测；它只证明 wrapper 逻辑未退化，不替代 live token、真实群聊或真机语音最终证据 |
| 可见群聊回答正文策略 | `smoke-fb2-visible-chat.ps1 -AllowVisibleMessages` 检查 `@EL` 和长按 `AI回复` 回复正文含来源、事实/观点/推断分层、风险边界，且拒绝“肯定赢盘/重注”类说法；最终 summary 输出 `visible_answer_policy_evidence` | 独立 visible smoke 已于 2026-06-22 03:04 通过：`@EL` 回复 `gai_4df8a06989b149ecadf780abc1b0914d`，长按 `AI回复` 回复 `gai_37f12f3fc7da4598a44f1b622955709d`。最终验收仍需带真实 token，把 feedback/quality/permission/voice 证据一起跑进 `smoke-fb2-final-acceptance.ps1` summary |
| 群总结帖回答策略 | `smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage` 创建 summary post，并检查 summary source references、事实/观点/推断/风险分层和禁止投注保证；有 `FB2_AI_CENTER_TOKEN` 时还等待 `trigger=group_summary_post` 的 fb2 feedback | 独立 summary smoke 已于 2026-06-22 09:06 通过：summary post `gsp_46720718477f4c6e953b55d5fc309568`，`status=ready`，脚本 `failed=0 skipped=2`。总结帖模型链路已改用 `social_ai` fallback；本轮 summary 生成成功后已接入 `social_group_summary_post:<post_id>` 自动 feedback 回写。最终验收仍需与 feedback/quality/permission/voice 证据绑定 |
| billing policy | `chat-bootstrap.billing` 验证 ASR/TTS/context fetch 免费，AI 回复生成前扣费 | 需要 authenticated bootstrap 验证 |
| observability | `PROGRESS.md`/`handoff.md` 记录 server version、summary JSON、log path、feedback evidence、`feedback_coverage`、voice artifact refs、`preflight_evidence`、`final_acceptance_evidence` | 已有记录要求，最终验收未闭环 |

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
| 质量汇总 | `context_quality_summary` | `-CheckQuality` |
| 权限审计 | `context_permission_summary`、`context_audit_summary` | `-CheckPermissionBoundaries` 会触发 403 负向请求，并读取 permission summary |
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

- 缺真实 `FB2_AI_CENTER_TOKEN`，不能完成 live Context Pack、平台匿名摘要、质量汇总和 feedback 样本的最终验收。
- 缺 `finalAcceptanceReady=true` 的真实 fb2 真机语音证据 JSON，不能证明小米/HyperOS 等设备上主项目 `VoiceComposerView`、系统 ASR、云端兜底、TTS 和 ASR/TTS 免费策略已完整可用；最终证据还必须提供真实可访问的 logcat 和截图/视频 artifact。当前 ADB 半成品证据只能证明 UI/录音浮层没有静音卡死。
- 已有独立真实群聊 visible smoke 和 summary post smoke 通过，但仍缺最终 `scripts/smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages` summary，不能把真实群聊可见消息、AI 回复、总结帖、summary-post feedback、正文策略检查、source references、`feedback_coverage`、`visible_answer_policy_evidence` 和 `final_acceptance_evidence` 绑定为同一批证据。
- 缺带真实 token 的 `-CheckPermissionBoundaries` 当前运行结果，不能把历史权限负向验证当成本次最终完成证据。
