---
title: 任务级分布式算力联邦 V1
status: accepted
date: 2026-08-04
owners: backend, node, ai-economy
implementation_status: foundation_in_progress_uncompiled
---

# 任务级分布式算力联邦 V1

## 背景

项目已经能让节点显式共享本地模型、限制并发和每日 Token、维持执行租约并记录提供者收益，但当前路径仍是一条“一个请求选择一个在线节点”的专用 LLM 通道。它不能统一表达异构任务、外部矿池、工件版本、重试尝试、多源验证和未来容量价格。

产品方向要求把一龙建设成 AI 算力矿池：用户节点、平台集群和不同公司的算力池共同为用户的 AI 任务提供能力，并从单次推理共享演进到算力供给、价格发现和容量期货市场。

## 决定

### 1. 采用任务级联邦

公网异构节点以完整任务或独立可重试分片为单位聚合。V1 不把普通 WAN 节点伪装成一张虚拟 GPU。需要低延迟多 GPU 并行的工作负载由受管集群内部处理，集群作为一个逻辑 Provider 接入。

### 2. 统一三类 Provider

核心调度统一支持 `user_node`、`managed_cluster` 和 `external_pool`。外部公司或矿池通过服务端 Adapter 接入；普通客户端不安装每个外部 Provider 的 SDK。

### 3. 采用五平面架构

系统拆为控制面、数据面、工件面、验证与计量面、市场与结算面。大模型与大输入不通过控制 WebSocket 内联搬运，只传内容摘要和短期工件引用。

### 4. 核心合同版本化且不可变引用

建立 `ComputeProvider`、`ComputeOffer`、`WorkloadSpec`、`ComputeJob`、`ComputeReservation`、`ComputeAttemptLease`、`ExecutionReceipt`、`PriceSnapshot` 和 `SettlementReceipt`。Job 必须引用确定的 Offer 与 Price Snapshot 版本；历史版本不原地修改。

### 5. Job 与 Attempt 分离

一个 Job 可有多个 Attempt。每次尝试持有递增 fencing token；过期尝试不能被心跳复活，迟到结果不能覆盖新尝试。Reservation 同时保护 Provider 容量与消费者预算，并在终态幂等释放。

### 6. 节点采用按需插件

客户端默认只带轻量 Compute Bootstrap。用户开启共享后才下载 Node Core、Plugin Host、所需 Runtime Plugin 和模型工件；关闭共享时不得后台安装重型算力组件。模型和插件分开版本化、校验、缓存和卸载。

### 7. 计量事实分层

Provider 声明用量、平台观测用量和验证后用量分别保存。结算只使用验证策略接受的用量；节点自报数据不是唯一真相。

### 8. 使用标准 SKU 与期货价格快照

市场以任务类型、模型/Tokenizer 摘要、运行时、精度、形状/上下文档、信任等级、区域和交付窗口组成 Compute SKU。首期在 Reservation 前锁定 `capacity_future` 价格快照，按整数微单位结算；禁止用一个通用 TFLOPS 价格或浮点金额覆盖所有任务。

### 9. 消费者与 Provider 两条价格腿

Price Snapshot 同时冻结消费者收费和 Provider 应得规则，平台价差显式计算。Provider 收益先进入待验证状态，不能在节点上报结束事件后立即成为可提取余额。

### 10. 保留旧协议兼容

现有 `LlmStreamRequest`、节点模型白名单、Token 预留、流租约和账本继续工作。第一阶段只增加领域合同与 NodeAgent 内部 Host seam，不提升协议版本、不让旧节点强制升级。正式通用 Compute 协议必须通过 capability gate 灰度启用。

## 被否决的首期方案

- 默认把 CUDA、DirectML、各种矿池 SDK 和模型全部打包给普通用户；
- 直接把不稳定公网 GPU 聚合成同步张量并行集群；
- 把节点自报 Token、GPU 时间或成功状态直接用于结算；
- 用可变“当前价格”重算历史任务；
- 用 `f32/f64` 保存金额、价格或份额；
- 在 Provider Adapter 尚未形成统一合同前，让调度器直接依赖各厂商字段。

## 与既有决定的关系

`docs/decisions/node-compute-sharing-supply-v1.md`、Token 预算预留、运行健康和过期执行回收继续有效，它们是 `user_node + llm_chat` 的已实现兼容路径。本决定扩展其边界，不覆盖或伪造其实现状态。

现有影子结算可作为追加式 Receipt 和纠正思路的参考，但新的算力市场不能把未验证的节点终态直接沿用为最终经济事实。

## 后果

- 系统获得跨用户节点、集群和外部矿池的统一语言；
- 客户端安装体积与共享意愿解耦；
- 调度、验证、市场和结算可以独立演进；
- 首期会增加合同和状态数量，要求所有代理明确区分 Offer、Job、Attempt 与 Receipt；
- 未来 tensor parallel 仍可存在，但位于 Managed Cluster Provider 内部，而不是污染公网 Broker。

## 落地顺序

1. 文档、知识图与核心 Rust 领域合同；
2. 旧 LLM 路径接入 NodeAgent Plugin Host seam；
3. Offer Registry、Broker、Reservation 和 Attempt Lease 存储；
4. 通用节点协议、按需安装器和首批 Runtime Plugin；
5. 多源验证、Price Snapshot 与待确认收益；
6. 外部 Provider Adapter；
7. 容量承诺、订单、持仓与清算。

## 验证状态

本决定已接受。2026-08-04 按产品负责人的“先铺架构、后统一检查”指令推进：首批代码会提交但不编译、不运行、不迁移，因此在后续集成门禁通过前只能称为“基础合同已写入，尚未验证”。
