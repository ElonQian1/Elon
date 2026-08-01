# 节点模型算力显式共享供给 V1 验收

验收日期：2026-08-01

## 已验证闭环

- 新节点默认不向其他用户共享模型，且开启时必须选择至少一个模型。
- 只有节点所有者可以查看和修改自己的完整供给策略。
- 指定节点与自动调度都经过共享策略检查；自动调度在原子占位失败后尝试下一候选。
- 对外任务按允许模型、最大并发和每日已完成 Token 阈值限制；所有者自用不消耗共享并发或阈值。
- 同一调用编号可安全重试，但改变用户、节点、模型或用途会被拒绝。
- `/api/nodes/models` 不再把所有在线节点模型默认暴露为公共供给。
- PC 节点详情可设置开关、模型范围、并发和每日阈值；节点市场只展示明确开放的服务。
- 节点执行、用量、失败和提供者收益继续进入现有链外账本。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain node-compute-sharing -- test `
  --manifest-path server\Cargo.toml --bin elon-server node_compute_sharing

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain node-compute-runs -- test `
  --manifest-path server\Cargo.toml --bin elon-server node_compute_runs

Set-Location pc-frontend
npm run test:node-compute-sharing
npm run build
```

## 尚未覆盖

- 超过 15 分钟任务的租约心跳与主动续期。
- 为未完成任务预留输入和输出 Token，因此每日阈值可能被最后一批并发任务少量超过。
- 通用 CPU/GPU 批处理、训练、图片或视频生成任务的异构报价与调度。
- 节点质押、服务等级协议、自动故障赔付、真实提现和 Sui 网络结算。

因此，本批只能描述为“节点模型推理供给授权与受限调度 V1 已实现”，不能描述为“公开算力交易市场已经完成”。
