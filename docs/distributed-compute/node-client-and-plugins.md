---
title: 分布式算力节点客户端与按需插件
status: current
reviewed_at: 2026-08-04
owners: node, pc
---

# 分布式算力节点客户端与按需插件

## 1. 产品原则

普通用户安装一龙客户端时，只携带轻量 `Compute Bootstrap`。共享算力默认关闭；关闭时不启动算力守护进程、不下载 CUDA/DirectML/模型运行时，也不保存与算力市场无关的重型组件。

用户主动开启“贡献闲置算力”后，客户端才探测硬件、展示预计任务和收益、生成安装计划，并在用户确认后下载所需 Node Core、插件、运行时和模型工件。

## 2. 组件分层

| 层 | 默认携带 | 职责 |
|---|---|---|
| Compute Bootstrap | 是 | 开关、硬件探测、磁盘预算、签名下载器、恢复与卸载 |
| Node Core | 否 | 与控制面维持会话、策略同步、任务队列、租约、审计事件 |
| Plugin Host | 否 | 统一启动/停止插件，限制资源并转换通用任务合同 |
| Runtime Plugin | 否 | Ollama、LM Studio、llama.cpp、CUDA、DirectML 等适配 |
| Model Artifact | 否 | 内容寻址的模型、Tokenizer、LoRA 和辅助工件 |

现有 Windows PC 节点可以复用身份、升级器和 WebSocket 会话，但“远程开发节点”与“贡献算力节点”仍是两个独立权限。

## 3. 开启流程

1. 用户打开贡献开关；Bootstrap 只读探测 CPU、GPU、显存、内存、磁盘、驱动和现有本地模型服务。
2. 控制面根据硬件、用户允许的任务种类和磁盘预算返回带修订号的 `PluginPlan`。
3. 客户端展示下载体积、预期磁盘、运行时权限、模型工件和资源上限。
4. 用户确认后，Bootstrap 分块下载并校验清单签名、文件摘要与兼容约束。
5. Plugin Host 在本机完成健康探测和预热，只上报真实 `ready` 的 Offer。
6. 控制面接受 Offer 后，节点才进入可调度集合。

“开关打开”不等于“立刻可调度”；只有插件和工件验证完成、策略允许且健康检查通过才发布 Offer。

## 4. 关闭与卸载

关闭共享时先进入 `draining`，停止领取新任务，允许在资源上限内完成或检查点化已有任务；到达截止时间后取消剩余 Attempt、释放 Reservation 并停止 Node Core。

用户可以分别选择：

- 仅暂停：保留插件和模型，后续快速恢复；
- 停止并清缓存：删除可重下的模型和运行时缓存；
- 完全移除：删除 Node Core、插件、模型和算力专用状态，但保留必要的结算回执与用户账户记录。

## 5. 插件生命周期

```text
absent -> planned -> downloading -> verified -> installed -> warming -> ready
ready -> draining -> disabled -> removed
任何阶段 -> quarantined
```

- `verified` 只表示签名与摘要正确，不表示运行时可用；
- `ready` 必须绑定插件版本、运行时摘要、模型摘要和硬件档案；
- `quarantined` 禁止发布 Offer，只有显式升级或复核后才能恢复；
- 回滚保留前一个已验证版本，不能原地覆盖正在执行任务使用的插件。

## 6. 插件清单

每个插件清单至少包含：

- `plugin_id`、`schema_version`、插件版本与包摘要；
- 支持的 `task_kinds` 和 Host API 版本区间；
- 操作系统、架构、GPU 后端、驱动和运行时约束；
- 入口进程、健康检查、最大内存/显存/CPU/磁盘和网络策略；
- 所需模型/Tokenizer/辅助工件摘要；
- 升级、回滚和数据清理声明。

插件能力必须由实际健康检查生成，禁止根据“已安装”推断为“可执行”。

## 7. 第一批插件路线

1. `bridge.ollama.v1`：复用用户已经运行的 Ollama；
2. `bridge.lmstudio.v1`：复用 LM Studio OpenAI 兼容端点；
3. `runner.llama-cpp.cpu.v1`：通用 CPU 低门槛供给；
4. `runner.llama-cpp.cuda12.v1`：NVIDIA GPU；
5. `runner.llama-cpp.directml.v1`：Windows 多品牌 GPU。

首批代码已经把现有 `LlmStreamRequest` 经 NodeAgent 内部 Host seam 委托给旧本地推理实现，输出仍由原实现产生 Chunk/End/Error。内部登记使用稳定 `runner_id` 和本机 revision，不序列化成 Plugin Manifest，也不发布新 capability。该代码尚未编译；此时只能称为“兼容路径接入 Host seam”，不能宣称通用插件下载与协议已完成。

## 8. Host 与 Sidecar 边界

插件默认以独立 Sidecar 进程运行，通过 Windows Named Pipe 或等价本机 IPC 与 Plugin Host 通信，不暴露公网监听端口。Host 负责：

- 验证任务合同和工件摘要；
- 施加 CPU、内存、显存、磁盘、时长和并发限制；
- 转发取消、心跳、进度、检查点和终态；
- 在进程退出或租约过期时生成失败事实；
- 只允许最新 fencing token 提交候选终态。

第一批内部兼容 seam 可以直接调用现有 Rust 本地推理函数；正式 Sidecar 协议在安装器和能力上报准备好后单独版本化。

## 9. 模型与缓存

模型采用内容寻址标识，至少绑定 SHA-256、文件长度、格式、Tokenizer 摘要、许可元数据和来源。下载支持断点续传、分块校验、镜像切换和去重。

缓存分为 pinned、active、warm 和 evictable。正在运行或已承诺交付窗口的工件不能被回收；其余根据用户磁盘配额和最近使用顺序回收。插件升级不能隐式下载任意模型，模型计划必须单独可见。

## 10. 策略与资源保护

控制面下发带 `policy_revision` 的期望策略，节点返回 `applied_revision` 和本机有效策略。每次任务开始前仍需本地重新检查：

- 用户开关、允许任务和允许模型；
- 使用时段、最大功耗、温度、CPU/GPU/内存占用；
- 最大并发、每日/周期容量和网络上行；
- 前台交互优先级、游戏/会议/电池状态；
- 插件、运行时与模型的实际健康状态。

本地策略比云端期望更严格时，以本地为准。节点拒绝任务必须返回稳定原因码，使 Broker 能立即改派。

## 11. 任务与数据

V1 只运行平台批准的任务种类和签名 Runner，不接受请求方上传任意可执行代码。任务输入使用摘要和短期引用；日志默认不复制完整 Prompt 或模型输出。技术信任通过验证策略逐步提高，而不是把节点自报结果直接当成结算事实。

## 12. 当前未验证声明

本文是已接受的客户端目标设计。NodeAgent 内部 legacy Host seam 已形成代码但尚未编译或启动；插件安装器、Sidecar IPC、typed plugin events、通用能力上报和 Offer 发布仍未实现。
