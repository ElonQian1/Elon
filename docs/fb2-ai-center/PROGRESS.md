# fb2 AI Center 当前进度

## 当前状态

- 工作目录：`D:\rust\active-projects\elon-main-fb2-docs-20260621`
- 分支：`main`
- 远端：`origin/main`
- 最新已同步提交：`e3761564 test(fb2): 增加语音真机证据验收`
- 任务性质：主项目侧 AI Center、聊天/语音 SDK、上下文注入、验收脚本和文档交接。

## 已完成

- 主项目已建立 `docs/fb2-ai-center/` 工作台，固定主项目与 fb2 的分工：主项目提供 AI Center 和聊天/语音能力，fb2 提供业务数据。
- 主项目已提供 fb2 `chat-bootstrap` 和 `context-contract` 验收项，覆盖聊天、ASR、TTS、AI 回复、计费和 answer policy。
- 主项目 smoke 脚本已能验证 live fb2 tool manifest、工具执行策略、六类固定评测场景和主项目聊天自动工具覆盖。
- `scripts/smoke-fb2-ai-center.ps1` 已支持用 `123qwe/123qwe` 这类 fb2 用户账号桥接主项目登录，验证 authenticated `chat-bootstrap`，不会发送群消息。
- 脚本已支持 `-CheckFb2ApkVersion`，验证 fb2 `/api/app-version`、APK 版本、`update_kind=full_apk`、checksum、size 和 APK 下载 HEAD。
- 脚本已支持 `-CheckLocalVoiceSdkBuild`，验证主项目 `android/chat-voice-kit` 可执行 `:chat-voice-kit:assembleDebug`。
- 脚本已支持 `-RequireNoSkips`，防止缺 token 或缺覆盖项时把 skip 当成完成。
- 脚本已支持 `-FinalAcceptance`，自动打开 live 数据、完整场景、平台摘要、质量反馈、APK、语音 SDK 构建、真机语音证据和 no-skip 门槛。
- 已新增 `scripts/smoke-fb2-final-acceptance.ps1`，支持 `-PreflightOnly` 无副作用预检，也会在写群前解析 `ExternalUserId` 并预检用户订单上下文，再把真实群聊可见触发和 `-FinalAcceptance` 绑定为同一批 `QualitySince` 证据，并输出机器可读 summary、子脚本日志路径、可见消息 ID、AI 回复 ID 和 feedback evidence。
- 已新增 `docs/fb2-ai-center/voice-device-evidence.example.json`，要求 fb2 真机验证 `VoiceComposerView`、按住说话、上滑取消、三段底部操作区、系统 ASR、云端 ASR 兜底、TTS 和 ASR/TTS 免费策略。
- `scripts/smoke-fb2-visible-chat.ps1` 已作为有副作用真实群聊 smoke，只有传 `-AllowVisibleMessages` 后才会发送 `@EL` 和 selected-message `ai-reply`。
- 最近一次无副作用 smoke 通过，主项目线上版本返回 `0.3.579 8106b0cca6bbe95370625def93f32a2716fb56ca`。
- 最近一次 `-FinalAcceptance -Fb2Username 123qwe -Fb2Password 123qwe` 正确失败在缺 `FB2_AI_CENTER_TOKEN` 和缺 `-VoiceDeviceEvidencePath`，说明最终验收不会误报完成。

## 未完成

- 未拿到 `FB2_AI_CENTER_TOKEN`，因此不能完成 fb2 live Context Pack、我的票、平台匿名摘要、质量汇总和反馈样本的最终验收。
- `123qwe` 登录能桥接主项目，并且最终验收 wrapper 现在可从 `-Fb2Username/-Fb2Password` 自动解析 `ExternalUserId=6fe5aa17-0403-427a-8e91-7f414beca35d`；仍需用真实 `FB2_AI_CENTER_TOKEN` 确认该账号确实有可分析订单样本。
- 未拿到真机语音证据 JSON；示例文件只能验证脚本分支，不能证明真实 APK 在小米/HyperOS 上不会卡住“识别中”。
- 真实群聊可见入口还需要继续抽样 `@EL`、长按 `AI回复` 和总结帖入口，确认 AI 回答持续区分比赛事实、本人订单、平台汇总、群友观点和 AI 推断。
- 多账号权限验收未完全完成：需要证明用户不能读取他人订单，平台摘要不泄露单个用户，未授权请求会被拒绝并审计。
- 固定质量评测集仍需继续积累 feedback 样本，观察 `missing_context`、`wrong_context`、`citation_unmatched` 和大 Context Pack 比率。

## 验证结果

已通过：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1
pwsh -NoProfile -Command '$parseErrors = $null; $tokens = $null; [System.Management.Automation.Language.Parser]::ParseFile("scripts\smoke-fb2-final-acceptance.ps1", [ref]$tokens, [ref]$parseErrors) | Out-Null; if ($parseErrors.Count -gt 0) { exit 1 }'
git diff --check
```

预期失败：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -PreflightOnly -AllowVisibleMessages
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -AllowVisibleMessages -Fb2AiCenterToken invalid-test-token -Fb2Username 123qwe -Fb2Password 123qwe -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence.example.json
```

失败原因：

- 缺显式 `-AllowVisibleMessages`，因此 wrapper 不会发送真实群聊消息。
- `-PreflightOnly` 和 `-AllowVisibleMessages` 同时传会失败，避免模式歧义。
- 缺 `FB2_AI_CENTER_TOKEN` 时在写群前失败。
- 使用无效 service token 时，wrapper 能解析 `123qwe` 的 fb2 用户 UUID，但订单上下文预检在写群前 401 失败。

## 下一步最小动作

1. 让 fb2 会话提供 `FB2_AI_CENTER_TOKEN` 或等价服务 token，用于主项目最终验收拉取 live Context Pack 和质量反馈。
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
