# fb2 AI Center 交接

## 当前快照

日期：2026-06-21

## 2026-06-21 线上验证快照

- 真实群聊补充验证：账号 `123qwe` 已通过 fb2 外部应用会话绑定到主项目用户，群 `ext_fb2_official` 可发送可见 `@EL` 消息；实测 `Context Pack` 和 `match_analysis_brief` 已返回该用户本人订单，但 AI 回复曾被超时的补充 `search_user_orders` 结果干扰。
- 主项目已修复提示和工具规划规则：Context Pack `user_orders` 与 `match_analysis_brief.data.user_orders` 都算当前用户订单来源；`search_user_orders unavailable` 只表示补充展开失败，不能否定已有本人订单事实。
- 主项目 prompt metadata 新增 `context_fact_summary`，把比赛/本人订单/群消息数量及少量 source id 投影到 metadata，避免模型漏看长 Context Pack 中已有的订单来源。
- 第二轮真实群聊验证发现 `match_analysis_brief` 成功返回 8 条本人订单，但 executed tool JSON 因大赔率数据在 6000 字处截断，导致 `data.user_orders` 明细没进入 prompt；主项目已补 `tool_fact_summary` 和更详细的 `context_fact_summary.user_order_samples`，把本人订单样例提前投影到截断前。
- 第三轮真实群聊验证已成功让 AI 引用并分析本人订单 `order_id`、金额、状态和首个选项；随后发现自动反馈回写偶发 `send request` 失败，主项目已把 generated-answer feedback callback 改为携带 `X-FB2-AI-CONTEXT-USER-ID`，并在首次 HTTP 传输失败后使用 fresh client 重试。
- 第四轮真实群聊验证定位到线上主项目有 `HTTP_PROXY/HTTPS_PROXY/ALL_PROXY`，且 fb2 固定 IP 不在 `NO_PROXY`；curl 直连 fb2 feedback 15ms 成功，但 reqwest POST 走代理后 10 秒超时。主项目已给 fb2 Context Pack、today-matches、tool manifest、tools/execute、feedback 和 opinion-adoption 增加统一 no-proxy direct client。
- 第五轮真实群聊验证通过：可见消息 `gmsg_237cff0200a94f6d94aa61e339feaa37` 触发 AI 回复 `gai_94f0083cd1ac4a1a92c34181e40f52ef`，回复引用本人订单 `531cee5c-382a-4513-b297-5939b024fcd9` 并提示不承诺命中；主项目日志显示 `fb2 generated-answer feedback callback recorded`，fb2 `/context/feedbacks` 返回自动反馈 `68ab0efb-0660-4466-8acf-27aeaa6c3433`，`matched_cited_source_count=1`。
- 长按 `AI回复` 后端入口验证通过：对消息 `gmsg_237cff0200a94f6d94aa61e339feaa37` 调用 `/api/me/groups/ext_fb2_official/messages/{messageId}/ai-reply` 后生成 AI 回复 `gai_596b1a4309a54bf4bdaa2c398ab4eccc`；fb2 自动反馈 `dbc25e69-d677-4503-a3bf-d97638866a62` 落库，`trigger=selected_message_ai_reply`，`matched_cited_source_count=1`。
- 第六轮平台匿名订单摘要验证通过：可见消息 `gmsg_2413b6fb2c8a47e1a8bc6e8b3614b827` 触发 AI 回复 `gai_e258e05fc0b54a45991ce7d92843fd8f`，回复显式引用 `platform_order_summary:2026-06-21:all`，未泄露单个用户订单且未承诺命中；fb2 audit `a4343000-cd19-4757-9bab-5ca75f8c79aa` 含 `platform_order_summary` citation source，自动反馈 `69290519-e5ba-45da-bddf-a08945b1bd9d` 返回 `cited_source_count=1`、`matched_cited_source_count=1`。
- 第七轮群友观点验证通过：可见消息 `gmsg_35c1be9597c14098ace5a50e07beb7b9` 触发 AI 回复 `gai_530ea615bafb4215b317f200c619eaa0`，回复区分“群友观点”和“AI推断”，引用群消息 `c0910321-77b5-4ac1-a398-40615f32051e` 与比赛 `EXT-2589467`，且在 fb2 未展开具体群观点内容时明确说明信息不足；fb2 自动反馈 `116d8041-4283-4a84-9a97-ec0c73055413` 返回 `cited_source_count=2`、`matched_cited_source_count=2`。
- 第八轮“这条消息说得对吗”验证通过：先发送不带 `@EL` 的可见消息 `gmsg_7f808244d0084bf8b441fac80bf3e12a`，内容包含“西班牙让两球肯定赢盘、可以重注”，再调用长按 `AI回复` 后端入口；AI 回复 `gai_54627ba13175499ea2eef77085da3837` 基于 `EXT-2589467` 赔率和盘口事实纠正该说法，明确不承诺命中且提示重注风险；fb2 自动反馈 `062d14b9-bdba-4e43-a1f9-7bcd9c07b5b4` 返回 `trigger=selected_message_ai_reply`、`matched_cited_source_count=1`。
- 第九轮平台摘要排除验证通过：真实群 `ext_fb2_official` 可见消息 `gmsg_3bb5b3f52a644068acab708ea89eb4f4` 内容为“群里大家怎么看西班牙这场？只说群友观点和AI推断，不要平台订单汇总。”，触发 AI 回复 `gai_4e94761bf6b9439d97b4e5155dd39860`；fb2 audit 返回 `include_platform_orders=false`、`platform_summary_count=0`，说明主项目已尊重明确排除平台订单汇总的群聊意图。
- 权限负向验证通过：平台摘要缺少 `X-FB2-AI-CONTEXT-SCOPE: platform_order_summary` 返回 403；用户订单工具缺少 `X-FB2-AI-CONTEXT-USER-ID` 返回 403；Context Pack 携带 `external_user_id` 但缺少同值上下文用户头返回 403；随后 fb2 `/context/permission-summary?from=2026-06-21T09:47:00Z` 返回 `total_blocks=3`、`missing_external_user_id_count=2`、`platform_scope_count=1`。
- 质量汇总验证通过：fb2 `/context/quality-summary?from=2026-06-21T09:20:00Z` 返回 `total_packs=10`、`total_feedback=6`、`matched_cited_source_count=6`、`unmatched_cited_source_count=0`、`permission_block_rate=0.23076923076923078`；`missing_context_count=0`、`wrong_context_count=0`，但 `large_context_pack_rate=0.6` 仍提示后续要继续做上下文预算压缩。
- 主项目服务端已发布：`v0.3.556`，线上 `/api/server/version` 返回 `gitSha=78e6c17f7a4e9c48d7794b6d3d06ee280dc78742`。
- fb2 后端部署记录显示最新 AI Center 后端部署为 `f6374f27`，线上 `/health` 返回 healthy；后续 `06ce4333` 是 shop 前端/文档相关提交，不改变本轮 AI Center 后端能力。
- 主项目 live smoke 已通过：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -MainToken <123qwe主项目会话token> -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d -IncludePlatformOrderSummary`，并携带 `FB2_AI_CENTER_TOKEN` 访问 fb2 live 数据；最新结果 `failed=0 skipped=0`，覆盖 `chat-bootstrap aiReply / voice composer / billing`。
- 本轮 live smoke 已验证：主项目健康和版本、fb2 live tool manifest、Context Pack、比赛分析简报、群观点摘要、赛后复盘摘要、平台匿名订单摘要、统一工具执行 `group_opinion_summary`/`match_analysis_brief` 及其 visibility。
- 本轮已获授权在真实群 `ext_fb2_official` 发送可见 `@EL` 联调消息，并验证“我的票”正例、本人订单引用、AI 回复计费、工具执行、source reference 匹配和 fb2 feedback 自动回写。
- 测试账号 `123qwe` 对应 fb2 用户 `6fe5aa17-0403-427a-8e91-7f414beca35d`、主项目用户 `usr_13c9832b7cad4b26b50768fa961e0de4`；线上已配置大额测试余额 `balance_fen=1000000000`，无 `user_token_quota` 月限额行，`/api/me/balance` 已验证可见。
- 长按 `AI回复` 后端入口已验证；APK 侧仍需确认 UI 长按菜单能调用该接口，并检查 AI 回答 source references、fb2 feedback、opinion adoption 和权限审计。

主项目当前已经具备：

- fb2 外部应用注册、默认群和品牌配置。
- fb2 账号同步、会话创建、主项目授权登录 fb2。
- fb2 首次登录试用额度配置。
- `chat-bootstrap` 输出聊天、语音、ASR/TTS 和推荐体验协议。
- `chat-bootstrap` 已输出机器可读 `voice.composer` 契约，明确 fb2 应接 `VoiceComposerView`、开启录音浮层、系统 ASR 超时后走云端兜底。
- `android/chat-voice-kit` 已新增 `VoiceComposerBootstrap.applyFb2GroupChatConfig(...)`，fb2 可直接把主项目 `chat-bootstrap` JSON 映射为 `VoiceComposerConfig`，默认开启系统 ASR 预热、stop 后超时、云端 ASR 兜底和主项目录音浮层，避免业务页漏配 `serverFallbackEnabled/serverConfig`。
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
- 主项目工具契约、planner、grounding 和 prompt 已把 fb2 聚合工具 `match_analysis_brief`、`group_opinion_summary` 纳入聊天自动执行：比赛/今日/预测/“我的票”问题优先查 `match_analysis_brief`，群友观点/大家怎么看问题优先查 `group_opinion_summary`，再按需展开细分 search/detail 工具。
- 线上 fb2 `POST /api/main-project/tools/execute` 已验证 `group_opinion_summary` 返回 `visibility=single_group_lightweight_memory`，`match_analysis_brief` 返回 `visibility=match_focused_brief`；主项目 grounding 会按这两个 visibility 校验，缺少 source_ids 时只作为弱证据使用。
- 主项目 `/api/external/apps/fb2/context-contract` 会主动读取 fb2 `/api/main-project/context/tool-manifest`，并以 `live_tool_manifest` 返回脱敏摘要（状态、工具数量、工具 id、usage_policy/tool_selection_policy 可用性），不暴露 token 或完整大 payload。
- `live_tool_manifest.main_project_tool_execution_policy` 会把 fb2 实时 manifest 拆成 `chat_auto_executable_tool_ids`、`manifest_only_tool_ids` 和 `main_project_allowed_missing_tool_ids`。fb2 新增工具后，只有进入 `chat_auto_executable_tool_ids` 才代表主项目群聊 AI 会自动规划执行；其它工具只是发现信息、回调端点或待接入能力。
- 主项目新增 `scripts/smoke-fb2-ai-center.ps1`，用于不往生产群聊发消息的 live smoke：默认验证主项目健康、版本、context-contract 和工具覆盖；传 `FB2_AI_CENTER_TOKEN` 后验证 fb2 Context Pack、比赛分析、群观点、赛后复盘摘要；传 `-IncludePlatformOrderSummary` 后验证平台匿名摘要；传 `-ExternalUserId` 后验证本人订单上下文。
- 主项目上下文日志已补 `topic_hint_present`、`fallback_used`、`answer_policy_schema`、`context_quality_warning_count`、`tool_readiness_status`，用于排查 fb2 AI 为什么没用上业务数据。
- 群聊 AI 可拉取 fb2 Context Pack 并做预算裁剪。
- `android/chat-voice-kit` 已输出 `VoiceComposerView`、`VoiceComposerBootstrap`、录音浮层、系统 ASR、云端 ASR 兜底和 TTS。

当前仍需重点推进：

- 用真实群聊消息和真实用户票据继续扩充联调样本，确认“我的票/群观点”在不同账号权限下都返回期望数据；平台订单风险工具的匿名聚合和单群轻量群观点正向 smoke 已通过。
- 发布后抽样验证主项目群聊链路里 `user_order_context_present=true` 的日志，确认用户订单上下文已经从 fb2 进入 prompt；平台摘要仍应只在双端开关和 scope 同时开启时出现。
- 用更多账号继续抽样主项目真实群聊入口触发 `@EL` 和长按消息 `AI回复`；当前账号 `123qwe` 已验证 AI 回答能显式区分比赛事实、本人订单、群观点和 AI 推断。
- 用已完赛比赛样本验证 `opinion_result_review_summary` 和 `opinion_result_reviews` 在主项目真实群聊回答中只被描述为历史复盘/样本统计，不被写成未来命中承诺。
- fb2 接入 `VoiceComposerView` 的完整输入栏，并优先使用 `VoiceComposerBootstrap.applyFb2GroupChatConfig(...)`，而不是只接 ASR/TTS 或手写临时 Web 浮层。
- fb2 真机验证小米/HyperOS 系统 ASR 超时后云端兜底。
- 主项目和 fb2 建立固定 AI 数据回答评测集。
- 后续把 fb2 工具执行从当前的 Context Pack + 轻量工具调用继续升级为更细粒度的可评测工具链。

## 主项目负责人待办

- 保持 `/api/external/apps/fb2/context-contract` 与文档同步。
- 观察 `live_tool_manifest.status`，如果 fb2 manifest 变成 degraded/unavailable，要先修 fb2 contract 或 token/base_url，而不是让 AI 编造工具能力。
- 观察 `live_tool_manifest.main_project_tool_execution_policy.coverage_status`。如果出现 `main_project_allowed_missing_tool_ids`，说明主项目静态 allowlist 与 fb2 线上 manifest 漂移；如果 fb2 新工具长期停在 `manifest_only_tool_ids`，需要单独评估是否接入 planner、grounding 和权限规则。
- 每次 fb2 或主项目 AI Center 改动后运行：
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1`
  需要验证 live fb2 数据时先设置 `FB2_AI_CENTER_TOKEN`；需要验证平台摘要时加 `-IncludePlatformOrderSummary`；需要验证“我的票”时加 `-ExternalUserId <fb2_user_uuid>`。
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
