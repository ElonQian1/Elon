# 项目记忆与桌面 AI 代理接入

最后更新：2026-08-04

本文只回答 Codex Desktop/CLI、直接安装的 Codex 和其他 MCP 代理如何复用项目文档记忆。完整文档治理接口见 `docs/project-document-governance-mcp.md`。

## 目标与非目标

项目记忆不是第二套代码搜索。代理仍用自己的 `rg`、Git、符号索引和文件读取能力核对当前实现；本功能只提前给出少量高置信导航摘要和证据路径，减少第一次宽搜与重复读取。

固定信源顺序是：

1. 当前源码与测试；
2. 约束性项目规则和 current ADR；
3. Git 中已审核、证据仍有效的导航记忆；
4. 本机待审核候选。

低优先级来源与高优先级来源冲突时，低优先级来源失效，不做自动合并。摘要不保存源码正文、聊天、prompt、transcript、命令或工具输出。

## 可移植记忆

跨 PC 真源是 `.elon/document-sections.json.context_memories`，随 Git 同步。每条记忆包括：

- 最多 800 字符的导航摘要和最多 8 个 topic；
- 1–8 个工作区相对证据路径、定位符、SHA-256，以及可用时的 Git 对象身份；
- owner；
- repository 或 paths scope，并可进一步限制 federation `scope_id`、Git branch、release/channel 与 clean/dirty worktree；
- 审核回执、审核日期、审核者、复核周期和可选到期日。

新 PC 不需要复制旧 PC 的 SQLite 候选。SQLite 只是本机审核收件箱；只有 suggestions/revision/authorization/apply/Git 链路通过后的共享记忆才可移植。

旧共享记忆缺少生命周期字段时继续可读，不自动改写；Memory CI 会提示补齐。到期、漂移或与另一条共享事实存在潜在冲突的记忆不进入 `verified_project_memory`。

共享 manifest 最多保存 256 条记忆。轻量规划先按任务路径、联邦分片、当前分支、release 和工作区状态零 I/O 排除不适用项，再只对相关性最高的 24 条做证据验证，最终最多注入 3 条。调用方没有提供某个限定值时，显式依赖该限定值的记忆失败关闭，不会作为“可能相关”混入上下文。

## Memory CI

完整 governance MCP 提供 `project_docs_check_native_context_memory`；PC 使用同源接口：

```http
POST /api/project-docs/native-context/health
Content-Type: application/json

{
  "project_root": "D:\\repo",
  "offset": 0,
  "limit": 50,
  "failure_policy": "advisory",
  "include_capabilities": false
}
```

失败策略：

- `advisory`：任何问题只返回 `warn`，建议退出码仍为 0；
- `fail_on_drift`：证据漂移或记忆过期返回建议退出码 1；
- `strict`：复核逾期、治理缺口和共享事实潜在冲突也返回建议退出码 1。

响应包含全量计数、当前页、next offset/cursor、逐项状态和 repair plan。repair plan 永远 `automatic=false`：重定位只给候选路径，冲突只要求按当前源码和约束文档人工裁决，更新仍走审核链路。PC 仅在“一条漂移路径恰好对应一个 Git 对象验证过的新路径”时显示显式确认按钮；确认后也只生成 pending 候选，仍需审核与 apply。

仓库提供可执行包装器，由它读取服务端建议退出码并结束 CI 进程：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\project-memory-ci.ps1 `
  -ProjectRoot D:\repo -FailurePolicy strict
```

能力矩阵默认省略，只有需要代理/Hook/app-server/Memories 接入合同时传 `include_capabilities=true`，避免分页 CI 重复消耗响应 token。

## 代理接入矩阵

| 入口 | 当前能力 | 不自动做的事 |
|---|---|---|
| 节点托管的普通 Codex 宽任务 | 自动注入只读 context、只写 receipt 和会话级 Hook 配置 | 不注入完整治理 schema，不绕过 Hook 信任 |
| 直接运行的 Codex Desktop/CLI | 可手动 bootstrap 描述符，或安装仓库内 `plugins/yilong-project-memory` 接入 context/receipt | 仓库只提供插件包，不替用户安装、不自动信任 Hook |
| 其他支持 HTTP MCP 的代理 | 可使用 vendor-neutral governance 描述符；context/receipt 可由适配器手动接入 | 无可靠工具过滤时不自动注入普通任务；Hook 需供应商适配 |
| Codex plugin bundle | 已生成并通过静态 plugin manifest 校验；含两个最小权限 MCP server 与路径账本 Hook | 未安装、未启动、未做真实 Codex 兼容验证，也不声称 Hook 已加载 |

