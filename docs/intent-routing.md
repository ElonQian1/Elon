# Intent Routing / 能力路由

## 目标

用户入口会同时来自 Web、APK，未来还可能来自 Windows 客户端。入口可以不同，但服务端需要先判断“这句话要用哪种能力”，再选择具体模型或执行管线。

本模块把分流规则集中在 `server/src/intent_router.rs`，避免在 Web、APK、项目会话、CLI fallback 等不同路径里重复写关键词判断。

## 当前能力矩阵

| 能力路线 | 代码枚举 | 主要执行者 | 适合请求 |
| --- | --- | --- | --- |
| 普通聊天 | `CapabilityRoute::ChatAgent` | API chat agent | 闲聊、解释、配置问题、模型选择说明 |
| 代码/项目开发 | `CapabilityRoute::CodeAgent` | Codex CLI 优先，失败可回退 API agent | App、Web、服务端、APK、部署、修复、重构 |
| 文生图 | `CapabilityRoute::TextToImage` | `image_generation` | “画一张图”“生成头像/海报/壁纸” |
| 先文生图再代码 | `CapabilityRoute::ImageThenCode` | 文生图模型 + 代码代理 | “生成 App 图标并替换”“做启动图并放进项目” |

## 意图类型

`UserIntent` 是用户语义，`CapabilityRoute` 是执行路线。

| 意图 | 执行路线 | 是否需要图片 | 是否需要改代码 |
| --- | --- | --- | --- |
| `NormalChat` | `ChatAgent` | 否 | 否 |
| `ModelConfig` | `ChatAgent` | 否 | 否 |
| `AppDevelopment` | `CodeAgent` | 否 | 是 |
| `TextToImage` | `TextToImage` | 是 | 否 |
| `ImageAssetForApp` | `ImageThenCode` | 是 | 是 |
| `Unknown` | `ChatAgent` | 否 | 否 |

## 运行顺序

1. `agent::run_dispatch_with_workspace` 先调用 `intent_router::classify(user_message)`。
2. 如果是 `TextToImage`，直接调用 `image_generation::generate_text_to_image`，不受用户当前选择的 Codex CLI/API 聊天模型影响。
3. 如果是 `ImageThenCode`，先生成图片，拿到 `image_url` 后，把原始需求、图片 URL、图片提示词一起交给代码代理。
4. 其他开发类请求优先走 Codex CLI；只有用户显式指定 API agent，或 CLI 不可用并允许 fallback 时，才进入 API agent。
5. 普通聊天和配置解释请求走 API chat agent，不触发项目工具。

## 为什么不让 Codex CLI 直接处理文生图

当前 Codex CLI 是代码代理能力，不能稳定返回真实图片文件。文生图属于独立能力，需要走配置了 `IMAGE_API_KEY` 的图片模型。

因此路由规则是：只要意图明确需要真实图片，就先使用图片模型；如果还需要修改 App/Web/项目资源，再把图片结果交给代码代理。

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
  "needs_image_generation": true,
  "needs_code_change": true,
  "reason": "用户要求生成图标并替换 App 启动图标"
}
```

## 给并行 AI 代理的约定

- 新增能力时，优先扩展 `server/src/intent_router.rs`，不要在 `agent.rs`、Web handler 或 APK handler 里复制分流逻辑。
- 新增意图必须添加单元测试，至少覆盖正例、近似反例、和混合意图。
- Web、APK、未来 Win 端都只负责采集用户输入和展示结果，不承担核心能力分流。
- 用户历史保存的聊天模型配置不能覆盖开发类能力路线；开发类默认走 Codex CLI，显式选择某个 agent 时才按用户选择执行。
- 如果新增图片下载/资源落盘能力，应优先补齐 `ImageThenCode` 的资产导入链路，而不是让代码代理猜测图片来源。
- 如果新增低价分类模型，请把它放在 `intent_router` 的低置信度补充层，并保留确定性规则作为第一道门。

## 当前限制

- `ImageThenCode` 当前是“图片 URL 桥接”：服务端先生成图片 URL，再把 URL 交给代码代理。代码代理是否能下载并保存图片，取决于运行环境和网络权限。
- 文生图依赖 `IMAGE_API_KEY`、`IMAGE_API_BASE`、`IMAGE_MODEL` 等环境变量；未配置时会返回明确错误。
- 路由现在是启发式关键词和上下文判断，适合当前产品阶段；复杂语义后续可接低价分类模型增强。
