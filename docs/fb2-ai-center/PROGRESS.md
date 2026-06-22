# fb2 AI Center 当前进度

## 当前状态

- 工作目录：`D:\rust\active-projects\elon-main-fb2-docs-20260621`
- 分支：`main`
- 远端：`origin/main`
- 当前代码状态以 `git status -sb` 和 `git log -1 --oneline` 为准；每轮收尾必须在最终回复里给出本轮提交 SHA。
- 任务性质：主项目侧 AI Center、聊天/语音 SDK、上下文注入、验收脚本和文档交接。

## 已完成

- 2026-06-22 09:06 补齐“总结帖/群聊总结入口”的主项目侧验收闭环：提交 `1d41cb5a` 增加 fb2 总结帖回答契约和 smoke 场景，要求输出 `数据事实`、`群友观点`、`AI推断`、`风险边界` 并保留 source references；提交 `225bfc6f` 把总结帖生成从单一默认模型改为 `social_ai` 多代理 fallback，避免默认模型额度/接口不可用时只生成兜底文案；提交 `96bf5ce4` 放宽总结帖脚本对“相关发言”原文引用的误判。上述提交均已推送到 `origin/main`，其中服务端运行代码已发布到 `v0.3.588 / 225bfc6f0d9d33552f60dfd96a220753b3f7f7b6`，`96bf5ce4` 为脚本验收修正，无需重新发布服务端。
- 2026-06-22 09:06 总结帖真实群 smoke 已通过：命令 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage -Fb2Username 123qwe -Fb2Password <redacted> -PollTimeoutSec 120` 在真实群 `ext_fb2_official` 创建 summary post `gsp_46720718477f4c6e953b55d5fc309568`，最终 `status=ready`，脚本结果 `failed=0 skipped=2`。该总结帖回复通过非空 summary、source references、事实/观点/推断/风险分层、风险边界和禁止投注保证检查。
- 2026-06-22 09:06 本轮修复过一个线上根因：`gsp_b4c717d3c2d947188ccc755fe4f6ff32` 曾返回 `ready_with_fallback`，错误为“当前 AI 模型额度已用尽或接口不可用”，这不是 fb2 用户余额或 ASR/TTS 计费问题，而是总结帖链路没有使用群聊 AI 的模型 fallback。修复后 `gsp_400c852f06054a9eba16c8b643a3ae73` 和 `gsp_46720718477f4c6e953b55d5fc309568` 均进入 `ready`，模型 fallback 使用 `hunyuan-turbo` 正常生成。
- 2026-06-22 09:06 ADB 真机复核再次确认当前 fb2 APK 已具备主项目式聊天体验：设备 `e0d909c3` 上 fb2 `com.duoguan.football 1.1.48(96)`，`RECORD_AUDIO granted=true`，appops 为 `foreground/allow`；启动 `com.duoguan.football/.MainActivity` 后，`夺冠体育官方群` 页面可见主项目 AI 回复包含 `数据事实`、`AI推断`、`风险边界`、`context_audit_id`、`selected_message_id`，底部输入栏显示 `按住 说话`。截图证据保存在 `target\fb2-current-20260622.png`，UI dump 保存在 `target\fb2-window-20260622.xml`；本轮 logcat 未见 fb2 `AndroidRuntime/FATAL`。
- 2026-06-22 03:04 复核主项目 git/线上/真机链路：主项目修复提交 `589d2bacf51cf4c679505da52d8ecfea1762420b`（`修复fb2群聊AI回答缺少分层边界`）已推入 `origin/main`，并包含在当前线上最新 `v0.3.585 / 4b0fb9dd363e3619faab7bf73c3ded680e1ad40e` 中。该修复在 fb2 外部上下文下对群聊 `@EL` 和长按 `AI回复` 回复做后处理兜底：如果模型漏掉短标签，会补齐 `数据事实：`、`AI推断：` 和 `风险边界：`，风险边界明确“不保证命中、不建议重注或梭哈”。本轮验证命令包括 `cargo test social_ai --bin elon-server`、`cargo test social_ai_message_reply --bin elon-server`、`cargo test external_app_context_answer_policy --bin elon-server`、pre-push `cargo check --workspace`、`publish-server.ps1`、`smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <redacted>` 和 `smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password <redacted>`。
- 2026-06-22 03:04 真实群聊可见 smoke 已通过，不再是“未验证”：账号 `123qwe` 通过 fb2 session bridge 解析为 `ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`，目标群 `ext_fb2_official`。`@EL` 可见消息 `gmsg_b2d834caf30c4265acd638cb3868bf21` 触发 AI 回复 `gai_4df8a06989b149ecadf780abc1b0914d`；selected-message seed `gmsg_a71960917eeb494f8993c4e43adb927d` 触发 AI 回复 `gai_37f12f3fc7da4598a44f1b622955709d`。脚本结果 `failed=0 skipped=0`，两条回复均通过来源标记、事实/推断分层、风险边界、禁止投注保证和反驳“肯定赢盘/重注”检查。
- 2026-06-22 03:04 ADB 真机复核已把真实群聊 AI 结果带到用户端：设备 `Xiaomi 23116PN5BC` 上安装 fb2 `com.duoguan.football 1.1.48(96)`，`RECORD_AUDIO granted=true`，启动 `com.duoguan.football/.MainActivity` 无 `AndroidRuntime/FATAL`。进入“聊天 -> 🏆 夺冠体育官方群”后，聊天列表摘要显示 `数据事实：...`，群聊详情页可见 selected-message AI 回复正文包含 `数据事实`、`AI推断`、`风险边界`、`来源`、`context_audit_id` 和 `selected_message_id`；底部输入栏显示主项目式 `按住 说话`。本轮 logcat 只见 Google Play/系统网络超时和卫星电话能力探测噪声，未见 fb2 崩溃。
- 2026-06-22 主项目回答策略继续收紧：`fb2.answer_policy.v1` 的 `prompt_answer_rules` 现在要求使用 `数据事实：`、`用户订单：`、`平台汇总：`、`群友观点：`、`AI推断：`、`风险边界：` 等短标签；凡涉及比赛、赔率、票据、推荐、预测或今日比赛讨论，至少要输出 `数据事实`、`AI推断` 和 `风险边界`，并明确赛果不确定、不保证命中、不建议重注或梭哈。群聊基础 prompt 和长按 `AI回复` prompt 同步采用该口径；可见群聊 smoke 也补了 negation-aware 的投注保证判定，避免把“不要重注/不宜稳赢”误判为诱导。已验证 `cargo fmt --check`、PowerShell 解析、`cargo test external_app_context_answer_policy --bin elon-server`、`cargo test social_ai --bin elon-server`、`cargo test social_ai_message_reply --bin elon-server`、`smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password 123qwe`。
- 2026-06-22 ADB 真机阶段验证已完成并记录到 `docs/fb2-ai-center/voice-device-evidence-20260622-adb.json`：设备 `Xiaomi 23116PN5BC / Android 16 / HyperOS OS3.0`，fb2 APK `com.duoguan.football 1.1.48(96)`，系统 ASR `com.xiaomi.mibrain.speech/.asr.AsrService`，录音权限和 appops 正常。实测进入 `夺冠体育官方群` 后可见主项目式 `按住 说话` 输入栏，文本/语音切换可用，按住后出现绿色录音气泡和 `取消 / AI回复 / 转文字 / 发送` 控制区，上滑取消可恢复，静音释放到 `转文字` 后 10 秒内回到 `按住 说话`，未复现永久卡在“识别中”。本轮没有人工语音样本，未证明 system ASR final、云端 ASR 成功、TTS 播放和余额为 0 时 ASR/TTS 免费，因此该 JSON 明确 `finalAcceptanceReady=false`，不能作为最终完成证据。
- 2026-06-22 已用 `smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence-20260622-adb.json` 复核这份半成品证据：脚本正确通过 UI/录音相关项，并在 `tooShort`、`systemAsrSuccess`、`systemAsrTimeoutServerFallback`、`serverAsrSuccess`、`serverAsrFailureRecoversUi`、`ttsPlayback`、`asrTtsFreeWithZeroAiBalance` 7 项上失败，结果为 `failed=7 skipped=2`。这证明最终验收不会被 ADB 静音阶段证据误放行。
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
- 默认 smoke 已新增 live manifest 必需工具检查，覆盖 `context_pack`、`today_matches`、`match_analysis_brief`、`group_opinion_summary`、用户订单、平台摘要、feedback、quality、permission audit 和 `tool_manifest`。
- `scripts/smoke-fb2-ai-center.ps1` 已新增 `-CheckPermissionBoundaries`，用于验证缺当前用户头、缺 platform scope、用户订单工具缺当前用户头都会 403，并读取 fb2 `/context/permission-summary` 证明被审计；`-FinalAcceptance` 和 `smoke-fb2-final-acceptance.ps1 -PreflightOnly` 会自动开启该门槛。
- `scripts/smoke-fb2-final-acceptance.ps1` 的 summary 已新增 `preflight_evidence` / `final_acceptance_evidence`，直接摘录 APK、语音、场景、权限和质量关键 OK 行，减少最终验收时人工翻日志的空间。
- 本轮 summary 证据补强验证通过：`smoke-fb2-final-acceptance.ps1` 解析通过；默认无副作用 smoke 仍为 `failed=0 skipped=2`；缺 `FB2_AI_CENTER_TOKEN` 时 `-PreflightOnly` 仍立即失败，不会写真实群。
- 本轮无 token 验证通过：默认 smoke 仍为 `failed=0 skipped=2`；显式 `-CheckPermissionBoundaries -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d` 会因缺 `FB2_AI_CENTER_TOKEN` 返回 `failed=1`，说明最终验收不会跳过权限负向检查。
- 已新增 `docs/fb2-ai-center/voice-device-evidence.example.json`，要求 fb2 真机验证 `VoiceComposerView`、按住说话、上滑取消、三段底部操作区、系统 ASR、云端 ASR 兜底、TTS 和 ASR/TTS 免费策略。
- `scripts/smoke-fb2-visible-chat.ps1` 已作为有副作用真实群聊 smoke，只有传 `-AllowVisibleMessages` 后才会发送 `@EL` 和 selected-message `ai-reply`。
- 最近一次无副作用 smoke 通过，主项目线上版本返回 `0.3.579 8106b0cca6bbe95370625def93f32a2716fb56ca`，fb2 live manifest 返回 `tool_count=30`。
- 最近一次 `-FinalAcceptance -Fb2Username 123qwe -Fb2Password 123qwe` 正确失败在缺 `FB2_AI_CENTER_TOKEN` 和缺 `-VoiceDeviceEvidencePath`，说明最终验收不会误报完成。
- 本轮 `-PreflightOnly` 安全验证通过：缺 `FB2_AI_CENTER_TOKEN` 会立即失败；同时传 `-PreflightOnly -AllowVisibleMessages` 会立即失败；传无效 `Fb2AiCenterToken` 时能解析 `123qwe` 为 `6fe5aa17-0403-427a-8e91-7f414beca35d`，但会在写群前因订单上下文预检 401 失败。

## 未完成

- 未拿到 `FB2_AI_CENTER_TOKEN`，因此不能完成 fb2 live Context Pack、我的票、平台匿名摘要、质量汇总和反馈样本的最终验收。
- `123qwe` 登录能桥接主项目，并且最终验收 wrapper 现在可从 `-Fb2Username/-Fb2Password` 自动解析 `ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`；authenticated `chat-bootstrap` 已验证通过。仍需用真实 `FB2_AI_CENTER_TOKEN` 确认该账号确实有可分析订单样本。
- 未拿到真机语音证据 JSON；示例文件只能验证脚本分支，不能证明真实 APK 在小米/HyperOS 上不会卡住“识别中”。
- 真实群聊 `@EL`、长按 `AI回复` 和总结帖入口都已单独抽样通过；仍需把可见群聊、summary post、feedback、质量汇总、权限审计和完整语音证据放进 `scripts/smoke-fb2-final-acceptance.ps1` 同一批 summary 中，才能宣布终极完成。
- 多账号权限验收未完全完成：需要证明用户不能读取他人订单，平台摘要不泄露单个用户，未授权请求会被拒绝并审计。
- 固定质量评测集仍需继续积累 feedback 样本，观察 `missing_context`、`wrong_context`、`citation_unmatched` 和大 Context Pack 比率。

## 验证结果

已通过：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -Fb2Username 123qwe -Fb2Password 123qwe
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <redacted>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages -SkipMention -SkipSelectedMessage -Fb2Username 123qwe -Fb2Password <redacted> -PollTimeoutSec 120
cargo test --manifest-path server\Cargo.toml summary_policy_shape --bin elon-server
cargo test --manifest-path server\Cargo.toml social_ai_agents --bin elon-server
scripts\publish-server.ps1
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
```