context、receipt、governance profile 与短期 session/token 固定；修改 URL 查询参数不能提升权限。普通任务只使用两个极小 profile，因此不会每次积压完整治理工具 schema 或项目全文。`project_context_plan` 可传 `task_paths`、`scope_id` 和 `release`；服务端另从当前 Git 工作区绑定 branch 与 clean/dirty 状态，这些范围也进入缓存键和 plan receipt。

## Hook 生命周期

当前会话只配置 `PostToolUse`、`Stop`、`SessionEnd`。`SessionStart`、`PreCompact`、`PostCompact`、`SubagentStart`、`SubagentStop` 在执行程序中仅作为有界 no-op 适配缝，真实运行验收前不配置。

节点会话与仓库插件包都只配置这三个事件。插件 Hook 的账本位于 `PLUGIN_DATA`（没有时退回系统临时目录），只保留最多 48 个规范相对路径及 read/write 类型；SessionEnd 删除，异常残留 24 小时后清理。Hook 配置不等于执行：Codex 首次或定义变化后仍需用户在 `/hooks` 审核；项目不写信任记录，也不使用 trust bypass。

## 真实 token 与时间观测

静态 capability manifest 定义 `elon.project_context_runtime_observation.v1`，只接受 Codex app-server 的以下通知类型：

- `hook/started`、`hook/completed`；
- `thread/tokenUsage/updated`；
- `turn/started`、`turn/completed`；
- `item/started`、`item/completed`，只保留 item 类型和工具名以聚合原生文件读取次数。

`scripts/project-memory-app-server-observer.mjs` 已提供 stdin 事件接入器：它在进入 loopback API 前先缩减为方法名、token 累计计数或 item 类型/工具名；服务端再次白名单校验，并只在工作区外 SQLite 保存不可逆 session 指纹、baseline/enabled 窗口、事件计数、input/cached-input/output token 计数、返回元数据字节、选择记忆数和耗时。原始事件对象处理后立即丢弃；prompt、聊天、transcript、assistant message、tool input/output、源码正文和命令文本不落库。24 小时未完成的窗口标为 abandoned；非活动窗口只保留最近 2000 条，避免观测索引无限增长。

当前状态是 `ingest_adapter_available`，不是“已经产生效果数据”。只有同一个 `benchmark_key` 同时取得 `baseline_without_project_memory` 与 `with_project_memory` 两个完整窗口后，接口才返回输入 token、耗时和原生文件读取次数的差值；否则只报告无测量或部分测量，不能宣称真实节省，也不能冒充供应商账单或完整任务 token。

## 审核质量反馈

拒绝候选时必须从 duplicate、task_local、unsupported、conflict、stale、not_reusable 中选择原因；原因与人工决定时间只保存在本机候选索引。候选列表同时聚合最近最多 200 条的 producer 状态计数，帮助发现某种回执来源是否经常生成重复或任务局部内容。该统计只描述本机审核结果，不自动封禁 producer，也不能改变信源优先级。

## Codex 官方 Memories 边界

Codex 官方 Memories 是供应商自己的本机生成状态，与项目 Git 记忆分离。本项目：

- 不读取、导入、写入、删除或备份 Codex 私有 Memories；
- 不假设存在稳定的通用第三方 memory import/export 接口；
- 团队规则继续放在 `AGENTS.md` 和受控项目文档；
- 只把代理显式提交、证据绑定且经过人工审核的短导航事实写入 Git 共享记忆。

因此“换 PC 后免除全部理解成本”不是承诺。可实现的是：新代理先获得小而可验证的导航层，再按任务只打开少量当前文件；这既保留供应商原生搜索优势，也避免把旧私有状态复制成冲突信源。

## 仍需真实验收的项目

以下工作依赖实际运行，不属于代码静态落位阶段：

- Cargo/npm 编译和单元测试；
- Codex Desktop/CLI 对 context/receipt profile 的真实 schema 与 token 对比；
- 仓库内 Codex plugin 的安装、相对脚本解析、MCP 启动与卸载；
- Hook 信任、触发、SessionEnd 清理以及 compact/subagent 生命周期行为；
- app-server 事件流接线和匹配 baseline 测量；
- 跨 PC clone/apply 后的 Git 对象、换行与重定位验证；
- PC 浏览器中的分页、修复计划和旧节点降级体验。
