# Copilot 共享项目指令

本文件是项目内所有 AI 代理的共享规则权威来源。Copilot 可以自动读取本文件；Codex、Claude、Gemini 和其它 CLI 通过 `AGENTS.md`、`CODEX.md`、`CLAUDE.md` 或 `GEMINI.md` 跳转到这里。

## 工作原则

- 先理解用户目标、当前项目结构、真实调用链和完成标准，再修改代码。
- 优先复用项目已有框架、脚本、目录、命名和数据模型。
- 保持改动小而完整：实现、验证、提交，并说明结果。
- 不覆盖用户已有文件；发现脏工作区、冲突、缺失依赖或权限问题时，先诊断再处理。
- 不把其他项目的发布脚本、目录约定或业务规则套到本项目。
- 不在多个 AI 工具文档里复制规则；通用规则只写在本文件，专项规则写到 `.github/instructions/`。

## 开始任务

1. 查看当前目录、Git 状态和用户需求。
2. 阅读 `AGENTS.md` 和本文件。
3. 按任务读取相关专项文档：
   - Git、提交、分支、发布、回滚：`.github/instructions/git-workflow.instructions.md`
   - Android、APK、Gradle、移动端构建：`.github/instructions/android.instructions.md`
   - UI、交互、样式、移动端页面：`.github/instructions/ui.instructions.md`
   - 后端、API、数据库、鉴权、服务部署：`.github/instructions/backend.instructions.md`
   - 任务卡住、跨模块改动、流程不清：`.github/instructions/project-workflow.instructions.md`
4. 搜索相关源码，确认真实入口、状态来源、数据模型和调用链。

## 实现规则

- 入口文件只做组装、路由或依赖注入；新逻辑放到职责清楚的模块。
- 能用结构化 API、现有模型或项目内 helper 时，不用临时字符串拼接和重复逻辑。
- 用户可见功能要考虑加载态、空态、错误态、权限态和离线/失败兜底。
- 涉及数据写入、账号、项目、文件、部署或计费时，要确认隔离边界和鉴权条件。
- 修改 AI 文档时，保持 `AGENTS.md`、`CODEX.md`、`CLAUDE.md`、`GEMINI.md` 为轻量桥接文件。

## 完成标准

- 运行与改动风险匹配的最小有效验证：编译、测试、脚本检查、页面检查或接口检查。
- 有代码改动就提交；如果项目配置了远端，按项目规则推送。
- 最终说明要包含：做了什么、验证结果、提交/推送状态、发布状态。
- 如果验证或发布未执行，明确说明原因，不暗示已经完成。
