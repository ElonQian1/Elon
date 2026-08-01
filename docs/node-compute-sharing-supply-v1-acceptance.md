# 节点模型算力显式共享供给 V1 验收

验收日期：2026-08-01

## 已验证闭环

- 新节点默认不向其他用户共享模型，且开启时必须选择至少一个模型。
- 只有节点所有者可以查看和修改自己的完整供给策略。
- 指定节点与自动调度都经过共享策略检查；自动调度在原子占位失败后尝试下一候选。
- 对外任务按允许模型、最大并发和每日 Token 预算限制；预算同时覆盖今日实耗、有效活动预留和本次请求预留，所有者自用不消耗共享额度。
- 同一调用编号可安全重试，但改变用户、节点、模型或用途会被拒绝。
- `/api/nodes/models` 不再把所有在线节点模型默认暴露为公共供给。
- PC 节点详情可设置开关、模型范围、并发和每日阈值；节点市场只展示明确开放的服务。
- 节点执行、用量、失败和提供者收益继续进入现有链外账本。
- 长时间流式推理通过 30 秒心跳续期 2 分钟执行租约，并发名额以最近心跳而非开始时间判断。
- 流提前断开、请求编号不匹配或服务重启不会被当成成功结果结算；遗留预授权会进入释放流程。
- 每个共享推理在原子准入时保存保守 Token 预留；今日实耗、活动预留与本次预留之和不得超过每日预算。
- 同一调用编号不能改变预留预算；终态只按实际 Token 计量，活动预留随任务结束、失败或租约过期释放。
- 过期推理的迟到心跳不能重新占用并发和每日预算。
- 所有者接口会按终态时间从持久化记录派生近 24 小时终态、失败、Token 预留超出和当前过期租约；所有者自用不会被误计为共享风险。
- 健康快照只向节点所有者展示，使用稳定告警码，不进入公开节点目录，也不自动执行经济处罚。
- 收到匹配终态事件后，实际 Token 会先冻结为 `usage_received`，避免异步记账期间被执行租约清理器误判为失联。
- 后台会周期性原子终结真正过期的 `started` 推理，并以同一调用编号幂等释放预授权；重复扫描不重复退款。
- 节点执行的最终结算、失败或释放状态不可被迟到结果改写，同时保留 PC CLI 的 `verification_pending` 合法恢复路径。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain node-compute-sharing -- test `
  --manifest-path server\Cargo.toml --bin elon-server node_compute_sharing

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain node-compute-runs -- test `
  --manifest-path server\Cargo.toml --bin elon-server node_compute_runs

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain node-compute-sharing-health -- test `
  --manifest-path server\Cargo.toml --bin elon-server node_compute_sharing_health

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain node-llm-stream -- test `
  --manifest-path server\Cargo.toml --bin elon-server node_llm_stream

Set-Location pc-frontend
npm run test:node-compute-sharing
npm run build
```

## 尚未覆盖

- 不同模型的消息模板可能使估算与实际 Token 存在偏差；当前已向所有者展示偏差次数和 Token 数，但尚未建立按模型的时间序列、自动调参或 SLA 裁决。
- 通用 CPU/GPU 批处理、训练、图片或视频生成任务的异构报价与调度。
- 节点质押、服务等级协议、自动故障赔付、真实提现和 Sui 网络结算。

因此，本批只能描述为“节点模型推理供给授权与受限调度 V1 已实现”，不能描述为“公开算力交易市场已经完成”。
