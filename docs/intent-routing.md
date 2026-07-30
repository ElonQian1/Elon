# Intent Routing / 能力路由

## 目标

用户入口会同时来自 Web、APK，未来还可能来自 Windows 客户端。入口可以不同，但服务端需要先判断“本轮选择哪条能力路线”，再选择具体模型或执行管线。

本模块把分流规则集中在 `server/src/intent_router.rs`，避免在 Web、APK、项目会话、CLI fallback 等不同路径里重复写关键词判断。

能力路由不是“普通聊天优先 vs 开发任务”的二分。项目 CLI 透传、项目执行、产品讨论、验收发布、图片/资产旁路、非执行类项目问答等都可以是合法路线。`intent_router` 只在未显式指定路线时提供确定性辅助，不能让低成本分类模型直接获得写代码或发布权限。

当用户或产品功能已经显式选择 Route A / passthrough / plan / Route B/C 等路线时，显式路线优先。不能因为用户消息很短、像问候、像解释性问题，就自动把本轮改判为“普通聊天/轻量聊天”。“轻量问答”只是并行能力之一，不是项目会话的默认产品重心。

## 当前能力矩阵

| 能力路线 | 代码枚举/模式 | 主要执行者 | 适合请求 | 权重说明 |
| --- | --- | --- | --- | --- |
| 项目 CLI 透传 / Route A 直连 | `AiCliRequestMode::Passthrough` + Route A | 用户 PC 上已登录的 Codex / Copilot / Claude CLI | 用户明确选择“直连本机 CLI”“强制 Codex”“按项目上下文完整交给 CLI” | 一等路线；即使输入很短也不降级为轻量问答 |
| 项目执行 / 开发 | `CapabilityRoute::CodeAgent` | Codex CLI only（当前测试期强制） | App、Web、服务端、APK、部署、修复、重构 | 一等路线；需要项目队列、worktree、命令、文件和验证过程 |
| 非执行类项目问答（可选） | `CapabilityRoute::ChatAgent` | Codex CLI only（当前测试期强制） | 不需要项目执行的说明、配置、能力讨论、模型选择说明 | 并行路线之一；不能作为项目会话的默认重心 |
| 图片处理（测试期） | `CapabilityRoute::CodeAgent` | Codex CLI only | “画一张图”“生成头像/海报/壁纸”“生成 App 图标并替换” | 当前测试期仍交给 Codex CLI；后续可恢复图片旁路 |

## 意图类型

`UserIntent` 是用户语义，`CapabilityRoute` / Route A/B/C / passthrough / plan 等是执行路线。语义分类不能覆盖用户或产品功能已经显式选择的路线。

| 意图 | 执行路线 | 是否需要图片 | 是否需要改代码 |
| --- | --- | --- | --- |
| `NormalChat` | `ChatAgent` | 否 | 否 |
| `ModelConfig` | `ChatAgent` | 否 | 否 |
| `AppDevelopment` | `CodeAgent` | 否 | 是 |
| `TextToImage` | `CodeAgent` | 否 | 是 |
| `ImageAssetForApp` | `CodeAgent` | 否 | 是 |
| `Unknown` | `ChatAgent` | 否 | 否 |

## 运行顺序

1. 先读取产品功能、用户设置和 API 参数里是否已经显式指定 Route A/B/C、passthrough、plan、图片、项目执行等路线。
2. 如果路线已经显式指定，按该路线执行；`intent_router` 只能做补充解释和安全校验，不能把本轮改判到另一条路线。
3. 未显式指定路线时，`agent::run_dispatch_with_workspace` 再调用 `intent_router::classify(user_message)` 选择默认路线。
4. 当前默认启用 `AI_CODEX_CLI_ONLY=true`，不论路线结果是非执行类项目问答、模型配置、图片处理还是项目开发，默认执行后端都会被锁定为 Codex CLI；用户显式保存自带 API Key 的 BYOK 配置时，可作为例外走 API Agent。
5. Codex CLI only 只表示“主执行者是 Codex CLI”，不表示只有“轻量问答”和“重型开发”两种选择。Route A 直连、passthrough、plan、CodeAgent、ChatAgent 都是不同执行路线。
6. `ChatAgent` / 轻量问答路线只服务于“明确不需要项目执行”的项目问答；它可以不做 Git 检查、不读项目文档、不修改文件、不注入发布规则。用户选择项目 CLI 透传时，不因为消息短就改走这条路线。
7. 只有 `CodeAgent`、图片转项目资产、编译、部署、发布、代码修改等执行路线，才进入项目队列并注入通用项目工作流和强制 Git/文档/验证规则。非执行类项目问答不能抢项目锁，也不能在进入 agent 前触发 `git pull`。
8. Codex CLI only 模式会忽略 APK/Web 传来的非 Codex 预设 `agent` 选择，并关闭 API fallback，避免 CLI 失败后切回 Hunyuan、TokenHub 或其他 API 模型；唯一例外是 `AI_USER_BYOK_API_ENABLED=true` 时用户自己保存的自定义 API base/key/model。
9. 需要恢复多模型路由时，必须显式设置 `AI_CODEX_CLI_ONLY=false`，并同步检查 APK 模型选择 UI、服务端 fallback 和本文档。

