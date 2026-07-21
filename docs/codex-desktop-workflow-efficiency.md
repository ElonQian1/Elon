# Codex 桌面工作流效率与可靠性协议

最后更新：2026-07-21

本文说明 Codex 桌面端与一龙 PC 节点之间的低 token 工作协议。优化目标是减少重复上下文和重复证据，同时保持可恢复、可审计和失败关闭。节省比例必须用同任务 A/B 数据验证，不以估算值代替实测。

## 优先级

1. P0：任务终态可退出、Wait 真增量、协议能力协商、权威 Resume 上下文。
2. P1：历史 capability gap 对账、Android Runtime 幂等重连。
3. P2：FitRun 动态区域 mask、稳定节点选择器、MCP 工具契约摘要。
4. P3：不可变收尾契约、同任务 A/B 度量。

## Wait 增量协议

桌面端要求节点声明 `delta_wait_v1`，未声明时不得静默退回全量轮询。每次 Wait 只返回本次游标窗口内的新增事件，并附带：

- `cursorEpoch`：游标世代；变化时客户端丢弃旧窗口的去重状态。
- `nextCursor`：客户端成功消费事件后才持久化。
- `eventKey`：以 `cursorEpoch:sequence` 去重，允许网络重试和服务端重放。
- `stateDigest`、`evidenceDigest`：快速判断状态或证据是否变化。
- 终态证据：只在任务进入终态的增量中携带一次，非终态轮询不重复传输大数组。

客户端可在下一次调用传入 `ExpectedStateDigest` 和 `ExpectedEvidenceDigest`；相同时响应省略对应状态或证据正文，只保留 digest 与变化标志。可靠性来自“至少一次传输 + 客户端去重”，而不是假设网络只投递一次。一次 Wait 内若发生分页或多轮轮询，桌面端先聚合所有页，再更新游标；游标重置后用新世代重新开始，避免漏事件或把不同世代的相同序号误判为重复。

## Resume 上下文协议

桌面端要求节点声明 `resume_context_v1`。Resume 请求只发送父任务、根任务等引用，不复制父任务 Prompt。节点从任务账本编译 `elon.resume_context.v1`：

- 根任务原始目标只出现一次；
- 验收标准以根任务为权威，子任务不得漂移；
- 只包含有界的父任务结果、证据与评审摘要；
- 校验 owner、agent、安装、项目、根任务和 worktree 继承关系；
- 生成确定性摘要和 SHA-256，账本保存引用与摘要，不递归嵌套历史 Prompt；
- 任一校验不成立时失败关闭，不生成猜测式续跑 Prompt。

这消除了多层 `Resume the original task` 和父任务完整 Prompt 的指数式重复。

## 终态与能力协商

Codex CLI sidecar 优先使用明确的 `turn.completed` / `turn.failed` JSON 终态；输出管道支持跨 chunk JSONL 缓冲。旧 CLI 缺少明确终态时，仅在最终 agent message、无在途工具且持续静默达到宽限期后使用保守回退。终态确认后停止空闲/总时限误杀，并回收残留进程。

节点状态公开 `desktop_supervision` 能力与协议版本。桌面端在使用增量 Wait、权威 Resume 或桌面评审票据前先协商能力；评审票据按节点声明选择 v2 公钥签名或 v1 共享凭据。v2 Review 必须显式传入 `-StateRoot` 与 `-InstallRoot`，不得猜测凭据目录。缺失能力时给出明确升级错误，不发送未声明的旧格式请求。

## Runtime、gap 与 UI 稳定性

- Runtime 准备先检查已在线且源码证明匹配的会话；APK 可复用时优先重连、端口转发和握手，失败才进入安装流程。
- 平台 evolution gap 完成并通过复检后，按 origin 关系和能力集合对账历史业务 gap，避免历史 `DEFERRED` 镜像持续阻塞收尾。
- FitRun `visualMask` 只允许目标裁剪区内的动态内容或批注区域，最多 24 个，总面积不超过目标面积 25%。mask 同时用于基线、迭代和最终比较。
- UI 节点优先使用稳定选择器 `definitionId + instanceKey + screenId`；多匹配时必须报歧义，不得选择第一个结果。仍支持明确的 `runtimeNodeId`。
- MCP `tools/list` 返回稳定契约摘要与 digest，客户端可在执行前识别 schema 漂移。

## 不可变收尾契约

Windows 预检为有效任务 worktree 签发 `elon.ai_finish_contract.v1`，保存于仓库外的本机状态目录。收尾命令必须携带 contract ID；收尾脚本重新校验 payload digest、worktree、branch、origin 和 base commit 祖先关系。普通受管 `codex/*` 任务没有契约时失败关闭；`AllowLegacyNoTaskContract` 仅用于旧测试或显式迁移。

## A/B 度量

使用 `scripts/compare-ai-workflow-efficiency.ps1` 对同一任务、同一验收标准、同一模型档位的基线与候选数据进行比较；三项身份不一致时脚本拒绝生成报告。输入 JSON 至少记录：

- `taskFingerprint`、`acceptanceCriteriaDigest`、`modelProfile`；
- `inputTokens`、`cachedInputTokens`、`outputTokens`；
- `durationMs`、`eventCount`、`failureCount`；
- `failedTools`；
- `waitPayloadBytes`。

报告中负数 delta 表示节省。只有 matched-task 数据可用于宣称 token、时延或可靠性改善；不同任务的结果只能作为观察，不能作为因果结论。
