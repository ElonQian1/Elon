# fb2 AI Center 交接

## 当前快照

日期：2026-06-22

## 2026-06-21 线上验证快照

- 2026-06-22 09:06 当前主项目代码和线上状态：本轮 worktree 已先 fast-forward 到 `origin/main` 最新 `b80c5e95`，随后只补 `docs/fb2-ai-center/` 交接记录。fb2 总结帖相关运行代码已在提交 `1d41cb5a` 和 `225bfc6f` 中推送并发布，线上服务端为 `v0.3.588 / 225bfc6f0d9d33552f60dfd96a220753b3f7f7b6`；后续 `96bf5ce4` 是 smoke 脚本误判修正，已推送到远端但不需要服务端重新发布。
- 2026-06-22 09:06 总结帖入口抽样通过：真实群 `ext_fb2_official` 使用 `123qwe/123qwe` 创建 summary post `gsp_46720718477f4c6e953b55d5fc309568`，最终 `status=ready`，`scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage -Fb2Username 123qwe -Fb2Password <redacted> -PollTimeoutSec 120` 返回 `failed=0 skipped=2`。脚本检查了非空 summary、source references、`数据事实 / 群友观点 / AI推断 / 风险边界`、风险提示和禁止投注保证。
- 2026-06-22 09:29 本轮把总结帖入口补进质量闭环：主项目 summary post 生成成功后会后台调用 fb2 `/context/feedback`，`main_request_id=social_group_summary_post:<post_id>`、`trigger=group_summary_post`。`scripts\smoke-fb2-visible-chat.ps1` 的 summary-only 场景在有 `FB2_AI_CENTER_TOKEN` 时会等待 summary-post feedback；缺 token 时仍只验证可见 summary 正文策略。
- 2026-06-22 09:29 本轮收紧真机语音证据：`scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence` 必须看到 `finalAcceptanceReady=true` 才能通过；`scripts\smoke-fb2-final-acceptance.ps1` summary 会输出设备、APK、按住说话、上滑取消、too short、system ASR、server ASR、TTS、ASR/TTS 免费和附件证据字段。示例 JSON 默认 `finalAcceptanceReady=false`，只能作为格式模板。
- 2026-06-22 09:56 本轮继续收紧最终验收 summary：`scripts\smoke-fb2-final-acceptance.ps1` 输出 `feedback_coverage`，并把最终 `success` 绑定到三类自动 feedback 覆盖完整：`visible_mention`、`selected_message`、`summary_post`。以后不能只看 feedback 数组非空，要看 `feedback_coverage.complete=true`。
- 2026-06-22 09:56 本轮继续收紧真机语音 artifact：`scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence` 会拒绝占位 artifact ref，要求本地 ref 解析到真实文件或远端 `http(s)://` URL，并要求至少一条 logcat 和一条截图/视频证据；最终 summary 会摘录 artifact refs 是否完整、是否有 logcat/视觉证据。
- 2026-06-22 10:30 本轮给最终总验收 wrapper 增加 `-SelfTest` 离线自测，覆盖三类 feedback 解析、三类 feedback 分别缺失的失败路径、visible/final 子脚本 exit code、voice/quality/permission evidence 摘录和最终 `success` 的布尔门槛。该命令不需要 token、不访问 fb2、不写真实群，后续改 wrapper 前后都应先跑它。
- 2026-06-22 10:52 本轮给主 smoke 增加 `-SelfTest` 离线自测，并把真机语音证据校验抽到 `scripts/fb2-ai-center-voice-evidence.ps1`。后续改 `-RequireVoiceDeviceEvidence` 规则前后都要跑 `scripts\smoke-fb2-ai-center.ps1 -SelfTest`，它会检查严格布尔值、APK 版本、必需 ASR/TTS/UI 字段、artifact 路径/URL、占位 ref 拒绝、logcat 和截图/视频门槛，但不替代真实 APK 的 final-ready 设备证据。
- 2026-06-22 11:15 当前主项目状态复核：`HEAD=origin/main=cb8f5aff`，`scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password <redacted>` 通过，线上版本 `0.3.592 / 37625843aa50b433d9469b8a9c175551d061075d`，live manifest `tool_count=31`。当前环境未配置 `FB2_AI_CENTER_TOKEN` 和 `FB2_VOICE_DEVICE_EVIDENCE_PATH`，所以不能进入最终 `-PreflightOnly` / `-AllowVisibleMessages` 验收。
- 2026-06-22 11:20 fb2 子项目状态复核：本地 `D:\rust\active-projects\fb2` 落后 `origin/main` 约 59 个提交，并且有本地改动/未跟踪文件，主项目会话不要在该目录直接修改、pull 或覆盖。fb2 远端已提供 `/integration`、`/context/readiness`、`/tool-manifest`、`/context/pack`、比赛/赔率、本人订单、群观点、平台匿名摘要、feedback/quality/permission/audit、`/tools/execute` 和受控 `match-context-index/refresh`；主项目下一轮应按 live 合同动态发现和消费。
- 2026-06-22 09:06 总结帖修复根因：旧 summary post `gsp_b4c717d3c2d947188ccc755fe4f6ff32` 进入 `ready_with_fallback`，错误为“当前 AI 模型额度已用尽或接口不可用”。这不是 fb2 用户余额不足，也不是 ASR/TTS 免费策略问题，而是总结帖链路此前只调用默认模型，没有使用 `social_ai` 多代理 fallback。现在总结帖和 `@EL`/长按 `AI回复` 使用同一类模型 fallback，`hunyuan-turbo` fallback 已在线上正常生成。
- 2026-06-22 09:06 ADB 真机复核：设备 `e0d909c3` 在线，fb2 包 `com.duoguan.football 1.1.48(96)`，`RECORD_AUDIO granted=true`，appops 为 `foreground/allow`。启动 `com.duoguan.football/.MainActivity` 后，当前页面为 `夺冠体育官方群`，UI dump 可见 `数据事实`、`AI推断`、`风险边界`、`context_audit_id`、`按住 说话`；截图位于 `target\fb2-current-20260622.png`，UI dump 位于 `target\fb2-window-20260622.xml`。本轮 logcat 未见 fb2 `AndroidRuntime/FATAL`。
- 2026-06-22 03:04 当前主项目远端和线上状态：`origin/main` 最新为 `4b0fb9dd363e3619faab7bf73c3ded680e1ad40e`，线上服务端为 `v0.3.585 / 4b0fb9dd363e3619faab7bf73c3ded680e1ad40e`，其中包含 fb2 群聊 AI 分层兜底修复 `589d2bacf51cf4c679505da52d8ecfea1762420b`。本轮工作树 `D:\rust\active-projects\elon-main-fb2-docs-20260621` 已 fast-forward 到 `origin/main` 并保持干净。
- 2026-06-22 03:04 可见群聊 smoke 重新通过：`@EL` 消息 `gmsg_b2d834caf30c4265acd638cb3868bf21` -> AI 回复 `gai_4df8a06989b149ecadf780abc1b0914d`；selected-message seed `gmsg_a71960917eeb494f8993c4e43adb927d` -> AI 回复 `gai_37f12f3fc7da4598a44f1b622955709d`。`scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <redacted>` 返回 `failed=0 skipped=0`，并通过来源、事实/推断分层、风险边界、禁止投注保证和反驳“肯定赢盘/重注”的正文检查。
- 2026-06-22 03:04 ADB 真机抽样：Xiaomi `23116PN5BC` 上 fb2 `1.1.48(96)` 可打开 `聊天 -> 🏆 夺冠体育官方群`，群列表摘要和群详情页都能看到主项目 AI 的 `数据事实 / AI推断 / 风险边界 / 来源 / context_audit_id` 分层回复，底部输入栏显示 `按住 说话`。logcat 未见 fb2 `AndroidRuntime/FATAL`；这证明真实群聊 AI 回复和主项目式输入栏已在当前 APK 可见，但仍不替代完整语音 ASR/TTS final acceptance。
- 2026-06-22 主项目 answer policy/prompt 已进一步收紧：所有使用 fb2 外部上下文的群聊 AI 回复都应显式使用 `数据事实：`、`用户订单：`、`平台汇总：`、`群友观点：`、`AI推断：`、`风险边界：` 等短标签；涉及比赛、赔率、票据、推荐、预测或今日比赛讨论时，至少要输出 `数据事实`、`AI推断` 和 `风险边界`，风险边界必须说明赛果不确定、不保证命中、不建议重注或梭哈。长按 `AI回复` 对“肯定赢盘、稳赢、稳赚、包赢、重注、梭哈”等被选中消息会明确按过度确定/诱导投注处理；可见群聊 smoke 的投注保证判定也已改为识别否定/反驳语境，避免把“不要重注/不宜稳赢”误判为违规。
- 2026-06-22 ADB 真机阶段验证：主项目会话在 `Xiaomi 23116PN5BC / Android 16 / HyperOS OS3.0` 上验证 fb2 APK `com.duoguan.football 1.1.48(96)`，系统 ASR 为 `com.xiaomi.mibrain.speech/.asr.AsrService`，录音权限和 appops 正常。已确认 `夺冠体育官方群` 页面具备主项目式 `按住 说话` 输入栏，文本/语音切换、绿色录音气泡、`取消 / AI回复 / 转文字 / 发送` 控制区、上滑取消和静音转文字后 UI 恢复均可用；证据文件为 `docs/fb2-ai-center/voice-device-evidence-20260622-adb.json`。该证据明确 `finalAcceptanceReady=false`，因为本轮未包含人工语音样本，尚未证明 system ASR final、云端 ASR 成功、TTS 播放和余额为 0 时 ASR/TTS 免费。
- 2026-06-22 语音证据门槛复核：使用上述 JSON 运行 `scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence`，脚本正确通过 UI/录音项，并在 `finalAcceptanceReady`、`tooShort`、system ASR final、云端 ASR 兜底、服务端 ASR 成功/失败恢复、TTS 播放、ASR/TTS 免费策略 8 项失败，结果 `failed=8 skipped=2`。这份 ADB 证据只能证明“主项目式语音 UI 已在 fb2 APK 出现且不会静音卡死”，不能替代最终真机语音完成证据。
- 2026-06-21 质量门槛补强：主项目 `scripts/smoke-fb2-ai-center.ps1` 新增 `-CheckQuality`、`-RequireFeedbackCoverage`、`-QualitySince/-QualityUntil`、`-MaxLargeContextPackRate`、`-MaxCitationUnmatchedRate`、`-MaxMissingContextRate`、`-MaxWrongContextRate`、`-MinFeedbackCount`、`-MinMatchedCitedSourceCount`。日常无副作用 smoke 仍默认轻量；携带 `FB2_AI_CENTER_TOKEN` 后可升级验证 fb2 `/context/quality-summary` 和 `/context/feedbacks`，把 missing/wrong context、引用未命中、大包率和反馈覆盖变成自动门槛。
- 2026-06-21 完整场景 smoke 补强：`scripts/smoke-fb2-ai-center.ps1 -RequireAllScenarios` 现在会要求 Context Pack 有 `context_audit_id`，今日比赛/比赛分析/我的票等场景有实际数据数量，并要求关键场景存在 `citation_sources`，减少只检查字段存在导致的假通过。
- 2026-06-21 chat-bootstrap 语音契约 smoke 补强：`scripts/smoke-fb2-ai-center.ps1` 现在可通过 `-Fb2Username/-Fb2Password` 或 `FB2_USER_TOKEN` 无副作用桥接主项目 token，并在 authenticated `chat-bootstrap` 中自动检查 `VoiceComposerView`、`VoiceComposerBootstrap.applyFb2GroupChatConfig(...)`、`SERVER_PROCESSING`、`AI_REPLY`、系统 ASR 本地优先、云端 ASR 兜底、ASR/TTS 免费和 AI 回复扣费门槛，防止 fb2 再退化成只接 ASR/TTS 能力。
- 2026-06-21 fb2 APK 发布 smoke 补强：`scripts/smoke-fb2-ai-center.ps1 -CheckFb2ApkVersion` 会检查 fb2 `/api/app-version` 至少达到 `1.1.48`、`update_kind=full_apk`、checksum/size 有效，并 HEAD 实际 `apk_url` 确认为 APK 下载响应，用来防止只验证后端而漏掉用户端完整 APK 发布。
- 2026-06-21 本地语音 SDK 构建 smoke 补强：`scripts/smoke-fb2-ai-center.ps1 -CheckLocalVoiceSdkBuild` 会执行 `android\gradlew.bat :chat-voice-kit:assembleDebug --quiet`，把 `VoiceComposerBootstrap`/`VoiceComposerView` 是否仍可被 fb2 编译引用纳入同一巡检脚本。
- 2026-06-21 最终验收门槛补强：`scripts/smoke-fb2-ai-center.ps1 -RequireNoSkips` 会在任何检查被 skip 时失败，避免缺少主项目 token、`FB2_AI_CENTER_TOKEN` 或 live 覆盖时误判“终极目标已完成”。
- 2026-06-22 最终总验收开关补强：`scripts/smoke-fb2-ai-center.ps1 -FinalAcceptance` 会自动打开 live fb2 场景、完整场景、平台摘要、质量反馈、fb2 APK 发布、主项目语音 SDK 构建和 no-skip 门槛。当前环境没有 `FB2_AI_CENTER_TOKEN` 时该命令应失败，这代表终极验收条件仍未满足，而不是脚本异常。
- 2026-06-22 最终预检门槛补强：`scripts/smoke-fb2-final-acceptance.ps1 -PreflightOnly` 现在会在不发送真实群消息的情况下，先强制跑 fb2 live 数据、六类标准场景、平台匿名摘要、fb2 APK 发布、主项目语音 SDK 构建、真机语音证据和 no-skip 门槛；通过后才进入 `-AllowVisibleMessages` 的真实群聊验收。
- 2026-06-22 终极目标矩阵补强：`docs/fb2-ai-center/final-acceptance-matrix.md` 记录上下文格式、主项目能力、fb2 能力、用户场景和剩余证据缺口；默认 smoke 同步检查 live manifest 必需工具，防止 fb2 端工具/接口从线上 manifest 漂移消失。
- 2026-06-22 权限负向验收补强：`scripts/smoke-fb2-ai-center.ps1 -CheckPermissionBoundaries` 会触发缺用户头 Context Pack、缺 platform scope 平台摘要、缺用户头用户订单工具三个 403 检查，并读取 `/context/permission-summary`，证明拒绝访问已进入审计；`-FinalAcceptance` 和最终预检会自动打开。
- 2026-06-22 最终 summary 证据补强：`scripts/smoke-fb2-final-acceptance.ps1` 会在 summary JSON 中写入 `preflight_evidence` / `final_acceptance_evidence`，摘录主项目版本、live manifest、fb2 APK、语音证据、场景 audit、权限审计和质量反馈关键检查，减少只看 exit code 的误判空间。
- 2026-06-21 SDK 构建复核通过：`cd android && .\gradlew.bat :chat-voice-kit:assembleDebug` 成功，确认主项目当前 `android/chat-voice-kit` 可产出 debug AAR，fb2 可继续引用 `VoiceComposerBootstrap` 和 `VoiceComposerView`。
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
- fb2 用户端完整 APK 已发布：`1.1.46 / versionCode 94`，代码提交 `add05196 feat(chat): use main voice composer bootstrap` 和 `782a2c1e chore(user): publish apk 1.1.46` 已推到 fb2 `origin/main`；线上 `/api/app-version` 返回 `update_kind=full_apk`、`checksum=sha256:b4b65bec80ed69455ac4f0ef4b82c0a8a0ce5ed93fc6de26d8678947ab73b84e`，远端 `football-user-v1.1.46.apk` 与 `football-user-latest.apk` hash 一致。
- 主项目 live smoke 已通过：`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -MainToken <123qwe主项目会话token> -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d -IncludePlatformOrderSummary`，并携带 `FB2_AI_CENTER_TOKEN` 访问 fb2 live 数据；最新结果 `failed=0 skipped=0`，覆盖 `chat-bootstrap aiReply / voice composer / billing`。
- 本轮 live smoke 已验证：主项目健康和版本、fb2 live tool manifest、Context Pack、比赛分析简报、群观点摘要、赛后复盘摘要、平台匿名订单摘要、统一工具执行 `group_opinion_summary`/`match_analysis_brief` 及其 visibility。
- 本轮已获授权在真实群 `ext_fb2_official` 发送可见 `@EL` 联调消息，并验证“我的票”正例、本人订单引用、AI 回复计费、工具执行、source reference 匹配和 fb2 feedback 自动回写。
- 测试账号 `123qwe` 对应 fb2 用户 `6fe5aa17-0403-427a-8e91-7f414beca35d`、主项目用户 `usr_13c9832b7cad4b26b50768fa961e0de4`；线上已配置大额测试余额 `balance_fen=1000000000`，无 `user_token_quota` 月限额行，`/api/me/balance` 已验证可见。
- 2026-06-21 复核：`123qwe` 是 fb2 账号，主项目直登 `account=123qwe` 返回“账号不存在或已停用”；正确链路是 fb2 `/api/main-project/session` 桥接到主项目账号 `15692409898`，当前 `/api/me/balance` 返回 `balance_fen=999999876`，足够继续 AI 回复联调。
- 2026-06-21 复核：`GET /api/external/apps/fb2/chat-bootstrap` 对桥接 token 返回 `defaultGroupId=ext_fb2_official`、`voice.asr.billing=free_auth_and_limits_only`、`voice.tts.billing=free_auth_and_limits_only`、`billing.gates.beforeAiReplyGeneration=check_balance_or_trial_credit`、`VoiceComposerBootstrap.applyFb2GroupChatConfig(...)`。
- 2026-06-21 复核：fb2 `/context/pack?external_user_id=6fe5aa17-0403-427a-8e91-7f414beca35d&topic_hint=帮我分析我的票` 返回 `success=true`、`user_orders=6`、`matches=6`、`citation_sources=13`、`context_audit_id=028e2d63-f42b-4483-a607-9567e2114abf`；带 `X-FB2-AI-CONTEXT-SCOPE: platform_order_summary` 后平台匿名摘要也返回成功。
- 2026-06-21 真实群复核通过：可见消息 `gmsg_4c0b8693032f418d9ee38d2010aaeaa9` 触发 AI 回复 `gai_d80ab422454345b78f6db70264cfdd25`，约 10 秒完成；回复按“数据事实 / 我的订单 / 平台汇总 / 群友观点 / AI推断”分层，并继续保持不承诺命中。
- 长按 `AI回复` 后端入口已验证；APK 侧仍需确认 UI 长按菜单能调用该接口，并检查 AI 回答 source references、fb2 feedback、opinion adoption 和权限审计。
- fb2 用户端完整 APK 已发布 `1.1.48 / versionCode 96`，代码提交 `41f8fbc3 feat(chat): surface main project AI replies` 和 `e2202266 docs(ai-center): record chat ai apk release` 已推到 fb2 `origin/main`；线上 `/api/app-version` 返回 `update_kind=full_apk`、`checksum=sha256:1456304d1275b8333a93c82c46c019a068e566edcf494bbdffe1a01c8787141d`，远端 `football-user-v1.1.48.apk` 与 `football-user-latest.apk` hash 一致。
- 主项目已新增显式授权可见群聊 smoke：`scripts/smoke-fb2-visible-chat.ps1`。脚本默认拒绝写群，只有传 `-AllowVisibleMessages` 才会发送真实消息；支持用 fb2 用户账号桥接主项目 token，不需要手工复制 bearer。
- 2026-06-21 可见群聊 smoke 已通过：命令 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <redacted> -PollTimeoutSec 120`；`@EL` 消息 `gmsg_b0760a14e0b54d508043a7da1d46e2d4` 触发回复 `gai_64dea8a838864730aff30fccb1f27069`，selected-message seed `gmsg_42443de0304f4975b46248f0417d5708` 触发 `AI回复` 回复 `gai_512c68f7355942f1b9e111c8250fcc16`，结果 `failed=0 skipped=0`。
- 2026-06-21 后续真实群复核发现：Context Pack 和工具链仍能拉到 `123qwe` 的 fb2 数据，但主项目模型生成层返回“当前 AI 模型额度已用尽或接口不可用”，因此真实群只生成失败兜底文案，fb2 feedback 里 `matched_cited_source_count=0`。这是模型供应/运行配置层问题，不是 fb2 用户余额问题；`123qwe` 桥接主项目余额仍足够。
- 本轮修复了两个可见群 smoke 暴露的问题：`latest_unanswered_group_social_ai_mention` 现在排除 `usr_elon_ai` 自己发出的群消息，避免失败兜底文案里的 `@EL` 再次触发群聊 AI；`scripts/smoke-fb2-visible-chat.ps1` 在配置 `FB2_AI_CENTER_TOKEN` 后改为通过 fb2 feedback 的 `main_request_id` 反查 `social_group_message:*` 和 `social_group_selected_message:*`，避免群里并发回复时误抓其它 `gai_*` 消息。
- 本轮本地验证通过：`cargo test social_ai_pending`、`cargo fmt --check`、`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage -Fb2Username 123qwe -Fb2Password <redacted>`。完整真实群 smoke 仍需等主项目模型生成层恢复后再跑，否则会继续因无引用来源而失败。
- 本轮已发布主项目服务端：提交 `c797bf2d`，服务器版本 `v0.3.559`，线上 `/health=OK`，`/api/server/version.gitSha=c797bf2d309aaac8e248c1c26c7f9b67a27a0145`。发布后可见群单条 `@EL` smoke 发送 `gmsg_3f34e14c9ae945b18ae0f7780d53ac34`，收到回复 `gai_5840b26ede824f0bb5289bf7f9396e5e`；日志确认仍是模型生成层不可用导致兜底文案，但 8 秒后未出现 `trigger_message_id=gai_5840...`，说明 AI 自己回复不再二次触发 `@EL`。
- 2026-06-21 20:51 本地新增主项目 `social_ai` 多代理 fallback：`@EL` 与长按 `AI回复` 先用默认代理，遇到模型供应/接口类错误（例如“当前 AI 模型额度已用尽或接口不可用”、provider 超时、rate limit、endpoint inactive）会按已配置代理顺序尝试备用代理；用户余额不足、封禁、token 月限额、计费系统错误不会 fallback。本地验证已通过 `cargo fmt --check`、`cargo test social_ai_agents`、`cargo test social_ai`、`cargo check --bin elon-server`。该修复仍需提交、发布并重新跑真实群 visible smoke，才能确认线上 AI 生成层恢复。
- 2026-06-21 21:12 线上复核发现 `@EL` 在多代理 fallback 后已能生成真实回答，但长按 `AI回复` 的 selected-message 链路仍返回兜底文案；服务器日志定位为备用模型要求 `system` 消息只能出现在最开始，而 selected-message 请求构造了两个连续 `system` 消息。主项目已改为把长按专用指令合并进首个 `system` prompt，本地验证通过 `cargo fmt --check`、`cargo test social_ai_message_reply`、`cargo check --bin elon-server`；仍需提交、发布并重新跑 selected-message visible smoke。
- 2026-06-21 21:35 线上 selected-message 复核已能生成真实回答并正常计费，但反馈日志显示 `cited_source_count=0`，原因是回复使用了 fb2 比赛/赔率上下文却未显式写出 `EXT-*` 或其它 source id。主项目已把“使用 fb2 外部上下文必须写出来源 ID 或 label，否则说信息不足”的规则加入 `social_ai` 基础 prompt，并补 `base_prompt_requires_fb2_source_references` 测试；本地验证通过 `cargo fmt --check`、该单测和 `cargo check --bin elon-server`，仍需发布并重新跑 visible smoke 验证 cited source 计数。
- 2026-06-21 21:52 线上 selected-message 复核再次超时，日志显示 fallback 仍按字典序尝试 `copilot:*` 代理，其中 `copilot:gpt-4o` 请求拖到约 2 分钟。主项目已把 `social_ai` fallback 候选限制为 `usage_mode=server_api_key` 且排除 `copilot:*` / `api.githubcopilot.com`，避免用户 token/CLI 类代理进入实时群聊 AI 生成链路；本地验证通过 `cargo fmt`、`cargo test social_ai_agents`、`cargo check --bin elon-server`，仍需发布后重新跑 visible smoke。
- 2026-06-21 22:20 线上 selected-message 复核在 `v0.3.568 / 7e8d200b` 已 11 秒内生成回复，且不再尝试 `copilot:*`，但 feedback 仍显示 `cited_source_count=0`。主项目继续补齐 selected-message 来源闭环：prompt 显式传入 `selected_message_id` 并要求末尾标注来源，回复后处理在模型遗漏时自动追加 `selected_message_id`，feedback payload 同时把被长按消息作为 `selected_message` citation source 合并，避免“这条消息说得对吗”场景没有可审计来源；本地验证通过 `cargo test selected_message_source_uses_stable_shape`、`cargo test payload_merges_extra_selected_message_source`、`cargo test selected_message_source_is`、`cargo check --bin elon-server`，仍需发布后重新跑 visible smoke 验证 `cited_source_count>=1`。
- 2026-06-21 22:47 线上 selected-message 复核在 `v0.3.570 / e807f73f` 已生成回复且自动追加 `selected_message_id`，但发现模型可能从最近聊天历史复制旧 `context_audit_id`。主项目已在 selected-message 回复后处理里把可见回复中的 `context_audit_id` 强制替换/补充为本次 Context Pack 的当前 audit，避免 source line 出现历史 audit；本地验证通过 `cargo test selected_message_reply_`、`cargo check --bin elon-server`，仍需发布后重新跑 visible smoke。
- 2026-06-21 22:56 线上 selected-message 最终复核通过：服务端 `v0.3.571 / eb064a64`，可见 seed `gmsg_d5dc444a694147a29d9dc22bc4076b7c` 触发回复 `gai_d60572a9041e4c2baef6a8b95e21f927`；回复正文包含当前 `context_audit_id b0537db0-9cab-445e-a7f4-e0406d474eaf` 和 `selected_message_id=gmsg_d5dc444a694147a29d9dc22bc4076b7c`，服务端 feedback 日志记录 `trigger=selected_message_ai_reply`、`cited_source_count=2`。本轮修复后，“这条消息说得对吗”已具备真实群可见回复、当前来源引用、selected message 来源和自动反馈记录。

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
- `answer_policy_contract.eval_scenarios` 固定六个机器可读验收场景：今日比赛、我的票、平台匿名订单摘要、群友观点、长按消息复核、来源审计；`scripts/smoke-fb2-ai-center.ps1` 默认检查这些场景的 id、权限边界、必需来源、必需引用和禁止输出，避免契约漂移。
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
- `scripts/smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password 123qwe` 已验证 fb2 session bridge 能换取主项目 token，`ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`，authenticated `chat-bootstrap` 能返回主项目语音 composer、ASR/TTS 免费策略和 AI 回复扣费策略；这条 smoke 不发送群聊消息。
- 主项目新增 `scripts/smoke-fb2-visible-chat.ps1`，用于获得明确授权后验证真实群聊可见入口：发送 `@EL`、调用 selected-message `/ai-reply`、等待 `usr_elon_ai`/`gai_*` 回复；默认没有 `-AllowVisibleMessages` 时会失败退出，避免无意写入生产群。
- `scripts/smoke-fb2-visible-chat.ps1` 现在还会检查真实 AI 回复正文：`@EL` 和 selected-message `AI回复` 都必须带来源标记、事实/观点/推断分层、风险边界，并避免“肯定命中/稳赢/重注/包赢”等投注保证；selected-message 场景还必须明确反驳被测消息里的“肯定赢盘、重注”说法。
- `scripts/smoke-fb2-final-acceptance.ps1` 的最终 summary 已新增 `visible_answer_policy_evidence`，把真实群聊回复正文策略证据和 `feedback_evidence`、`final_acceptance_evidence` 放在同一批验收 JSON 中。
- 主项目上下文日志已补 `topic_hint_present`、`fallback_used`、`answer_policy_schema`、`context_quality_warning_count`、`tool_readiness_status`，用于排查 fb2 AI 为什么没用上业务数据。
- 群聊 AI 可拉取 fb2 Context Pack 并做预算裁剪。
- `android/chat-voice-kit` 已输出 `VoiceComposerView`、`VoiceComposerBootstrap`、录音浮层、系统 ASR、云端 ASR 兜底和 TTS。

当前仍需重点推进：

- 用真实群聊消息和真实用户票据继续扩充联调样本，确认“我的票/群观点”在不同账号权限下都返回期望数据；平台订单风险工具的匿名聚合和单群轻量群观点正向 smoke 已通过。
- 发布后抽样验证主项目群聊链路里 `user_order_context_present=true` 的日志，确认用户订单上下文已经从 fb2 进入 prompt；平台摘要仍应只在双端开关和 scope 同时开启时出现。
- 用更多账号继续抽样主项目真实群聊入口触发 `@EL` 和长按消息 `AI回复`；当前账号 `123qwe` 已验证 AI 回答能显式区分比赛事实、本人订单、群观点和 AI 推断。
- 用已完赛比赛样本验证 `opinion_result_review_summary` 和 `opinion_result_reviews` 在主项目真实群聊回答中只被描述为历史复盘/样本统计，不被写成未来命中承诺。
- fb2 `1.1.48` 已接入主项目群聊可见回复刷新和长按 `AI回复` 客户端入口；仍需在小米/HyperOS 真机上验证系统 ASR 超时后云端兜底、录音浮层、直接发语音、转文字和 APK UI 长按菜单。
- 主项目最终验收脚本已把 fb2 真机语音证据纳入 `-FinalAcceptance`，证据格式见 `docs/fb2-ai-center/voice-device-evidence.example.json`；没有 `-VoiceDeviceEvidencePath` 时最终验收必须失败。
- 主项目和 fb2 建立固定 AI 数据回答评测集。
- 后续把 fb2 工具执行从当前的 Context Pack + 轻量工具调用继续升级为更细粒度的可评测工具链。

## 主项目负责人待办

- 保持 `/api/external/apps/fb2/context-contract` 与文档同步。
- 观察 `live_tool_manifest.status`，如果 fb2 manifest 变成 degraded/unavailable，要先修 fb2 contract 或 token/base_url，而不是让 AI 编造工具能力。
- 观察 `live_tool_manifest.main_project_tool_execution_policy.coverage_status`。如果出现 `main_project_allowed_missing_tool_ids`，说明主项目静态 allowlist 与 fb2 线上 manifest 漂移；如果 fb2 新工具长期停在 `manifest_only_tool_ids`，需要单独评估是否接入 planner、grounding 和权限规则。
- 每次 fb2 或主项目 AI Center 改动后运行：
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1`
  需要验证 live fb2 数据时先设置 `FB2_AI_CENTER_TOKEN`；需要验证平台摘要时加 `-IncludePlatformOrderSummary`；需要验证“我的票”时加 `-ExternalUserId <fb2_user_uuid>`。