注意：当前的 `intent_router` 是服务端本地确定性规则，不是另一个 AI 模型。它是路线辅助层，不是产品主线，也不是“普通聊天优先”的判断器。默认情况下真正需要模型理解、回复、澄清和代码协作的环节都走 Codex CLI。用户 BYOK API Agent 是显式选择的例外；API fallback 默认关闭，未来恢复通用 API 旁路必须显式设置 `AI_ALLOW_API_FALLBACK=true`。

只有在未显式指定执行路线，且本地规则怀疑消息可能需要进入开发流程时，才需要先做轻量意图确认，避免非执行类项目问答误进入 `git pull`、构建或发布。服务端可以调用同一个 Codex CLI 原生 session 做一次确认：Codex 高置信返回 `development` 才进入强流程；返回 `chat` 或置信度不足时，直接使用本次确认给出的 `chat_reply` 回复。这个确认器在返回 `chat` 时也要尽量正常回答用户的问题，只有确实看不懂时才追问，不应因为消息里出现 APK、项目、服务器或 Git 就要求用户重说。例如“APK 是否支持多个手机登录/并行修改/会不会冲突”属于能力和流程讨论，可以按非执行类项目问答回答；只有“现在帮我改代码、打包、发布、提交、推送”才进入 development。

## 多模型旁路原则

以后即使恢复或新增其他 AI 模型，Codex CLI 仍然是项目会话主线。其他模型只能承担轻量分类、摘要、图片/特殊分析、检索增强等旁路任务，不能成为长期对话 owner，也不能直接接管代码协作。

旁路模型的输入和输出必须被服务端整理成简洁上下文，再回灌到当前 APK 会话绑定的 Codex CLI 原生 session。这样同一个会话里的用户意图、分析结论、代码修改和后续追问仍然连续落在 Codex CLI 上下文里，不会因为临时调用其他模型而断层。

## 测试期图片处理策略

当前测试阶段，图片相关请求统一交给 Codex CLI 处理完成，服务端聊天链路不再自动调用独立文生图模型。
保留 `TextToImage` / `ImageAssetForApp` 意图，是为了后续恢复独立图片模型时不需要重做语义分类。

## 关于低价 AI 分类器

可以引入低价 token 的分类模型，但它应该是“补充层”，不是唯一判断来源。建议顺序：

1. 先读取用户或产品功能是否显式指定路线。
2. 未显式指定时，再跑 `intent_router` 的确定性规则。
3. 当置信度低于阈值，例如 `< 70`，再调用低价分类模型。
4. 分类模型只输出结构化 JSON，不直接决定执行。
5. 服务端再用能力矩阵做最终校验，防止模型把不能执行的能力误分出去。

建议 JSON 结构：

```json
{
  "intent": "image_asset_for_app",
  "confidence": 0.86,
  "needs_image_generation": false,
  "needs_code_change": true,
  "reason": "用户要求生成图标并替换 App 启动图标"
}
```

## 给并行 AI 代理的约定

- 新增能力时，优先扩展 `server/src/intent_router.rs`，不要在 `agent.rs`、Web handler 或 APK handler 里复制分流逻辑。
- 新增意图必须添加单元测试，至少覆盖正例、近似反例、和混合意图。
- Web、APK、未来 Win 端都只负责采集用户输入和展示结果，不承担核心能力分流。
- 讨论问题时先确认当前功能选择的路线。不要把“普通聊天 / 轻量聊天 / 避免误触发重型开发流程”当成默认主线或优先建议；它只是非执行类项目问答路线的安全约束。
- 文档、UI、诊断文案优先使用具体路线名：项目 CLI 透传、Route A 直连、项目执行、非执行类项目问答。少用“普通聊天/闲聊”作为项目会话的总称。
- 当前 Codex CLI only 模式下，用户历史保存的聊天模型配置不能覆盖任何能力路线；项目 CLI 透传、意图后的执行、非执行类项目问答和开发协作都只走 Codex CLI。
- 如果用户或功能显式选择 Route A / passthrough / plan / Route B/C，就尊重该路线，不因为消息短、像问候或像解释性问题而改走轻量问答。
- 多模型恢复后，任何非 Codex 模型都只能作为旁路证据源；它的结论必须写回统一会话记录，并作为后续提示输入到对应 Codex CLI session。
- 测试期图片请求必须继续走 Codex CLI；恢复独立图片模型前，需要同步更新路由、测试和本文档。
- 如果新增低价分类模型，请把它放在 `intent_router` 的低置信度补充层，并保留确定性规则作为第一道门。

## 当前限制

- `/api/image/generate` 仍是独立图片接口；但聊天/项目请求链路在测试期不会自动调用它。
- 路由现在是启发式关键词和上下文判断，适合当前产品阶段；复杂语义后续可接低价分类模型增强。
