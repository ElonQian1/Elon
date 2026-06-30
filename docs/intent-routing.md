# Intent Routing / 能力路由

## 目标

用户入口会同时来自 Web、APK，未来还可能来自 Windows 客户端。入口可以不同，但服务端需要先判断“这句话要用哪种能力”，再选择具体模型或执行管线。

本模块把分流规则集中在 `server/src/intent_router.rs`，避免在 Web、APK、项目会话、CLI fallback 等不同路径里重复写关键词判断。

长期上，能力路由不只区分聊天和代码开发，还要识别用户当前处于“产品讨论、demo 预演、Skill 规划、正式开发、验收发布”中的哪个阶段。现有 `intent_router` 继续承担安全分流；总调度 AI 在其后负责需求成熟度判断和 Skill 选择，不能让低成本分类模型直接获得写代码或发布权限。

## 当前能力矩阵

| 能力路线 | 代码枚举 | 主要执行者 | 适合请求 |
| --- | --- | --- | --- |
| 普通聊天 | `CapabilityRoute::ChatAgent` | Codex CLI only（当前测试期强制） | 闲聊、解释、配置问题、模型选择说明 |
| 代码/项目开发 | `CapabilityRoute::CodeAgent` | Codex CLI only（当前测试期强制） | App、Web、服务端、APK、部署、修复、重构 |
| 图片处理（测试期） | `CapabilityRoute::CodeAgent` | Codex CLI only | “画一张图”“生成头像/海报/壁纸”“生成 App 图标并替换” |

## 目标能力阶段

| 产品阶段 | 主要执行者 | 目的 | 默认副作用 |
| --- | --- | --- | --- |
| `ProductDiscussion` | 一龙会话主 AI | 通过讨论澄清用户、场景、目标和约束 | 无 |
| `DemoPreview` | 预言家 AI / Demo Oracle | 用低成本生成静态 demo、草图、流程和待确认问题 | 只写临时 demo 空间 |
| `SkillPlanning` | 总调度 AI + Skill Router | 选择 Skill 组合，生成 Matter、预算、风险和验收标准 | 无正式代码写入 |
| `FormalDevelopment` | Skill Agent / Worker Bot | 在隔离 worktree 中实现正式功能 | 受审批的代码修改 |
| `ReviewAndRelease` | Reviewer / Verifier + 人类决策者 | 审查、构建、发布和分发 | 受审批的合并/发布 |

这些是产品目标阶段，不表示当前代码枚举已经全部实现。MVP 应先在现有 `ChatAgent -> CodeAgent` 之间增加可选的 `DemoPreview` 和 `SkillPlanning`，并保持小改动可以直接进入正式开发。

## 意图类型

`UserIntent` 是用户语义，`CapabilityRoute` 是执行路线。

| 意图 | 执行路线 | 是否需要图片 | 是否需要改代码 |
| --- | --- | --- | --- |
| `NormalChat` | `ChatAgent` | 否 | 否 |
| `ModelConfig` | `ChatAgent` | 否 | 否 |
| `AppDevelopment` | `CodeAgent` | 否 | 是 |
| `TextToImage` | `CodeAgent` | 否 | 是 |
| `ImageAssetForApp` | `CodeAgent` | 否 | 是 |
| `Unknown` | `ChatAgent` | 否 | 否 |

## 运行顺序

1. `agent::run_dispatch_with_workspace` 先调用 `intent_router::classify(user_message)`。
2. 当前默认启用 `AI_CODEX_CLI_ONLY=true`，不论路由结果是普通聊天、模型配置、图片处理还是项目开发，默认执行后端都会被锁定为 Codex CLI；用户显式保存自带 API Key 的 BYOK 配置时，可作为例外走 API Agent。
3. Codex CLI only 只表示“主执行者是 Codex CLI”，不表示每句话都走重型开发流程。`ChatAgent` 普通聊天继续绑定同一个 Codex CLI 原生 session，但使用轻量聊天 prompt，不做 Git 检查、不读项目文档、不修改文件、不注入发布规则。
4. 只有 `CodeAgent`、图片转项目资产、编译、部署、发布、代码修改等开发路线，才进入项目队列并注入通用项目工作流和强制 Git/文档/验证规则。普通聊天不能抢项目锁，也不能在进入 agent 前触发 `git pull`。
5. Codex CLI only 模式会忽略 APK/Web 传来的非 Codex 预设 `agent` 选择，并关闭 API fallback，避免 CLI 失败后切回 Hunyuan、TokenHub 或其他 API 模型；唯一例外是 `AI_USER_BYOK_API_ENABLED=true` 时用户自己保存的自定义 API base/key/model。
6. 需要恢复多模型路由时，必须显式设置 `AI_CODEX_CLI_ONLY=false`，并同步检查 APK 模型选择 UI、服务端 fallback 和本文档。

