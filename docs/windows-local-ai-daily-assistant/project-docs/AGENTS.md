# Windows 本地 AI 工作日报助手 - AI 工作入口

最后更新：2026-07-09

本文件是未来独立项目仓库的 AI 入口。所有 Codex / Copilot / Claude / Gemini 等代理开始任务前必须先读本文件。

## 一句话结论

本项目先做 Windows 本地个人工作日报助手，默认保护用户隐私，不做员工监控、云端团队后台或静默采集。

## 必读顺序

1. 读 `AI_RULES.md`，确认硬边界。
2. 读 `AI_PROJECT.md`，理解项目目标、用户和 MVP 范围。
3. 读 `AI_ARCHITECTURE.md`，理解模块分层和数据流。
4. 读 `AI_INDEX.md`，按任务定位相关文档和未来源码入口。
5. 按任务类型读取 `docs/*.md` 专项约束。
6. 使用 `AI_TASK_TEMPLATE.md` 整理执行计划。

## 按任务读取文档

| 任务类型 | 继续读取 |
|---|---|
| 产品范围、需求、验收标准 | `AI_PROJECT.md`、`docs/product-scope.md` |
| 隐私、截图、数据保留、用户授权 | `docs/privacy-and-data-policy.md` |
| AI Vision、prompt、结构化 JSON、成本 | `docs/ai-analysis-policy.md` |
| Windows 截图、前台应用、托盘、系统权限 | `docs/windows-capture-policy.md` |
| 本地数据库、数据清理、迁移 | `docs/local-storage-policy.md` |
| UI、时间线、日报预览、设置页 | `docs/ui-product-rules.md` |
| 构建、测试、打包、发布 | `docs/build-and-release-policy.md` |

## 项目硬边界

- 不做远程员工监控。
- 不做键盘记录。
- 不做录屏。
- 不做无法退出的后台常驻。
- 不绕过用户授权截图。
- 不默认保存截图原图。
- 不把 API key、截图、数据库或日志提交到 Git。
- 不在未确认隐私策略前接入真实云端 Vision API。

## 修改前要求

任何代码任务开始前，AI 必须先说明：

1. 要改哪些模块。
2. 会影响用户隐私还是只影响本地 UI。
3. 是否会触碰截图、上传、数据库、删除数据、后台常驻。
4. 需要运行哪些验证。
5. 有哪些风险需要项目负责人拍板。

## 当前阶段

当前阶段是文档和约束设计，不是编码阶段。

允许：

- 完善 PRD。
- 完善架构文档。
- 完善隐私和 AI 约束。
- 设计数据结构。
- 设计 UI 原型说明。

不允许：

- 直接写截图采集代码。
- 直接接真实 AI API。
- 直接创建后台常驻服务。
- 直接上传用户屏幕内容。
