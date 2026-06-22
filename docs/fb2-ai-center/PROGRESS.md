# fb2 AI Center 当前进度

## 当前状态

- 工作目录：`D:\rust\active-projects\elon-main-fb2-docs-20260621`
- 分支：`main`
- 远端：`origin/main`
- 当前代码状态以 `git status -sb` 和 `git log -1 --oneline` 为准；每轮收尾必须在最终回复里给出本轮提交 SHA。
- 任务性质：主项目侧 AI Center、聊天/语音 SDK、上下文注入、验收脚本和文档交接。

## 已完成

- 2026-06-22 15:25 `3cf62536` 已推送 `origin/main` 并发布到主项目服务端 `v0.3.610`，线上 `/health=OK`，`/api/server/version` 返回 `gitSha=3cf625361727990807e044630b3b56e8040476a7`。线上 `/api/external/apps/fb2/context-contract` 已确认返回 `domain_context_projection_contract.schema=fb2.domain_context_projection.v1`、`format.wrapper=fb2_context_pack`、7 个必需小节、10 类 source kinds 和反模式清单。`pwsh -File scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password <redacted> -SkipVoiceContractChecks` 通过，结果 `failed=0 skipped=1`；唯一跳过项是未提供 `FB2_AI_CENTER_TOKEN`，所以 live fb2 Context Pack 场景仍未在 service-token 权限下验收。
- 2026-06-22 15:10 本轮把 repo map/RCP 讨论落成 fb2 域数据机器契约：主项目新增 `domain_context_projection_contract`，公开 `fb2.domain_context_projection.v1`，固定 XML-wrapped Markdown `fb2_context_pack`、`match_facts`、`user_order_slice`、`platform_order_summary`、`group_opinion_slice`、`retrieval_evidence`、`quality_feedback`、source registry、召回理由、权限投影、质量闭环和反模式。`scripts\smoke-fb2-ai-center.ps1` 已检查该契约，防止 fb2 AI 输入退化为原始 HTML、大 JSON、embedding dump 或无来源摘要。
- 2026-06-22 14:40 根据当前安排暂停 ASR/TTS 继续处理后，本轮新增非语音独立验收路径：`scripts\smoke-fb2-ai-center.ps1 -DataOnlyAcceptance` 会强制检查 live fb2 数据、六类场景、平台匿名摘要、APK、权限负向审计、质量反馈、非合成 feedback 和群观点采纳，同时用 `-SkipVoiceContractChecks` 明确跳过 chat-bootstrap 的 ASR/TTS/VoiceComposer 断言；`scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance` 可用于无副作用预检或真实群聊可见验收，不再要求 `VoiceDeviceEvidencePath`，summary 写入 `voice_status=deferred_by_user`。该模式只证明比赛/订单/平台摘要/群观点 AI 数据闭环，不替代最终语音验收。
- 2026-06-22 14:25 本轮继续处理非语音 AI 数据质量契约：主项目 `context_observability_contract` 新增 `non_synthetic_feedback_count`、`opinion_adoption_count`、`opinion_memory_ref_count` 三个推荐指标，并加入 recommended log fields；这让 `/api/external/apps/fb2/context-contract` 也能公开真实反馈、群观点采纳和观点记忆引用的长期观测口径，不只依赖 smoke 脚本。
- 2026-06-22 14:10 本轮把 fb2 新增的非合成质量 readiness 纳入主项目侧验收：`scripts\smoke-fb2-ai-center.ps1` 新增 `-RequireNonSyntheticQualityReadiness`、`-MinNonSyntheticFeedbackCount`、`-MinOpinionAdoptionCount`，会用 `exclude_synthetic=true` 同时读取 fb2 `feedback-summary`、`quality-summary`、`opinion-adoption-summary`，确认真实反馈计数、quality/feedback 计数一致、群观点采纳和 memory refs 可观测；`-FinalAcceptance` 自动启用。`scripts\smoke-fb2-final-acceptance.ps1` 已透传阈值并把非合成反馈/采纳/记忆引用写入 summary evidence。当前仍缺真实 `FB2_AI_CENTER_TOKEN` 和 final-ready 语音证据，不能完成最终验收。
- 2026-06-22 13:40 本轮在远端推进后重新 fast-forward 到 `origin/main=82e042227074939b01fcbe5c32319277ea425f37`；线上 `/api/server/version` 返回 `v0.3.605 / 82e042227074939b01fcbe5c32319277ea425f37`。`smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password <redacted>` 通过，authenticated `chat-bootstrap` 和 live manifest 继续健康，仅因缺 `FB2_AI_CENTER_TOKEN` 跳过 live fb2 data。用采集器重新生成的 ADB 半成品证据已记录 `mainProjectCommit=82e04222`，`smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence` 仍按预期失败在 `finalAcceptanceReady=false` 和缺失 ASR/TTS/免费策略 checks 上。
- 2026-06-22 13:35 本轮新增主项目侧 ADB 真机语音证据采集器：`scripts\collect-fb2-voice-device-evidence.ps1` 可保存 screenshot、UI dump、logcat、包版本、权限、系统 ASR 服务，并生成 `fb2.voice_device_evidence.v1` JSON。脚本默认 `finalAcceptanceReady=false`，不会把半成品证据当作最终完成；只有测试者已经用人工语音样本确认所有 UI/ASR/TTS/免费策略检查项，并为每个 `Observed*` 开关保留对应 artifact 时，才允许传 `-MarkFinalReady`。本轮用 `e0d909c3` 跑 `-CaptureHoldGesture` 成功生成 `target\fb2-voice-device-evidence\latest-adb-check\voice-device-evidence.json` 和 7 个 artifact；随后 `smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence` 按预期失败在 `finalAcceptanceReady=false` 和 10 个缺失 checks 上，证明采集器不会误放行最终验收。
- 2026-06-22 13:20 本轮复核当前主项目代码、线上契约和真机状态：主项目工作树干净，`HEAD=origin/main=ffa817befdbb046c69615574711a1cea70fd7b69`；线上 `/api/server/version` 返回 `v0.3.604 / ffa817befdbb046c69615574711a1cea70fd7b69`。`scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password <redacted>` 通过，fb2 session bridge 解析 `ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`，authenticated `chat-bootstrap` 继续验证 `VoiceComposerView`、`VoiceComposerBootstrap`、`ChatVoiceEventSink`、系统 ASR 本地优先、云端 ASR 兜底、ASR/TTS 免费、AI 回复扣费和 live manifest `tool_count=34`。当前仍未配置 `FB2_AI_CENTER_TOKEN`，因此 live Context Pack、平台摘要、质量汇总和权限最终验收仍按预期跳过。
- 2026-06-22 13:20 本轮验证最终验收门禁不会误写群：`scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -Fb2Username 123qwe -Fb2Password <redacted>` 在缺 `FB2_AI_CENTER_TOKEN` 时立即失败，输出 `FB2_AI_CENTER_TOKEN or -Fb2AiCenterToken is required`，不会进入可见群聊写入阶段。PowerShell 解析、`smoke-fb2-final-acceptance.ps1 -SelfTest`、`smoke-fb2-ai-center.ps1 -SelfTest`、`smoke-fb2-visible-chat.ps1 -SelfTest` 均通过。
- 2026-06-22 13:20 ADB 真机再次复核 fb2 语音 UI：设备 `e0d909c3`，fb2 `com.duoguan.football 1.1.48(96)`，`RECORD_AUDIO granted=true`，appops `foreground/allow`，系统 ASR 为 `com.xiaomi.mibrain.speech/.asr.AsrService`。当前前台是 `com.duoguan.football/.MainActivity`，群页 `夺冠体育官方群` 底部为 `按住 说话`；按住手势期间绿色浮层显示 `正在听...` 和 `取消 / AI回复 / 转文字 / 发送` 控制区，释放后回到 `按住 说话`，未出现 `识别中/准备中` 卡住，logcat 未见 `FATAL EXCEPTION`，并记录到 fb2 的 `MediaRecorder/AudioRecord` 录音启动和释放。证据位于 `target\adb-fb2-20260622-hold-cancel-1313\`；该证据仍只证明 UI/录音回收，不替代 `finalAcceptanceReady=true` 的 ASR/TTS 最终证据。
- 2026-06-22 本轮补齐 fb2 Context Pack / today-matches 响应归一化回归测试：`external_app_context_response` 现在用本地 TCP 假响应覆盖 HTTP 500、非法 JSON、`success=false`、today-matches 空数据和 Context Pack `metrics.budget_status=too_large`。测试固定了主项目边界：失败响应必须是 `status=unavailable`，空比赛数据必须保留 `empty_matches` 质量告警，超预算 Context Pack 必须进入 `fb2_budget_too_large`，避免 AI 在 fb2 数据缺失或过大时误当成完整事实来源。
- 2026-06-22 本轮补齐主项目 generated-answer feedback 的工具来源归因：`external_app_context_feedback` 现在会在 Context Pack `citation_sources` 和 selected-message 额外来源之外，继续扫描 `external_app.executed_tools.v1.results`；只有 `success=true` 且 `grounding.status=grounded/weak` 的工具结果、并且 AI 回复正文显式提到对应 `source_id` 时，才会把该工具来源写入 fb2 `/context/feedback` 的 `cited_sources`。若 source id 已在 Context Pack 候选中，会复用原 `kind/id/label`；否则按工具名和 id 前缀生成 `match/order/group_message/platform_order_summary/group_opinion_memory/tool_result` 等最小来源，避免工具补充数据无法进入质量闭环。已新增单测覆盖 grounded 工具来源合并，以及 unsafe 或未提及工具来源不回写。
- 2026-06-22 本轮给真实群聊可见 smoke 增加无副作用 `-SelfTest`：`scripts/smoke-fb2-visible-chat.ps1 -SelfTest` 用合成回复验证 `@EL` / selected-message / summary-post 的正文策略规则，覆盖必须引用来源、必须区分数据事实/AI推断/风险边界、缺来源或缺风险边界必须失败、AI 自己说“肯定赢盘/重注”必须失败、否定式“不能保证/不建议重注”不能误判，以及 selected-message 和 summary 只是在引用被复核原文或“相关发言”时不能被当成 AI 投注承诺。该自测不需要 token、不访问 fb2、不发送群消息，适合在真实群写入前先守住回答策略回归门槛。
- 2026-06-22 本轮继续补强主项目 AI prompt 投影：`external_app_context_budget` 的 `context_fact_summary` 现在显式包含 `preflight_readiness.status` 和少量 `warnings`，让 `fb2_readiness_blocked/degraded/unavailable/not_configured` 不只藏在大 JSON 里；`external_app_context_tool_prompt` 新增 `<tool_gap_summary>`，把 `skipped/failed/unavailable` 工具结果提前投影，并明确这些只是数据缺口，不能被 AI 编造成比赛、赔率、订单或群友观点事实。已新增单测覆盖 readiness 摘要进入 prompt、skipped readiness gap 在工具 JSON 之前出现、以及工具缺口不能作为事实使用。
- 2026-06-22 本轮把 AI 数据接入格式原则集中写入 `contracts.md`：fb2 给主项目 AI 的唯一主正文是 XML-wrapped Markdown Context Pack；JSON 只承载 compact metadata、citation sources、tool contract、usage/answer policy 和 readiness；禁止把原始 HTML、巨大 JSON、全量数据库、embedding 或未裁剪订单明细直接塞进 prompt；MCP/RAG 只能作为现有 REST Context Pack、tool manifest、tools/execute 的后续包装层，不能另立事实源。`data-tools.md` 同步补充按需工具选择规则，避免 manifest-only 工具被误当作聊天 AI 自动执行能力。
- 2026-06-22 11:35 本轮把 fb2 `/api/main-project/context/readiness` 从 smoke 验收信号推进到主项目运行时：`external_app_context_readiness` 新增 authenticated live preflight 拉取和归一化，Context Pack / today-matches 回退上下文都会注入 `preflight_readiness`；`context_quality.warnings` 会把 `blocked/degraded/unavailable/not_configured` 映射为 `fb2_readiness_*`，让 AI 回答显式提示数据链路缺口；工具执行层遇到明确 `preflight_readiness.status=blocked` 时会跳过深层 fb2 工具调用并记录 `external_app.executed_tools.v1 status=skipped`，避免在 fb2 自检认为上下文不足时继续假装可查明细。已新增单测覆盖 readiness 归一化、质量警告和 blocked 工具跳过。运行代码提交 `9d778940` 已推送并发布，线上先验证到 `v0.3.593 / 9d778940452dc2ceed615e4f4b1ac0a78908ec73`，`/health=OK`，默认 smoke 和 `123qwe/123qwe` authenticated bootstrap smoke 均通过。
- 2026-06-22 09:06 补齐“总结帖/群聊总结入口”的主项目侧验收闭环：提交 `1d41cb5a` 增加 fb2 总结帖回答契约和 smoke 场景，要求输出 `数据事实`、`群友观点`、`AI推断`、`风险边界` 并保留 source references；提交 `225bfc6f` 把总结帖生成从单一默认模型改为 `social_ai` 多代理 fallback，避免默认模型额度/接口不可用时只生成兜底文案；提交 `96bf5ce4` 放宽总结帖脚本对“相关发言”原文引用的误判。上述提交均已推送到 `origin/main`，其中服务端运行代码已发布到 `v0.3.588 / 225bfc6f0d9d33552f60dfd96a220753b3f7f7b6`，`96bf5ce4` 为脚本验收修正，无需重新发布服务端。
- 2026-06-22 09:06 总结帖真实群 smoke 已通过：命令 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage -Fb2Username 123qwe -Fb2Password <redacted> -PollTimeoutSec 120` 在真实群 `ext_fb2_official` 创建 summary post `gsp_46720718477f4c6e953b55d5fc309568`，最终 `status=ready`，脚本结果 `failed=0 skipped=2`。该总结帖回复通过非空 summary、source references、事实/观点/推断/风险分层、风险边界和禁止投注保证检查。
- 2026-06-22 09:29 本轮把总结帖生成结果接入 fb2 自动 feedback 回写：主项目 `spawn_group_summary_generation` 现在会在 summary 更新成功后，以 `main_request_id=social_group_summary_post:<post_id>`、`trigger=group_summary_post` 调用 fb2 `/context/feedback`，让“总结今天群聊讨论”也进入质量汇总和失败样本闭环。`scripts/smoke-fb2-visible-chat.ps1` 的 summary-only 场景在提供 `FB2_AI_CENTER_TOKEN` 时会等待这条 summary-post feedback；缺 token 时仍只做可见群 summary 正文策略检查。
- 2026-06-22 09:29 本轮收紧真机语音最终证据门槛：`scripts/smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence` 现在强制要求证据 JSON 顶层 `finalAcceptanceReady=true`，`scripts/smoke-fb2-final-acceptance.ps1` summary 会摘录设备、APK、VoiceComposer、按住说话、上滑取消、too short、system ASR、server ASR、TTS、ASR/TTS 免费和证据附件字段。示例模板默认 `finalAcceptanceReady=false`，不能被误当成完成证据。
- 2026-06-22 09:56 本轮继续收紧最终验收 summary：`scripts/smoke-fb2-final-acceptance.ps1` 现在会输出 `feedback_coverage`，显式标记 `visible_mention`、`selected_message`、`summary_post` 三类自动 feedback 是否都覆盖；最终 `success` 必须同时满足 visible smoke 通过、no-skip acceptance 通过、三类 feedback 覆盖完整，避免只看 feedback 数组而漏掉某个入口。
- 2026-06-22 09:56 本轮继续收紧真机语音证据 artifact：`scripts/smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence` 现在会拒绝空 ref、示例/placeholder ref；本地 artifact 必须能按证据 JSON 所在目录或仓库根目录解析为真实文件，远端 artifact 必须是 `http(s)://` URL；证据还必须至少包含一条 logcat 和一条 screenshot/video 类型附件。`scripts/smoke-fb2-final-acceptance.ps1` summary 会摘录 `voice_evidence_artifact_refs_complete`、`voice_evidence_artifact_logcat`、`voice_evidence_artifact_visual`。
- 2026-06-22 10:30 本轮给最终总验收 wrapper 增加无副作用 `-SelfTest`：它用合成日志验证 `visible @EL`、selected-message `AI回复`、summary-post 三类 fb2 feedback 能被解析进 `feedback_coverage`，分别缺 visible/selected/summary 任一类时都会报告缺项；同时验证 visible smoke 失败、final acceptance 失败或 feedback 覆盖不完整时最终 `success` 不会误为 true，并验证 voice/quality/permission 等关键 OK 行能映射到 `final_acceptance_evidence`。该自测不需要 `FB2_AI_CENTER_TOKEN`、不访问 fb2、不发送群消息，适合在拿不到最终 token/真机证据时继续守住验收脚本本身的回归门槛。
- 2026-06-22 10:52 本轮把主 smoke 的真机语音证据校验抽到 `scripts/fb2-ai-center-voice-evidence.ps1`，并给 `scripts/smoke-fb2-ai-center.ps1` 增加无副作用 `-SelfTest`。自测覆盖 final-ready 正例、artifact 解析变体、`finalAcceptanceReady=false`、字符串布尔值、占位 artifact、缺本地文件、缺 logcat、缺截图/视频、空 artifact、低 APK 版本和 system ASR 缺失等路径；它不需要 token、不访问 fb2、不写群，只证明语音证据门槛不会被脚本改动放松。
- 2026-06-22 11:15 复核当前代码和线上契约状态：主项目工作树 `D:\rust\active-projects\elon-main-fb2-docs-20260621` 干净，`HEAD=origin/main=cb8f5aff`；`scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password <redacted>` 通过，主项目线上版本返回 `0.3.592 37625843aa50b433d9469b8a9c175551d061075d`，live fb2 manifest 返回 `tool_count=31`，authenticated `chat-bootstrap` 继续验证 `VoiceComposerView`、本地 ASR 优先、云端兜底、ASR/TTS 免费和 AI 回复扣费门槛。当前环境没有 `FB2_AI_CENTER_TOKEN`、`FB2_VOICE_DEVICE_EVIDENCE_PATH`、`ELON_MAIN_TOKEN`、`FB2_USER_TOKEN`，因此最终验收仍不能执行。
- 2026-06-22 11:20 子项目只读复核：`D:\rust\active-projects\fb2` 本地 `main` 落后 `origin/main` 约 59 个提交，且存在本地改动 `docs/AI_CONTEXT_24X7_OPERATIONS.md` 和未跟踪脚本 `scripts/refresh_main_project_match_context_index.ps1`，主项目会话不要在该目录直接修改或拉取覆盖。fb2 远端当前已实现 `/api/main-project/integration`、`/context/readiness`、`/context/tool-manifest`、`/context/pack`、比赛/赔率、本人订单、群观点、平台匿名摘要、feedback/quality/permission/audit、`/tools/execute` 和受控 `match-context-index/refresh`；主项目下一步应按这些 live 合同消费，不应硬编码旧接口清单或绕过 Context Pack 读取 fb2 数据库。
- 2026-06-22 09:06 本轮修复过一个线上根因：`gsp_b4c717d3c2d947188ccc755fe4f6ff32` 曾返回 `ready_with_fallback`，错误为“当前 AI 模型额度已用尽或接口不可用”，这不是 fb2 用户余额或 ASR/TTS 计费问题，而是总结帖链路没有使用群聊 AI 的模型 fallback。修复后 `gsp_400c852f06054a9eba16c8b643a3ae73` 和 `gsp_46720718477f4c6e953b55d5fc309568` 均进入 `ready`，模型 fallback 使用 `hunyuan-turbo` 正常生成。
- 2026-06-22 09:06 ADB 真机复核再次确认当前 fb2 APK 已具备主项目式聊天体验：设备 `e0d909c3` 上 fb2 `com.duoguan.football 1.1.48(96)`，`RECORD_AUDIO granted=true`，appops 为 `foreground/allow`；启动 `com.duoguan.football/.MainActivity` 后，`夺冠体育官方群` 页面可见主项目 AI 回复包含 `数据事实`、`AI推断`、`风险边界`、`context_audit_id`、`selected_message_id`，底部输入栏显示 `按住 说话`。截图证据保存在 `target\fb2-current-20260622.png`，UI dump 保存在 `target\fb2-window-20260622.xml`；本轮 logcat 未见 fb2 `AndroidRuntime/FATAL`。
- 2026-06-22 03:04 复核主项目 git/线上/真机链路：主项目修复提交 `589d2bacf51cf4c679505da52d8ecfea1762420b`（`修复fb2群聊AI回答缺少分层边界`）已推入 `origin/main`，并包含在当前线上最新 `v0.3.585 / 4b0fb9dd363e3619faab7bf73c3ded680e1ad40e` 中。该修复在 fb2 外部上下文下对群聊 `@EL` 和长按 `AI回复` 回复做后处理兜底：如果模型漏掉短标签，会补齐 `数据事实：`、`AI推断：` 和 `风险边界：`，风险边界明确“不保证命中、不建议重注或梭哈”。本轮验证命令包括 `cargo test social_ai --bin elon-server`、`cargo test social_ai_message_reply --bin elon-server`、`cargo test external_app_context_answer_policy --bin elon-server`、pre-push `cargo check --workspace`、`publish-server.ps1`、`smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <redacted>` 和 `smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password <redacted>`。
- 2026-06-22 03:04 真实群聊可见 smoke 已通过，不再是“未验证”：账号 `123qwe` 通过 fb2 session bridge 解析为 `ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`，目标群 `ext_fb2_official`。`@EL` 可见消息 `gmsg_b2d834caf30c4265acd638cb3868bf21` 触发 AI 回复 `gai_4df8a06989b149ecadf780abc1b0914d`；selected-message seed `gmsg_a71960917eeb494f8993c4e43adb927d` 触发 AI 回复 `gai_37f12f3fc7da4598a44f1b622955709d`。脚本结果 `failed=0 skipped=0`，两条回复均通过来源标记、事实/推断分层、风险边界、禁止投注保证和反驳“肯定赢盘/重注”检查。
- 2026-06-22 03:04 ADB 真机复核已把真实群聊 AI 结果带到用户端：设备 `Xiaomi 23116PN5BC` 上安装 fb2 `com.duoguan.football 1.1.48(96)`，`RECORD_AUDIO granted=true`，启动 `com.duoguan.football/.MainActivity` 无 `AndroidRuntime/FATAL`。进入“聊天 -> 🏆 夺冠体育官方群”后，聊天列表摘要显示 `数据事实：...`，群聊详情页可见 selected-message AI 回复正文包含 `数据事实`、`AI推断`、`风险边界`、`来源`、`context_audit_id` 和 `selected_message_id`；底部输入栏显示主项目式 `按住 说话`。本轮 logcat 只见 Google Play/系统网络超时和卫星电话能力探测噪声，未见 fb2 崩溃。
- 2026-06-22 主项目回答策略继续收紧：`fb2.answer_policy.v1` 的 `prompt_answer_rules` 现在要求使用 `数据事实：`、`用户订单：`、`平台汇总：`、`群友观点：`、`AI推断：`、`风险边界：` 等短标签；凡涉及比赛、赔率、票据、推荐、预测或今日比赛讨论，至少要输出 `数据事实`、`AI推断` 和 `风险边界`，并明确赛果不确定、不保证命中、不建议重注或梭哈。群聊基础 prompt 和长按 `AI回复` prompt 同步采用该口径；可见群聊 smoke 也补了 negation-aware 的投注保证判定，避免把“不要重注/不宜稳赢”误判为诱导。已验证 `cargo fmt --check`、PowerShell 解析、`cargo test external_app_context_answer_policy --bin elon-server`、`cargo test social_ai --bin elon-server`、`cargo test social_ai_message_reply --bin elon-server`、`smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password 123qwe`。
- 2026-06-22 ADB 真机阶段验证已完成并记录到 `docs/fb2-ai-center/voice-device-evidence-20260622-adb.json`：设备 `Xiaomi 23116PN5BC / Android 16 / HyperOS OS3.0`，fb2 APK `com.duoguan.football 1.1.48(96)`，系统 ASR `com.xiaomi.mibrain.speech/.asr.AsrService`，录音权限和 appops 正常。实测进入 `夺冠体育官方群` 后可见主项目式 `按住 说话` 输入栏，文本/语音切换可用，按住后出现绿色录音气泡和 `取消 / AI回复 / 转文字 / 发送` 控制区，上滑取消可恢复，静音释放到 `转文字` 后 10 秒内回到 `按住 说话`，未复现永久卡在“识别中”。本轮没有人工语音样本，未证明 system ASR final、云端 ASR 成功、TTS 播放和余额为 0 时 ASR/TTS 免费，因此该 JSON 明确 `finalAcceptanceReady=false`，不能作为最终完成证据。
- 2026-06-22 已用 `smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence-20260622-adb.json` 复核这份半成品证据：脚本正确通过 UI/录音相关项，并在 `finalAcceptanceReady`、`tooShort`、`systemAsrSuccess`、`systemAsrTimeoutServerFallback`、`serverAsrSuccess`、`serverAsrFailureRecoversUi`、`ttsPlayback`、`asrTtsFreeWithZeroAiBalance` 8 项上失败，结果为 `failed=8 skipped=2`。这证明最终验收不会被 ADB 静音阶段证据误放行。
- 2026-06-22 可见群聊验收门槛已补强：`scripts/smoke-fb2-visible-chat.ps1` 现在会检查 `@EL` 和 selected-message `AI回复` 的回复正文，要求包含来源标记、事实/观点/推断分层词、风险或不保证边界，并禁止“肯定命中/稳赢/重注/包赢”等投注保证；selected-message 场景还要求明确反驳被测消息中的“肯定赢盘、重注”说法。`scripts/smoke-fb2-final-acceptance.ps1` 的最终 summary 已新增 `visible_answer_policy_evidence`，用于沉淀这些正文策略证据。
- 2026-06-22 已用 `123qwe/123qwe` 完成 authenticated `chat-bootstrap` 无副作用验证：`scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password 123qwe` 通过，fb2 session bridge 解析主项目 token 成功，`ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`，并验证 `chat-bootstrap` 返回 `VoiceComposerView`、`VoiceComposerBootstrap`、`ChatVoiceEventSink`、系统 ASR 本地优先、云端 ASR 兜底、`/api/voice/asr`、ASR/TTS 免费、Context Pack 拉取免费和 AI 回复生成前扣费。该验证不会发送群聊消息，仍因缺 `FB2_AI_CENTER_TOKEN` 跳过 live fb2 Context Pack 场景。
- 主项目已建立 `docs/fb2-ai-center/` 工作台，固定主项目与 fb2 的分工：主项目提供 AI Center 和聊天/语音能力，fb2 提供业务数据。
- 主项目已提供 fb2 `chat-bootstrap` 和 `context-contract` 验收项，覆盖聊天、ASR、TTS、AI 回复、计费和 answer policy。
- 主项目 smoke 脚本已能验证 live fb2 tool manifest、工具执行策略、六类固定评测场景和主项目聊天自动工具覆盖。
- `scripts/smoke-fb2-ai-center.ps1` 已支持用 `123qwe/123qwe` 这类 fb2 用户账号桥接主项目登录，验证 authenticated `chat-bootstrap`，不会发送群消息。
- 脚本已支持 `-CheckFb2ApkVersion`，验证 fb2 `/api/app-version`、APK 版本、`update_kind=full_apk`、checksum、size 和 APK 下载 HEAD。
- 脚本已支持 `-CheckLocalVoiceSdkBuild`，验证主项目 `android/chat-voice-kit` 可执行 `:chat-voice-kit:assembleDebug`。
- 脚本已支持 `-RequireNoSkips`，防止缺 token 或缺覆盖项时把 skip 当成完成。
- 脚本已支持 `-FinalAcceptance`，自动打开 live 数据、完整场景、平台摘要、质量反馈、APK、语音 SDK 构建、真机语音证据和 no-skip 门槛。
- 已新增 `scripts/smoke-fb2-final-acceptance.ps1`，支持 `-PreflightOnly` 无副作用预检，也会在写群前解析 `ExternalUserId` 并预检用户订单上下文，再把真实群聊可见触发和 `-FinalAcceptance` 绑定为同一批 `QualitySince` 证据，并输出机器可读 summary、子脚本日志路径、可见消息 ID、AI 回复 ID 和 feedback evidence。
- `-PreflightOnly` 已升级为进入真实群聊前的无副作用强门禁：除用户订单上下文和真机语音证据外，还会要求 fb2 live 数据、六类标准场景、平台匿名摘要、fb2 APK 发布、主项目语音 SDK 构建和 no-skip 全部通过。
- 已新增 `docs/fb2-ai-center/final-acceptance-matrix.md`，把终极目标拆成上下文格式、主项目能力、fb2 能力、用户场景和剩余证据缺口，作为宣布完成前的逐项审计入口。
- 默认 smoke 已新增 live manifest 必需工具检查，覆盖 `context_pack`、`today_matches`、`match_analysis_brief`、`group_opinion_summary`、用户订单、平台摘要、feedback、context audit 和 `tool_manifest`；`context_quality_summary` / `context_permission_summary` 作为受保护 HTTP 端点由 `/integration`、`-CheckQuality` 和 `-CheckPermissionBoundaries` 验证，不再要求必须出现在聊天工具 manifest id 中。
- `scripts/smoke-fb2-ai-center.ps1` 已新增 `-CheckPermissionBoundaries`，用于验证缺当前用户头、`external_user_id` 与 `X-FB2-AI-CONTEXT-USER-ID` 不一致、缺 platform scope、用户订单工具缺当前用户头都会 403，并读取 fb2 `/context/permission-summary` 证明被审计；`-FinalAcceptance` 和 `smoke-fb2-final-acceptance.ps1 -PreflightOnly` 会自动开启该门槛。
- `scripts/smoke-fb2-final-acceptance.ps1` 的 summary 已新增 `preflight_evidence` / `final_acceptance_evidence`，直接摘录 APK、语音、场景、权限和质量关键 OK 行，减少最终验收时人工翻日志的空间。
- 本轮 summary 证据补强验证通过：`smoke-fb2-final-acceptance.ps1` 解析通过；默认无副作用 smoke 仍为 `failed=0 skipped=2`；缺 `FB2_AI_CENTER_TOKEN` 时 `-PreflightOnly` 仍立即失败，不会写真实群。
- 本轮无 token 验证通过：默认 smoke 仍为 `failed=0 skipped=2`；显式 `-CheckPermissionBoundaries -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d` 会因缺 `FB2_AI_CENTER_TOKEN` 返回 `failed=1`，说明最终验收不会跳过权限负向检查。
- 已新增 `docs/fb2-ai-center/voice-device-evidence.example.json`，要求 fb2 真机验证 `VoiceComposerView`、按住说话、上滑取消、三段底部操作区、系统 ASR、云端 ASR 兜底、TTS 和 ASR/TTS 免费策略。
- `scripts/smoke-fb2-visible-chat.ps1` 已作为有副作用真实群聊 smoke，只有传 `-AllowVisibleMessages` 后才会发送 `@EL` 和 selected-message `ai-reply`。
- 最近一次 authenticated 无副作用 smoke 通过，主项目线上版本返回 `0.3.592 37625843aa50b433d9469b8a9c175551d061075d`，fb2 live manifest 返回 `tool_count=31`。
- 最近一次 `-FinalAcceptance -Fb2Username 123qwe -Fb2Password 123qwe` 正确失败在缺 `FB2_AI_CENTER_TOKEN` 和缺 `-VoiceDeviceEvidencePath`，说明最终验收不会误报完成。
- 本轮 `-PreflightOnly` 安全验证通过：缺 `FB2_AI_CENTER_TOKEN` 会立即失败；同时传 `-PreflightOnly -AllowVisibleMessages` 会立即失败；传无效 `Fb2AiCenterToken` 时能解析 `123qwe` 为 `6fe5aa17-0403-427a-8e91-7f414beca35d`，但会在写群前因订单上下文预检 401 失败。
- 2026-06-22 11:00 ADB 复测已完成：设备 `e0d909c3`，fb2 `com.duoguan.football` `1.1.48(96)`，小米语音服务 `com.xiaomi.mibrain.speech/.asr.AsrService`。真机确认群聊页已有 `按住 说话`、文本/语音切换、录音发送和上滑取消；释放路径新增 3 秒语音消息后回到 idle，没有卡在“识别中...”。
- 同轮 logcat 证明 `com.duoguan.football` 触发 `MediaRecorder/AudioRecord`，小米 `AsrService` 返回 `error code: 7 / empty_asr` 后 `ASR_END`，UI 正常回收；但日志没有观察到主项目 `/api/voice/asr` 云端兜底请求，因此这仍是半成品语音证据，不能用于最终 `finalAcceptanceReady=true`。
- 本轮补齐主项目动态发现验收：默认 `scripts/smoke-fb2-ai-center.ps1` 会直接读取 fb2 `/api/main-project/integration`，确认 `routing_mode=main_project_ready`、`service_token_header=X-FB2-AI-CENTER-TOKEN`、`official` 群映射和 Context Pack/readiness/tool manifest/订单/平台摘要/质量/权限端点存在；无 service token 时还会确认 `/context/readiness` 和 `/context/tool-manifest` 返回 401，证明受保护 discovery 没有裸露。
- `scripts/smoke-fb2-final-acceptance.ps1` 的 `preflight_evidence` / `final_acceptance_evidence` 已新增 dynamic discovery 摘录字段，最终 summary 会记录 integration、受保护 discovery、以及有 token 时的 authenticated readiness/manifest 证据。
- 2026-06-22 本轮继续收紧用户订单权限负向验收：`scripts/smoke-fb2-ai-center.ps1 -CheckPermissionBoundaries` 现在不仅验证缺 `X-FB2-AI-CONTEXT-USER-ID` 会 403，还会用同一 `external_user_id` 搭配一个不同的 `X-FB2-AI-CONTEXT-USER-ID` 验证 Context Pack 必须返回 403；permission summary 门槛同步提高到 `total_blocks>=4`、用户范围拦截 `missing_external_user_id_count>=3`、平台范围拦截 `platform_scope_count>=1`，避免“只能看自己的订单”只测缺头不测错头。
- 2026-06-22 本轮修正 live manifest 漂移误判：fb2 线上 manifest 当前提供 `context_feedback_summary`、`context_audit_summary` 和受保护 `/context/quality-summary`、`/context/permission-summary` 集成端点，但不再把 `context_quality_summary` / `context_permission_summary` 暴露为 tool id；主项目 smoke 已改为分别校验 manifest 工具能力和 `/integration` 端点能力，避免把“端点存在但不是聊天工具”误判为 manifest 缺失。

