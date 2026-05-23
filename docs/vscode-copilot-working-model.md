# VS Code Copilot 工作方式速记

> 更新时间: 2026-05-23  
> 依据: VS Code 官方 Copilot 文档。

## 一句话模型

VS Code Copilot 现在更接近一个可配置的 agent 系统，而不只是聊天或补全。一次任务的基本链路是: 组装上下文 -> 模型判断下一步 -> 调用工具读取/编辑/运行命令 -> 把工具结果放回上下文 -> 继续迭代 -> 验证并交付。

## 上下文模型

Copilot 能推理的内容只来自当前请求的上下文。VS Code 会把以下信息组装给模型:

- 系统指令和内置 agent 行为。
- 项目、用户、组织级 customizations，包括 instructions、custom agents、skills。
- 用户当前消息和当前会话历史。
- 隐式上下文，如当前文件、选择区、可见错误、Git 状态。
- 显式引用，如 `#file`、编辑器上下文、网页内容。
- 工具输出，如文件读取、终端命令、搜索结果。

实践原则: 不要假设 Copilot 自动知道整个仓库。关键文件、约束和验证命令要通过 instructions、prompt files、agent 文件或显式引用进入上下文。

## 自定义层级

| 类型 | 默认位置 | 触发方式 | 用途 |
|---|---|---|---|
| Always-on instructions | `.github/copilot-instructions.md`、`AGENTS.md`、`CLAUDE.md` | 自动进入每次 chat 请求 | 仓库级架构、编码规范、安全边界 |
| File-based instructions | `.github/instructions/*.instructions.md` | `applyTo` glob 或任务语义匹配 | Android、Rust、测试、部署等分场景规则 |
| Prompt files | `.github/prompts/*.prompt.md` | 手动 `/prompt-name` 调用 | 重复任务、固定输出格式、轻量工作流 |
| Custom agents | `.github/agents/*.agent.md` | 在 agent 下拉框选择，或被 prompt/agent 引用 | 固定 persona、工具集、模型、handoff |
| Skills / plugins / MCP / hooks | 按各自配置 | agent 会话中启用 | 多文件能力、外部系统、生命周期命令 |

要点:

- `.github/copilot-instructions.md` 适合短而稳定的项目级规则。
- `*.instructions.md` 适合带 `applyTo` 的局部规则，不要把所有细节塞进全局文件。
- `*.prompt.md` 是 slash command。它可以声明 `description`、`name`、`argument-hint`、`agent`、`model`、`tools`。
- `*.agent.md` 可以声明 `tools`、`agents`、`model`、`handoffs`、`hooks` 等。规划 agent 应限制为只读工具；实现 agent 才开放编辑和验证工具。
- 同时存在 prompt tools 和 custom agent tools 时，prompt 文件里的 tools 优先。
- VS Code 1.102 起，面向代码生成和测试生成的 settings-based instructions 已弃用，应使用文件型 instructions；代码审查、commit message、PR 描述仍可用 settings 指令。

## Agent Loop

官方 agent loop 可压成三步:

1. Understand: 读取文件、搜索代码、查询文档，理解任务和约束。
2. Act: 编辑代码、运行终端、调用 MCP/API 或其他工具。
3. Validate: 跑测试、检查编译错误、自审改动；失败则继续迭代。

工具分三类: VS Code 内置工具、MCP 工具、扩展贡献工具。工具越多，上下文和决策空间越大；对固定任务应通过 prompt files 或 custom agents 限定工具集。

## Planning、Subagents、Memory

- Plan agent 用于复杂任务的先研究后计划: discovery、alignment、design、refinement。计划确认前不写代码。
- Subagents 拥有独立上下文，只返回最终结论，适合并行做安全、性能、可访问性或代码模式研究。
- Memory 分两类: 本地 memory tool 和 GitHub-hosted Copilot Memory。前者有 user/repository/session scope；后者跨 Copilot surfaces 共享，但需要显式启用，并且要验证记忆是否仍符合当前代码。

## 本项目采用方式

- 仓库级总原则继续放在 `.github/copilot-instructions.md`。
- Git、部署、并发工作树等强制流程继续放在 `.github/instructions/git-deploy-workflow.instructions.md`，通过 `applyTo: "**"` 自动注入。
- 若以后出现高频任务，优先放进 `.github/prompts/*.prompt.md`，例如生成 APK 发布说明、准备部署检查单、修复失败构建。
- 若要分离角色，使用 `.github/agents/*.agent.md`: planner 只读，implementer 可编辑和验证，reviewer 只读审查。
- 修改 AI customization 时，先检查文件名、frontmatter 和默认目录是否符合 VS Code 约定，再提交。

## 官方资料

- VS Code Copilot customization overview: https://code.visualstudio.com/docs/copilot/customization/overview
- Custom instructions: https://code.visualstudio.com/docs/copilot/customization/custom-instructions
- Prompt files: https://code.visualstudio.com/docs/copilot/customization/prompt-files
- Custom agents: https://code.visualstudio.com/docs/copilot/customization/custom-agents
- Context: https://code.visualstudio.com/docs/copilot/concepts/context
- Tools: https://code.visualstudio.com/docs/copilot/concepts/tools
- Agents: https://code.visualstudio.com/docs/copilot/concepts/agents
- Planning: https://code.visualstudio.com/docs/copilot/agents/planning
- Memory: https://code.visualstudio.com/docs/copilot/agents/memory
