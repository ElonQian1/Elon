# Intent Routing / 能力路由

## 目标

用户入口会同时来自 Web、APK，未来还可能来自 Windows 客户端。入口可以不同，但服务端需要先判断“这句话要用哪种能力”，再选择具体模型或执行管线。

本模块把分流规则集中在 `server/src/intent_router.rs`，避免在 Web、APK、项目会话、CLI fallback 等不同路径里重复写关键词判断。

## 当前能力矩阵

| 能力路线 | 代码枚举 | 主要执行者 | 适合请求 |
| --- | --- | --- | --- |
| 普通聊天 | `CapabilityRoute::ChatAgent` | Codex CLI only（当前测试期强制） | 闲聊、解释、配置问题、模型选择说明 |
| 代码/项目开发 | `CapabilityRoute::CodeAgent` | Codex CLI only（当前测试期强制） | App、Web、服务端、APK、部署、修复、重构 |
| 图片处理（测试期） | `CapabilityRoute::CodeAgent` | Codex CLI only | “画一张图”“生成头像/海报/壁纸”“生成 App 图标并替换” |

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
2. 当前默认启用 `AI_CODEX_CLI_ONLY=true`，不论路由结果是普通聊天、模型配置、图片处理还是项目开发，最终执行后端都会被锁定为 Codex CLI。
3. Codex CLI only 只表示“主执行者是 Codex CLI”，不表示每句话都走重型开发流程。`ChatAgent` 普通聊天继续绑定同一个 Codex CLI 原生 session，但使用轻量聊天 prompt，不做 Git 检查、不读项目文档、不修改文件、不注入发布规则。
4. 只有 `CodeAgent`、图片转项目资产、编译、部署、发布、代码修改等开发路线，才进入项目队列并注入通用项目工作流和强制 Git/文档/验证规则。普通聊天不能抢项目锁，也不能在进入 agent 前触发 `git pull`。
5. Codex CLI only 模式会忽略 APK/Web 传来的非 Codex `agent` 选择，并关闭 API fallback，避免 CLI 失败后切回 Hunyuan、TokenHub 或其他 API 模型。
6. 需要恢复多模型路由时，必须显式设置 `AI_CODEX_CLI_ONLY=false`，并同步检查 APK 模型选择 UI、服务端 fallback 和本文档。

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