失败原因：

- 缺显式 `-AllowVisibleMessages`，因此 wrapper 不会发送真实群聊消息。
- `-PreflightOnly` 和 `-AllowVisibleMessages` 同时传会失败，避免模式歧义。
- 缺 `FB2_AI_CENTER_TOKEN` 时在写群前失败。
- 缺 `FB2_AI_CENTER_TOKEN` 时，显式权限负向检查也会失败，避免权限验收被 skip。
- 使用无效 service token 时，wrapper 能解析 `123qwe` 的 fb2 用户 UUID，但订单上下文预检在写群前 401 失败。

## 下一步最小动作

1. 让 fb2 会话提供 `FB2_AI_CENTER_TOKEN` 或等价服务 token，用于主项目最终验收拉取 live Context Pack、平台匿名摘要和质量反馈。
2. 确认 `123qwe` 或另一个 fb2 测试账号确实有可分析订单；如果不能用用户名密码解析，再手工提供有订单的测试用户 UUID。
3. 让 fb2 会话按 `docs/fb2-ai-center/voice-device-evidence.example.json` 回传真机证据。
4. 跑完整最终验收：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN> -VoiceDeviceEvidencePath <real-device-evidence.json>
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password 123qwe -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN> -VoiceDeviceEvidencePath <real-device-evidence.json>
```

## 风险

- 现在“代码/契约/脚本”明显领先于最终证据，不能把脚本可运行等同于用户端真实完成。
- 真实群聊 smoke 有副作用，会产生可见消息；必须保持显式授权开关。
- fb2 live 数据和权限边界是最终产品的核心，不能用示例 JSON 或无 token smoke 替代。
- 后端运行代码没有在本轮修改；如果后续改 server 代码，必须 commit + push 后按发布脚本部署并 live 验证。
