# fb2 AI Center 完成计划

## 目标

让 fb2 用户在主项目群聊和聊天入口中使用主项目 AI，基于 fb2 的实时比赛、赔率、本人订单、平台匿名订单汇总和群友观点回答问题。AI 回答必须有来源引用、权限边界、计费边界、反馈记录和可重复验收。

## 架构路线

1. 主项目作为 AI Center：负责账号互通、聊天/语音 SDK、AI 回复、工具规划、计费、上下文注入、观测和评测。
2. fb2 作为数据源：负责比赛、赔率、订单、群观点、平台聚合、反馈质量和工具 manifest。
3. 第一阶段不做完整 MCP/RAG：优先使用 REST Context Pack + tool manifest + 轻量工具执行闭环。
4. AI 输入格式采用 XML-wrapped Markdown Context Pack，结构化元数据使用 JSON，小块业务数据保留 source id，方便回答引用和审计。
5. ASR、TTS、Context Pack 拉取免费；只有 AI 生成回复内容扣 token/额度。

## 里程碑

1. 契约稳定：`chat-bootstrap`、`context-contract`、tool manifest、answer policy、billing policy 均可机器验收。
2. 数据闭环：主项目能拉取 fb2 Context Pack，并覆盖今日比赛、我的票、平台匿名订单摘要、群友观点、长按消息复核和来源审计场景。
3. 聊天闭环：真实群聊 `@EL`、长按 `AI回复`、总结帖入口都能触发主项目 AI，并把 fb2 业务上下文注入 prompt。
4. 语音闭环：fb2 APK 使用 `android/chat-voice-kit` 的 `VoiceComposerView`，具备按住说话、上滑取消、AI回复/转文字/发送、系统 ASR 优先、云端 ASR 兜底和 TTS。
5. 权限闭环：用户只能看自己的订单；平台摘要只返回匿名聚合；未授权请求拒绝并记录审计。
6. 质量闭环：AI 回答记录引用来源，fb2 回传 feedback，可查询质量汇总和失败样本。
7. 发布闭环：主项目服务、fb2 后端、fb2 APK/前端均发布并通过 live 验证。
8. 交接闭环：`docs/fb2-ai-center/`、fb2 对应文档和 handoff 保持同步，后续会话按同一验收表继续。

## 当前重点