- 需要验证 authenticated `chat-bootstrap` 的语音 SDK 契约时，传 `-MainToken <token>`，或传 `-Fb2Username/-Fb2Password` 让脚本通过 fb2 session bridge 自动获取主项目 token；这条 smoke 不会写真实群聊。
- 需要验证 fb2 用户端 APK 发布状态时加 `-CheckFb2ApkVersion`；默认最低版本为 `1.1.48`，可用 `-MinFb2ApkVersion` 临时提高门槛。
- 需要把主项目本地语音 SDK 编译也纳入验收时加 `-CheckLocalVoiceSdkBuild`。
- 最终验收或 CI 不允许跳过任何检查时加 `-RequireNoSkips`。
- 需要验证 fb2 真机语音链路时，加 `-RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath <json>`；正式证据必须覆盖 `VoiceComposerView`、按住说话、上滑取消、三段底部操作区、系统 ASR、云端 ASR 兜底、TTS 和 ASR/TTS 免费策略。
- 真机语音证据的 `artifacts[].ref` 必须是真实可访问证据：本地路径按 evidence JSON 所在目录或仓库根目录解析，远端路径必须是 `http(s)://`；不能使用 example/placeholder/“saved file path” 文案，并且必须同时包含 logcat 和截图/视频。
- 最终总验收直接跑 `-FinalAcceptance`，并同时提供主项目登录来源、`FB2_AI_CENTER_TOKEN` 和 `-VoiceDeviceEvidencePath`；否则必须失败，不能把 skip 当成完成。
- 需要验证长期质量门槛时加：
  `-CheckQuality -RequireFeedbackCoverage -QualitySince <RFC3339> -MaxLargeContextPackRate 0.75`
  该检查会读取 fb2 `/context/quality-summary` 和 `/context/feedbacks`，用于发现 missing/wrong context、引用未命中和 Context Pack 大包率退化。