## 未完成

- 未拿到 `FB2_AI_CENTER_TOKEN`，因此不能完成 fb2 live Context Pack、我的票、平台匿名摘要、质量汇总和反馈样本的最终验收。
- `123qwe` 登录能桥接主项目，并且最终验收 wrapper 现在可从 `-Fb2Username/-Fb2Password` 自动解析 `ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`；authenticated `chat-bootstrap` 已验证通过。仍需用真实 `FB2_AI_CENTER_TOKEN` 确认该账号确实有可分析订单样本。
- 已有 ADB 半成品真机语音证据 JSON，且其 artifact 文件真实存在；但它明确 `finalAcceptanceReady=false`。仍缺包含人工语音样本、system ASR final、云端 ASR 成功、server ASR 失败恢复、TTS 播放、ASR/TTS 免费策略的完整 final-ready 证据。
- 真实群聊 `@EL`、长按 `AI回复` 和总结帖入口都已单独抽样通过；仍需把可见群聊、summary post、三类 `feedback_coverage`、质量汇总、权限审计和完整语音证据放进 `scripts/smoke-fb2-final-acceptance.ps1` 同一批 summary 中，才能宣布终极完成。
- 多账号权限验收未完全完成：需要证明用户不能读取他人订单，平台摘要不泄露单个用户，未授权请求会被拒绝并审计。
- 固定质量评测集仍需继续积累 feedback 样本，观察 `missing_context`、`wrong_context`、`citation_unmatched` 和大 Context Pack 比率。
- 动态发现默认检查已补齐；但当前环境仍没有 `FB2_AI_CENTER_TOKEN`，所以 authenticated `/context/readiness` 和 `/context/tool-manifest` 的内容级检查还没有在本轮 live 跑通，最终验收时必须带 token 复核。

