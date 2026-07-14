---
name: elon-dev-task
description: 按一龙项目共享生命周期契约完成一次代码或文档任务
agent: elon-implementer
argument-hint: "<用户需求或任务描述>"
---

完成用户任务：`${input:task:请输入用户需求}`。

先读 [AGENTS.md](../../AGENTS.md)，按路由选择专项文档；不要固定全量读取 instructions/docs。

- 完整执行共享契约 `WF-START` 至 `WF-REPORT`，脚本输出优先。
- 先定位真实代码和影响范围，再做最小安全改动；避免继续扩大巨型文件。
- 按 `WF-FILES` 处理所有新增源码、测试、fixture 和临时产物。
- 运行风险匹配的验证；平台用户可见改动按共享完成类型决定发布动作与收尾 Kind。
- push 冲突只按 `WF-REBASE` 处理，不追逐持续前进的远端主线。
- 最终必须包含 commit、push/发布证据和统一收尾状态；`FINALIZABLE=false` 时不得宣称完整结束。