- 2026-06-22 后续收口已把“直接读取群聊”从文档要求升级为最终 wrapper 的机器门槛：主项目提交 `604b2b88f7ebe9a23dd5855c67f58039acc8ba0c` 已推送到 `origin/main`，`scripts\smoke-fb2-final-acceptance.ps1` 的 summary 新增 `visible_direct_read_complete`。最终 `success=true` 现在必须同时满足 baseline、`@EL` seed/回复、selected-message seed/`AI回复`、summary post 全部由群聊/summary-post 接口回读，且每项带 `text_len` 和 `text_sha256`；只有消息 ID、截图或人工日志描述不能通过。
- 本轮进一步把“可用但降级”从“完全完成”里拆出来：`-FinalAcceptance` 要求 fb2 authenticated readiness 为 `ready`，summary post 必须是模型生成 `ready`；`-DataOnlyAcceptance` 暂时允许 readiness `partial` 和 summary `ready_with_fallback`，但 summary JSON 必须显式记录 `fb2_authenticated_readiness_acceptable`、`summary_post_fallback_used` 和 `summary_post_ready_for_mode`。`degraded/blocked/unavailable` readiness 不能通过 data-only 或 full final。
- 截至 2026-06-22 21:34，最新非语音 data-only visible acceptance 已通过，且证据来自真实群聊接口直读，不是截图：`target\fb2-ai-center\data-only-acceptance-20260622T133357Z.json` 返回 `success=true`、`visible_chat_exit_code=0`、`final_acceptance_exit_code=0`、`voice_status=deferred_by_user`。`@EL` seed `gmsg_dac99a2fa97843f199cb55a154129468` 触发 AI 回复 `gai_55052a82215943339fb463bd2e362c36`，selected-message seed `gmsg_842bde06e5ce40d6b89a70ed5adfe96e` 触发 `AI回复` `gai_95f2186189814504b7fb3852d97fc778`，summary post `gsp_a15658c1aa1b4f51bc8f47c78a5e91f7 status=ready_with_fallback` 由 summary-post 接口回读且正文策略通过。三类 feedback 覆盖 `3/3`，`quality_unmatched_cited_sources=0`，非合成 `feedback=3`、`opinion_adoption=1`、`memory_refs=1`。主项目已发布 `v0.3.640 / cdfdff5e61f5a5455e5d0fc32997e234fd13ceb2`，本阶段剩余不是比赛/订单/群观点数据闭环，而是用户要求暂缓的 ASR/TTS final-ready 真机语音证据。
- 群聊对话验收不以截图为证据。每次 visible smoke 或最终验收都必须通过群聊接口直接读取 baseline messages、`@EL` seed/AI 回复、selected-message seed/`AI回复`、summary post、feedback 和 quality/adoption 结果，并保存对应 ID、count、正文 `text_len`、正文 `text_sha256`、matched/unmatched 统计；最终 wrapper 还必须输出 `visible_direct_read_complete=true`。截图只用于人工 UI 观感，不证明链路打通。
- full final acceptance 默认仍要求非合成观点采纳 `MinOpinionAdoptionCount=1`；data-only 可见窗口默认也保持该门槛。只有显式传 `-AllowNoNewOpinionAdoptionInShortWindow` 才允许短时间真实群 smoke 不新增 adoption，该 opt-out 不能作为 full final 的观点采纳证据。
- 无 token 的 smoke 只能证明主项目契约和 live manifest 可读，不能证明最终完成。
- `-FinalAcceptance` 必须同时具备主项目登录、`FB2_AI_CENTER_TOKEN`、fb2 live 数据、质量反馈样本、fb2 APK 发布检查、主项目语音 SDK 构建和 fb2 真机语音证据；只有 `-SelfTest` 例外，它只验证 wrapper 本地解析逻辑。
- 当前 ASR/TTS 暂缓期间，非语音闭环使用 `-DataOnlyAcceptance`，它不要求主项目语音 SDK 构建或真机语音证据，只验证比赛/订单/平台摘要/群观点、权限、质量、feedback 和真实群聊可见入口。该模式不能替代 `-FinalAcceptance`，也不能宣布终极目标完成。
- 当前主项目契约继续向“fb2 业务数据服务 AI”收口：`source_registry.required_kinds` 只代表比赛、赔率、本人订单、票据、群消息、观点记忆、平台匿名摘要和上下文审计等业务事实；`feedback/opinion_adoption` 是 `quality_history_kinds`。工具执行结果必须走 `tool_result_envelope_contract`，由 `source_ids + visibility + grounding` 决定是否可作为事实。
- `scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly` 是进入真实群聊前的无副作用强门禁，必须先验证 fb2 live 数据、六类标准场景、平台匿名摘要、权限负向审计、fb2 APK 发布、主项目语音 SDK 构建、真机语音证据和 no-skip。
- `final-acceptance-matrix.md` 是终极目标完成审计入口；任何会话要宣布完成前，必须逐项对照矩阵拿到当前证据。
- 真机语音证据必须使用 `docs/fb2-ai-center/voice-device-evidence.example.json` 同格式回传，不能只用口头描述。
- 最终总验收使用 `scripts\smoke-fb2-final-acceptance.ps1`，把真实群聊可见触发和 `-FinalAcceptance` 绑定成同一批证据，避免拿历史反馈或旧群聊记录凑数。
- `scripts\smoke-fb2-final-acceptance.ps1 -SelfTest` 现在同时覆盖 full final 和 data-only 两种摘要语义：data-only 必须写 `voice_status=deferred_by_user`，不能误要求本地语音 SDK 或 final-ready 语音证据，但仍必须保留用户订单、权限审计、质量 feedback 和群观点采纳 evidence。

## 风险

- 没有 `FB2_AI_CENTER_TOKEN` 时，无法验证 fb2 live Context Pack、质量汇总和反馈样本。
- 没有真机证据时，无法证明小米/HyperOS 等系统 ASR 超时后云端兜底不会卡在“识别中”。
- 真实群聊 smoke 会产生可见消息，只能在用户明确授权后运行。
- 多会话并行时，主项目只改主项目 SDK、服务端、契约和文档，fb2 会话只改 fb2 业务接口和客户端，避免同时修改同一实现。

## 验证命令

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -SelfTest
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -SelfTest
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1
# 预期失败：example 证据是格式模板，artifact ref 是占位，finalAcceptanceReady=false
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence.example.json
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>
# 最新严格直读样本：target\fb2-ai-center\data-only-acceptance-20260622T133357Z.json，直读、feedback、权限、quality 和观点采纳同批通过；ASR/TTS 仍暂停
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -FinalAcceptance -Fb2Username 123qwe -Fb2Password 123qwe -Fb2Token <FB2_AI_CENTER_TOKEN> -ExternalUserId <fb2_user_uuid_with_orders> -VoiceDeviceEvidencePath <real-device-evidence.json>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password 123qwe
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN> -VoiceDeviceEvidencePath <real-device-evidence.json>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN> -VoiceDeviceEvidencePath <real-device-evidence.json>
git diff --check
powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed
```

## 回滚策略

- 脚本/文档改动回滚对应 commit 即可。
- 后端运行代码若发布后异常，按项目发布脚本回滚到上一稳定 SHA。
- 不把 fb2 业务数据复制进主项目；出现数据错配先修 fb2 Context Pack 或 token/base_url，不让主项目编造。
- 如果最终验收 wrapper 的 `feedback_coverage` 判断异常，先跑 `-SelfTest` 复现本地解析问题，再决定是否修改真实群聊 smoke。
- `-SelfTest` 还会检查 voice/quality/permission 的合成 OK 行是否能进入 `final_acceptance_evidence`，如果某个子脚本 check name 改动导致 summary 字段变空，应先同步 wrapper 映射和文档。
