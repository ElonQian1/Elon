---
version_status: current
reviewed_at: 2026-08-05
implementation_status: implementation_uncompiled
---

# 项目功能需求登记与代理任务生命周期

本文定义“Codex 写正式需求文档后，其他 AI 代理如何低 token 地发现、认领、实现和验收该模块”。它是工作流合同，不是需求正文或实现真源。

## 真源与信源顺序

- 正式需求正文继续位于 Git 工作区 Markdown；Codex 使用原生编辑工具创建和修改正文。
- `.elon/project-features.json` 只保存短摘要、状态、优先级、需求路径与哈希、验收标准、依赖、任务路径、认领和实现证据，不复制 Markdown 或源码正文。
- 当前源码、测试和运行证据是实现真源；current 约束文档和已接受需求定义方向；Feature Registry 只提供工作流导航。
- `drafts`、`inbox`、讨论、历史和归档文档不能登记为 `accepted` 或 `ready`。
- 需求或实现证据 SHA-256/Git 身份漂移时，认领、验收和轻量上下文失败关闭；只有显式重绑工具能接受有意修改，并强制回退评审状态，不能静默选择旧信源。

## 状态机

```text
draft → proposed → accepted → ready → claimed → in_progress
                                                ↓
                                      blocked / implemented
                                                    ↓
                                             verified → released → retired
```

状态不能任意跳转：

- `claimed` 只能由认领工具创建，并带 5–1440 分钟租约；未过期的其他认领不能覆盖。
- 每次新认领或过期重认领都会清除上一轮实现证据，避免旧 hash 在重新打开功能后冒充本轮产出；代理可重新绑定仍然有效的当前文件。
- `in_progress` 必须持有匹配且未过期的 `claim_id`。
- `implemented` 必须已有服务端绑定的当前实现证据。
- `verified` 和 `released` 还必须至少有一条当前 `test` 证据。
- 依赖功能只有进入 `verified` 或 `released` 才视为完成；依赖未完成的 `ready` 功能不能认领。
- 回退修改先回到 `ready`，重新认领后继续，避免复用已经释放的 claim。

## MCP 工具

完整 governance profile 提供以下独立工具：

- `project_features_register`：需求正文已经存在后登记功能；服务端绑定当前需求哈希。
- `project_features_list`：分页查询状态、优先级、阻塞、认领和需求漂移，不读正文，也不逐条读取实现证据文件；实现证据显示为“按需校验”。
- `project_features_update`：更新标题、摘要、优先级、归属、标签、路径、依赖或验收标准；已接受范围发生变化会回退 `proposed` 并撤销旧证据。
- `project_features_rebind_requirement`：需求有意变化或移动后重绑哈希；已接受、ready 或 blocked 项必须回到 `proposed` 重新评审。
- `project_features_plan`：返回单功能的紧凑任务包、验收标准、依赖和证据入口，并精确校验该功能的实现证据。
- `project_features_claim` / `project_features_release_claim`：带 registry revision 和租约的认领/释放；过期 claim 可显式清理，需求或依赖仍无效时释放到 `blocked` 而不是伪装 `ready`。
- `project_features_transition`：按固定状态机推进。
- `project_features_record_evidence`：调用方只传相对路径、定位符和类型，服务端计算当前 SHA-256/Git 对象。
- `project_features_check_drift`：只读检查需求和实现证据，返回非自动修复计划。
- `project_features_history`：按时间倒序分页读取全局或单功能审计事件；只返回最近最多 200 条有界生命周期元数据，不返回正文。

所有写工具采用 optimistic `expected_registry_revision`。保存层在当前 worktree 的 Git 元数据目录取得跨进程独占锁，并在锁内重新核对磁盘 revision；注册表已经存在却不传 revision、两个代理同时首次创建，或其他会话已经修改时，操作都会失败关闭。工具不自动 push，也不把任务文本、聊天、Prompt、命令输出或 Codex 私有 Memories 写入项目。