## 验证结果

已通过：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -SelfTest
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -SelfTest
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password 123qwe -SkipVoiceContractChecks
cargo test --manifest-path server\Cargo.toml external_app_context_projection -- --nocapture
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password 123qwe
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <redacted>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage -Fb2Username 123qwe -Fb2Password <redacted> -PollTimeoutSec 120
cargo test --manifest-path server\Cargo.toml summary_policy_shape --bin elon-server
cargo test --manifest-path server\Cargo.toml social_ai_agents --bin elon-server
cargo test --manifest-path server\Cargo.toml external_app_context_budget --bin elon-server
cargo test --manifest-path server\Cargo.toml external_app_context_tool_prompt --bin elon-server
scripts\publish-server.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence-20260622-adb.json
pwsh -NoProfile -Command '$files = @("scripts\smoke-fb2-visible-chat.ps1", "scripts\smoke-fb2-final-acceptance.ps1"); foreach ($f in $files) { $parseErrors = $null; $tokens = $null; [System.Management.Automation.Language.Parser]::ParseFile($f, [ref]$tokens, [ref]$parseErrors) | Out-Null; if ($parseErrors.Count -gt 0) { exit 1 } }'
git diff --check
```

预期失败：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -CheckPermissionBoundaries -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -AllowVisibleMessages
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -Fb2AiCenterToken invalid-test-token -Fb2Username 123qwe -Fb2Password 123qwe -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence.example.json
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages -Fb2AiCenterToken invalid-test-token -Fb2Username 123qwe -Fb2Password 123qwe -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence.example.json
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password 123qwe
```

