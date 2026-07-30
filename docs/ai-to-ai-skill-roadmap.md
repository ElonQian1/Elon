# 内部 AI-to-AI Skill 路线

最后更新：2026-07-30

## 1. 产品判断

一龙的核心是让用户通过持续讨论，把模糊想法逐步变成清晰、可验收、可交付的产品。

在这个定位下：

- 用户不需要理解或手动组合 Skill。
- 一龙会话主 AI 是长期对话 owner，负责理解目标、维护上下文和向用户解释决策。
- 总调度 AI 根据确认后的目标生成 Matter，并调用 Skill Router 选择能力组合。
- Skill Agent / Worker Bot 在隔离工作区完成正式开发。
- Reviewer / Verifier 独立验收，人类在关键节点决定继续、打回、发布或放弃。
- Skill 是平台内部的官方工程能力，不面向用户售卖，不接受第三方上架，不建设 Skill 市场。

## 2. 目标链路

```text
用户讨论
  -> 一龙会话主 AI 总结目标、约束和验收标准
  -> 总调度 AI 生成 Matter
  -> Skill Router 选择官方 Skill 组合
  -> 写入 Skill 顺序、预算、权限、风险和选择理由
  -> 用户或项目规则确认
  -> Skill Agent / Worker Bot 执行
  -> Reviewer / Verifier 验收
  -> 构建、发布、分发
  -> Context / Taste / Skill 质量数据沉淀
```

## 3. 现有能力与缺口

当前已经具备：

- 一龙 AI 多轮会话和项目上下文。
- `intent_router` 的多路线辅助分流。
- Group AI Coordinator、Matter 计划、Bot 选择、节点授权和执行记录。
- Context compiler、repo map、符号索引、task pack、验证和修复线索。
- 项目频道、任务、artifact、审批、计费和发布链路。

当前还不具备完整的内部 Skill 基础设施：

- 没有统一 Skill manifest、版本、依赖和权限模型。
- 没有平台级 Skill Registry 和语义检索。
- 没有按需求、兼容性、成本、风险和历史质量综合评分的 Skill Router。
- 没有统一的 Skill 安装、运行、审计、质量评价和升级机制。

因此应在现有 Matter、Bot、项目上下文和执行链路上增量补齐 Skill 能力，不重建平行的调度或发布体系。

## 4. Skill MVP

长期只建设和维护内部官方 Skill，不开放市场，也不引入第三方创作者、Skill 交易或 Skill 订阅。

每个 Skill 至少包含：

```text
skill.json
SKILL.md
inputs.schema.json
permissions.json
tests/
examples/
```

`skill.json` 至少描述：

- ID、版本、平台维护者和适用项目类型。
- intents、capabilities、输入、输出。
- 前置条件、权限、风险和成本等级。
- 可组合 Skill、冲突领域和兼容版本。
- 验收方式和历史质量指标。

第一批官方 Skill 建议覆盖：

1. Android 或 Web 项目初始化。
2. 设计规范和 UI 实现。
3. 后端/API/数据模型实现。
4. 测试、审查和发布检查。

Skill Router 的结果必须写入 Matter，并包含选择理由、未选择候选、成本和风险。Skill 不具备购买或售卖能力，也不允许未经审批扩大权限。

## 5. 分阶段计划

### P0：文档和契约

- 固定会话主 AI、总调度 AI、Skill Router 和 Skill Agent 的职责边界。
- 定义 Skill manifest、Matter 关联字段、预算、权限和审计契约。
- 明确官方维护、临时文件、用户确认和发布门禁。

### P1：官方 Skill Registry 与 Router

- 建立官方 Skill Registry。
- 从需求摘要和项目 Context Pack 检索候选 Skill。
- 生成 Skill 组合、顺序、预算、权限和冲突说明。
- 先由人或项目规则确认，再派发给现有 Group AI Bot。

### P2：执行和质量学习

- 记录每次 Skill 调用的成功率、构建通过率、修复次数、token、耗时和用户采纳。
- 将 Matter 验收和失败原因用于排序，但不能自动绕过安全规则。
- 支持版本升级、兼容性检查和回滚。

### P3：内部 Skill 治理

- 官方 Skill 进入 Registry 前必须经过沙箱测试、安全扫描、权限审计和人工审核。
- 支持内部版本升级、依赖和兼容性检查、质量评分与回滚。
- Skill 的模型和节点消耗只作为平台运行成本记录，不形成 Skill 商品、售价或作者收益。
- 项目中沉淀的私有经验默认留在项目内；只有平台团队明确接管并完成脱敏和审核后，才能转成内部官方 Skill。

### P4：应用和模板分发

- Skill 生成的 APK、项目模板和插件可以进入分发仓库。
- 支持版本、更新、评价和二次创作。
- 分发不改变项目所有权、隐私和 Git 可追溯原则。
- 分发对象是应用、模板和插件，不包含 Skill 上架、Skill 交易或 Skill 订阅。

## 6. 当前建议

现在适合先建立内部官方 Skill 的最小契约，再复用现有 Matter 和 Group AI Bot 做一条受控执行链路。

第一项实施任务应是“Skill manifest + Matter 关联契约”，不包含任何 Skill 市场、交易或创作者入驻工作。