普通 Codex/直接插件另有一个最小 `profile=feature`，只暴露 `project_feature_workflow`。它用 `action` 路由到同一组服务端操作；字段未知时先执行 `describe`，详细 action schema 只在这次显式请求中返回。这样无需打开 PC 页面，也无需让每次普通编码任务携带完整文档治理工具目录。

## 轻量上下文

普通 `profile=context` 仍只暴露一个 `project_context_plan`。响应新增 `relevant_features`：

- 只考虑 `accepted` 至 `implemented` 的活动功能；
- 结合 query、`task_paths` 和优先级排序；只对排序后的前 12 个候选校验需求哈希，避免大型注册表把一次上下文发现退化为全库文件扫描；
- 最多返回 3 个功能，每项只含短摘要、状态、优先级、需求路径/哈希、最多 3 条验收标准和 4 个任务路径；
- 已检查候选中的需求漂移项不进入 selected，而进入有界 `invalidated` 与 `source_conflict_summary`；响应同时报告候选数、实际校验数和校验上限，不能把未进入前 12 的候选误解为“已校验且有效”；
- 同一 plan receipt 和响应预算继续负责去重，Feature Registry 不增加普通任务工具 schema。

`profile=context` 只负责低成本发现，不具备登记、重绑、认领或状态写权限。代理必须先打开命中的需求文档，再用原生搜索/读取核对当前源码和测试；需要推进生命周期时，调用独立单工具 feature profile，或进入显式完整 governance 会话。已有明确单文件目标时仍可跳过项目上下文计划。

## 标准登记流程

1. Codex 使用原生工具创建正式 current 需求 Markdown，写明目标、非目标、验收标准、依赖和预计实现范围。
2. 调用 `project_features_list` 获取当前 registry revision；首次尚无注册表时 revision 为空。
3. 调用 `project_features_register`，分配稳定 `feature_id`；可关联现有 `knowledge_node_id`，但不能假装未知图节点已经存在。
4. 将需求 Markdown 和 `.elon/project-features.json` 放在同一正常 Git 提交中。
5. 评审变化使用 `project_features_update`；需求正文有意变化则使用 `project_features_rebind_requirement`，重新接受后再进入 `ready`。
6. 后续代理用 `project_features_plan` 获取小任务包；`ready` 且依赖完成后认领。
7. 实现中用原生工具工作；完成后只回写证据路径/定位符，由服务端绑定哈希。
8. 进入 `implemented` 后独立验证；具备当前 test 证据才进入 `verified`，发布后进入 `released`。

PC 项目文档工作台的“功能需求”分区读取同一 loopback API，并只负责展示及生成代理指令，不在浏览器里复制状态机或自行裁决信源。页面按 100 条稳定 revision 分页读取，最多覆盖注册表合同允许的 512 条；分页期间 revision 改变会失败关闭并要求刷新。列表里的实现证据默认标注“按需校验”；只有明确打开单功能计划或执行漂移检查，服务端才读取证据文件并给出精确结论。

该分区发起 AI 任务时直接发送功能指令，不附带文档整理的完整长 Prompt；普通 Codex 因而只加载 context、feature、receipt 三个单工具 profile，并通过 `project_feature_workflow` 的 action 按需取得详细字段合同。完整 governance 工具目录仍只属于显式文档治理会话。

## 当前边界

- 本批次代码已形成但未编译、未运行 Rust/npm 测试，也未做浏览器和真实 Codex/MCP 验收。
- Git 中的 claim 只能对共享到同一分支/工作区的参与者可见；跨 worktree、离线代理和远端并发仍需真实冲突策略验证。
- 注册表不会自动创建 Markdown；这样保留 Codex 原生编辑、Git diff 和用户审核能力。
- 注册表不会自动修改 `AI_CURRENT.md`、`AI_INDEX.md` 或知识图谱；稳定项目入口仍由文档治理流程审核，避免一次需求登记同时改写多份权威信源。
- Feature Registry 不替代团队 backlog、项目管理平台或供应商私有任务记忆；未来适配器必须继续以该 Git 合同为项目内真源。