- 需要验证权限负向门槛时加：
  `-CheckPermissionBoundaries -ExternalUserId <fb2_user_uuid>`
  该检查会读取 fb2 `/context/permission-summary`，用于证明未授权订单/平台摘要请求会被拒绝并记录审计。
- 需要验证真实群聊可见入口时，必须确认用户已授权写生产群或提供沙盒群，再运行：
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages`
  没有 `-AllowVisibleMessages` 时脚本必须保持失败退出；有授权执行时，脚本必须同时通过回复正文策略检查，不能只看 AI 回复消息 ID。
- 如果只想抽样总结帖入口，可跳过 `@EL` 和 selected-message：
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage -Fb2Username 123qwe -Fb2Password <redacted> -PollTimeoutSec 120`
  该命令会创建真实 summary post，并检查 summary 是否具备 source references、事实/观点/推断/风险分层和禁止投注保证；如果同时提供 `FB2_AI_CENTER_TOKEN`，脚本还会等待 `trigger=group_summary_post` 的 fb2 feedback 回写。
- 如果只需要确认当前“可见群聊正文策略”是否仍健康，可用 `123qwe/123qwe` 跑上面的 visible smoke；如果要宣布最终完成，必须改用 `smoke-fb2-final-acceptance.ps1`，并同时提供 `FB2_AI_CENTER_TOKEN` 和完整 `VoiceDeviceEvidencePath`，让 feedback、quality、permission、APK、语音和真实群聊证据绑定到同一份 summary。
- 修改最终验收 wrapper 逻辑后先跑：
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -SelfTest`
  这只验证 wrapper 自身解析、evidence 摘录和 success 门槛，不替代 live token、真实群聊或真机语音最终证据。
- 修改主 smoke 的语音证据验收逻辑后先跑：
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -SelfTest`
  这只验证离线合成证据的通过/失败路径，不替代真实 fb2 APK 的 ASR/TTS 真机证据。
- 最终总验收优先运行，语音证据必须是 `finalAcceptanceReady=true`：
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <token> -VoiceDeviceEvidencePath <json>`
  `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <token> -VoiceDeviceEvidencePath <json>`
  先用 `-PreflightOnly` 做无副作用预检；该阶段会自动从 fb2 登录解析 `ExternalUserId`，预检该用户有订单上下文，并要求 fb2 live 数据、六类标准场景、平台匿名摘要、fb2 APK 发布、主项目语音 SDK 构建、真机语音证据和 no-skip 全部通过。预检通过后再用 `-AllowVisibleMessages` 写真实群聊，把真实群聊可见触发和 `-FinalAcceptance` 绑定到同一批 `QualitySince` 证据，并输出 summary JSON；其中 `feedback_coverage.complete` 必须为 true。
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
- 主项目不要修改 fb2 本地脏工作区；需要 fb2 改 handler、route、repository、schema 或 tool contract 时，把具体接口差距交给 fb2 会话处理。