注意：当前的 `intent_router` 是服务端本地确定性规则，不是另一个 AI 模型。它只做安全分流，防止一句普通聊天误触发 `git pull`、构建或发布。默认情况下真正需要模型理解、回复、澄清和代码协作的环节都走 Codex CLI。用户 BYOK API Agent 是显式选择的例外；API fallback 默认关闭，未来恢复通用 API 旁路必须显式设置 `AI_ALLOW_API_FALLBACK=true`。

当本地规则怀疑消息需要进入开发流程时，不能立刻抢项目锁或执行 Git 操作。服务端必须先调用同一个 Codex CLI 原生 session 做一次轻量意图确认：Codex 高置信返回 `development` 才进入强流程；返回 `chat` 或置信度不足时，直接使用本次 Codex 轻量确认给出的 `chat_reply` 回复，避免普通聊天被误判后等待很久。这个轻量确认器在返回 `chat` 时也要尽量正常回答用户的问题，只有确实看不懂时才追问，不应因为消息里出现 APK、项目、服务器或 Git 就要求用户重说。若 Codex 已判定为 `chat`，但 `chat_reply` 仍是低价值澄清句，服务端可以用固定护栏回复兜底，确保用户得到“这不会进入开发流程”的及时反馈。例如“APK 是否支持多个手机登录/并行修改/会不会冲突”属于能力和流程讨论，应该按 chat 回答；只有“现在帮我改代码、打包、发布、提交、推送”才进入 development。

## 多模型旁路原则

以后即使恢复或新增其他 AI 模型，Codex CLI 仍然是项目会话主线。其他模型只能承担轻量分类、摘要、图片/特殊分析、检索增强等旁路任务，不能成为长期对话 owner，也不能直接接管代码协作。

旁路模型的输入和输出必须被服务端整理成简洁上下文，再回灌到当前 APK 会话绑定的 Codex CLI 原生 session。这样同一个会话里的用户意图、分析结论、代码修改和后续追问仍然连续落在 Codex CLI 上下文里，不会因为临时调用其他模型而断层。

## 测试期图片处理策略

当前测试阶段，图片相关请求统一交给 Codex CLI 处理完成，服务端聊天链路不再自动调用独立文生图模型。
保留 `TextToImage` / `ImageAssetForApp` 意图，是为了后续恢复独立图片模型时不需要重做语义分类。

## 关于低价 AI 分类器

可以引入低价 token 的分类模型，但它应该是“补充层”，不是唯一判断来源。建议顺序：

1. 先跑 `intent_router` 的确定性规则。
2. 当置信度低于阈值，例如 `< 70`，再调用低价分类模型。
3. 分类模型只输出结构化 JSON，不直接决定执行。
4. 服务端再用能力矩阵做最终校验，防止模型把不能执行的能力误分出去。

预言家 AI 可以复用低价模型，但它不是纯分类器。分类器只回答“当前属于什么阶段”；预言家 AI 要基于总调度 AI 的需求摘要生成可讨论 demo。两者都不能绕过确定性安全规则、项目权限和 Matter 审批。

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
- 当前 Codex CLI only 模式下，用户历史保存的聊天模型配置不能覆盖任何能力路线；普通聊天、意图后的执行和开发协作都只走 Codex CLI，但普通聊天必须使用轻量 prompt，不能触发 Git/构建/发布强流程。
- 多模型恢复后，任何非 Codex 模型都只能作为旁路证据源；它的结论必须写回统一会话记录，并作为后续提示输入到对应 Codex CLI session。
- 测试期图片请求必须继续走 Codex CLI；恢复独立图片模型前，需要同步更新路由、测试和本文档。
- 如果新增低价分类模型，请把它放在 `intent_router` 的低置信度补充层，并保留确定性规则作为第一道门。

## 当前限制

- `/api/image/generate` 仍是独立图片接口；但聊天/项目请求链路在测试期不会自动调用它。
- 路由现在是启发式关键词和上下文判断，适合当前产品阶段；复杂语义后续可接低价分类模型增强。