失败原因：

- 缺显式 `-AllowVisibleMessages`，因此 wrapper 不会发送真实群聊消息。
- `-PreflightOnly` 和 `-AllowVisibleMessages` 同时传会失败，避免模式歧义。
- 缺 `FB2_AI_CENTER_TOKEN` 时在写群前失败。
- 缺 `FB2_AI_CENTER_TOKEN` 时，显式权限负向检查也会失败，避免权限验收被 skip。
- 使用无效 service token 时，wrapper 能解析 `123qwe` 的 fb2 用户 UUID，但订单上下文预检在写群前 401 失败。
- `-DataOnlyAcceptance` 缺 `FB2_AI_CENTER_TOKEN` 时也会立即失败，但不会因为缺 `VoiceDeviceEvidencePath` 提前失败。

## 下一步最小动作

1. 让 fb2 会话提供 `FB2_AI_CENTER_TOKEN` 或等价服务 token，用于主项目最终验收拉取 live Context Pack、平台匿名摘要和质量反馈。
2. 确认 `123qwe` 或另一个 fb2 测试账号确实有可分析订单；如果不能用用户名密码解析，再手工提供有订单的测试用户 UUID。
3. 当前 ASR/TTS 暂缓，先用 `-DataOnlyAcceptance` 跑非语音预检；拿到明确可见群授权后再跑 `-DataOnlyAcceptance -AllowVisibleMessages`，把 `@EL`、长按 `AI回复`、总结帖、feedback coverage、质量和权限证据绑到同一份 summary。
4. 后续恢复语音工作时，让 fb2 会话按 `docs/fb2-ai-center/voice-device-evidence.example.json` 回传 `finalAcceptanceReady=true` 的完整真机证据；半成品 ADB 静音证据只能用于定位，不能用于最终验收。
5. 用线上真实 token 跑一次群聊 AI 或 data-only/final preflight，确认 `preflight_readiness.status`、`context_fact_summary.preflight_readiness` 和 `<tool_gap_summary>` 能随 Context Pack / 工具结果进入真实 prompt；fb2 返回 `blocked` 的测试场景下工具执行应记录 skipped，而不是继续调用 `/tools/execute`。
6. 恢复语音后再跑完整最终验收：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN> -VoiceDeviceEvidencePath <real-device-evidence.json>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN> -VoiceDeviceEvidencePath <real-device-evidence.json>
```

## 风险

- 现在“代码/契约/脚本”明显领先于最终证据，不能把脚本可运行等同于用户端真实完成。
- 真实群聊 smoke 有副作用，会产生可见消息；必须保持显式授权开关。
- fb2 live 数据和权限边界是最终产品的核心，不能用示例 JSON 或无 token smoke 替代。
- 后端运行代码没有在本轮修改；如果后续改 server 代码，必须 commit + push 后按发布脚本部署并 live 验证。
