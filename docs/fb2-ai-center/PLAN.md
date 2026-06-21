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

- 无 token 的 smoke 只能证明主项目契约和 live manifest 可读，不能证明最终完成。
- `-FinalAcceptance` 必须同时具备主项目登录、`FB2_AI_CENTER_TOKEN`、fb2 live 数据、质量反馈样本、fb2 APK 发布检查、主项目语音 SDK 构建和 fb2 真机语音证据。
- 真机语音证据必须使用 `docs/fb2-ai-center/voice-device-evidence.example.json` 同格式回传，不能只用口头描述。
- 最终总验收使用 `scripts\smoke-fb2-final-acceptance.ps1`，把真实群聊可见触发和 `-FinalAcceptance` 绑定成同一批证据，避免拿历史反馈或旧群聊记录凑数。

## 风险

- 没有 `FB2_AI_CENTER_TOKEN` 时，无法验证 fb2 live Context Pack、质量汇总和反馈样本。
- 没有真机证据时，无法证明小米/HyperOS 等系统 ASR 超时后云端兜底不会卡在“识别中”。
- 真实群聊 smoke 会产生可见消息，只能在用户明确授权后运行。
- 多会话并行时，主项目只改主项目 SDK、服务端、契约和文档，fb2 会话只改 fb2 业务接口和客户端，避免同时修改同一实现。

## 验证命令

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-ai-center.ps1 -RequireVoiceDeviceEvidence -VoiceDeviceEvidencePath docs\fb2-ai-center\voice-device-evidence.example.json
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
