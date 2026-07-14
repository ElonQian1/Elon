# Elon AI Task Template

复杂任务可用下面的轻量模板。项目规则不粘贴进任务正文，由 `AGENTS.md` 路由和 `WF-*` 契约提供。

```md
## 目标

请实现：

## 验收标准

- 用户可见结果：
- 不应改变：
- 需要发布：是 / 否 / 由项目默认规则决定

## 已知上下文

- 相关模块或文件：
- 错误、截图或复现步骤：

## 工作要求

1. 先读 `AGENTS.md` 和共享 `WF-*` 契约，只加载本任务需要的专项文档。
2. 完整执行 `WF-START` 至 `WF-REPORT`，只在 `EDIT_ROOT` 修改。
3. 先定位和规划，再做最小安全改动；不回退或夹带其他任务。
4. 按 `WF-FILES` 处置源码、测试和临时产物，并运行风险匹配的验证。
5. 根据改动选择正确发布动作和统一收尾 Kind。
6. 只有统一收尾输出 `FINALIZABLE=true` 才宣告完成。

## 最终回复

- 修改与验证结果
- commit SHA 与 push/发布状态
- `BUSINESS_STATUS`
- `LOCAL_MAIN_STATUS`
- `MAIN_UNTRACKED_STATUS`
- `TASK_WORKTREE_STATUS`
- 未完成项或风险
```
