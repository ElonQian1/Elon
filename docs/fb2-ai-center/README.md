# fb2 AI Center 工作台

这个目录是主项目和 fb2 子项目长期协作的统一入口。目标不是一次性把所有能力写完，而是把聊天、语音、AI 回复、业务数据上下文、评测和发布交接固定成可持续演进的工作流。

## 当前结论

- 主项目是 AI Center，负责账号互通、默认群聊、聊天协议、语音 SDK、AI 生成、计费和上下文注入。
- fb2 是业务数据提供方，负责比赛、赔率、订单、群友观点、平台汇总和审计指标。
- 第一阶段不做 MCP。先用 HTTP Context Pack，把 fb2 业务上下文转成模型友好的 Markdown/XML，再由主项目注入群聊 AI。
- ASR、TTS、Context Pack 拉取免费；只有 AI 生成回复内容消耗 token/额度。
- fb2 不应该复制主项目内部聊天页代码。Android 原生侧优先接 `android/chat-voice-kit`，H5/WebView 侧按 `ChatVoiceInteractionContract` 还原。

## 已有主项目能力

- 外部应用注册：`server/src/external_app_registry.rs`
- 外部账号、授权、默认群和试用额度：`server/src/external_app_api.rs`、`server/src/store/external_apps.rs`
- fb2 聊天/语音启动协议：`GET /api/external/apps/fb2/chat-bootstrap`
- fb2 上下文契约：`GET /api/external/apps/fb2/context-contract`
- fb2 业务上下文拉取：`server/src/external_app_context.rs`
- Context Pack 预算和 prompt 投影：`server/src/external_app_context_budget.rs`
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

- `contracts.md`：主项目和 fb2 之间的 HTTP、上下文、工具和 SDK 契约。
- `roadmap.md`：P0 到 P3 的执行顺序和验收目标。
- `data-tools.md`：fb2 应该提供哪些业务数据能力，以及从 Context Pack 走向 MCP/tools 的路径。
- `voice-sdk.md`：fb2 复用主项目 ASR/TTS 和微信式语音输入栏的落地标准。
- `billing-policy.md`：免费通道和扣费通道的固定口径。
- `test-plan.md`：端到端验收和长期评测清单。
- `final-acceptance-matrix.md`：终极目标逐项验收矩阵，明确每个接口、场景、权限和质量项需要什么证据。
- `handoff.md`：7*24 协作交接记录模板和当前状态。

常规巡检先跑无副作用脚本 `scripts/smoke-fb2-ai-center.ps1`。只有拿到明确授权后，才运行有副作用脚本 `scripts/smoke-fb2-visible-chat.ps1 -AllowVisibleMessages`，它会向真实群聊发送可见消息。

最终验收使用 `scripts/smoke-fb2-final-acceptance.ps1`。先用 `-PreflightOnly` 做无副作用预检：解析 `ExternalUserId`、确认该用户有订单上下文，并在不发送群消息的前提下强制验证 fb2 live 数据、六类标准场景、平台匿名摘要、fb2 APK 发布、主项目语音 SDK 构建、真机语音证据和 no-skip 门槛。预检通过后，再用 `-AllowVisibleMessages` 把真实群聊可见触发和 `smoke-fb2-ai-center.ps1 -FinalAcceptance` 绑定到同一批证据，并输出机器可读 summary；summary 会记录子脚本日志路径、`@EL` 消息 ID、AI 回复 ID、长按 `AI回复` 消息 ID 和 feedback evidence。传 `-Fb2Username/-Fb2Password` 时会自动解析 `ExternalUserId`；缺 `FB2_AI_CENTER_TOKEN`、无法解析或手工提供有订单的 `ExternalUserId`、真机语音证据或显式写群授权时必须失败。
