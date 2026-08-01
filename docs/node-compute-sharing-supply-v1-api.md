# 节点模型算力共享 V1 API

## 身份与范围

策略接口要求用户登录且只能操作自己的节点。公开节点和模型目录只暴露供给状态，不提供策略修改能力。远程开发共享与模型算力共享是两个独立权限。

## 读取策略

```http
GET /api/me/nodes/{node_id}/compute-sharing
Authorization: Bearer <user-token>
```

响应：

```json
{
  "ok": true,
  "compute_sharing": {
    "policy": {
      "node_id": "node-123",
      "owner_user_id": "user-123",
      "enabled": false,
      "allowed_model_ids": [],
      "max_concurrent_runs": 1,
      "daily_token_limit": 0
    },
    "active_runs": 0,
    "tokens_used_today": 0,
    "tokens_reserved_today": 0,
    "available": false,
    "availability": "sharing_disabled"
  },
  "runtime_health": {
    "node_id": "node-123",
    "status": "healthy",
    "completed_runs_24h": 0,
    "failed_runs_24h": 0,
    "budget_overrun_runs_24h": 0,
    "budget_overrun_tokens_24h": 0,
    "expired_active_runs": 0,
    "attention_codes": [],
    "evaluated_at": "2026-08-02T03:00:00Z"
  },
  "observed_models": []
}
```

`observed_models` 来自当前在线节点上报，只用于辅助选择。策略中的模型白名单是所有者保存的授权事实；节点仍必须在线并实际上报目标模型才可调度。

`runtime_health` 只在本人节点策略接口返回。它从持久化的对外共享推理记录派生近 24 小时终态、失败、实际 Token 超出准入预留，以及当前过期活动租约。`critical` 表示存在预留超出或过期租约，`warning` 表示近期存在失败，`healthy` 表示当前未命中这些规则。该快照不进入公开节点目录，也不会自动触发扣款、赔付或链上操作。

## 更新策略

```http
PATCH /api/me/nodes/{node_id}/compute-sharing
Authorization: Bearer <user-token>
Content-Type: application/json

{
  "enabled": true,
  "allowed_model_ids": ["qwen2.5:7b"],
  "max_concurrent_runs": 2,
  "daily_token_limit": 100000
}
```

字段均可省略，省略时保留当前值。约束：

- 开启共享时至少选择一个模型；
- 单节点最多保存 64 个模型标识；
- `max_concurrent_runs` 为 1 至 16；
- `daily_token_limit` 为 0 至 1000000000000，0 表示不限；
- 每日预算不包含所有者自用；共享任务派发前原子检查今日实际 Token、活动租约预留和本次请求预留。
- 输入预算按消息 UTF-8 序列化大小和结构余量保守估算，输出预算按归一后的 `max_tokens` 计算；未传输出上限时使用 1024。

## 发现与调用

- `GET /api/me/nodes`：返回本人节点及完整 `compute_sharing` 状态。
- `GET /api/nodes`：返回在线节点及供给状态，供节点市场展示。
- `GET /api/nodes/models`：登录用户可看到自己的在线模型与当前可用共享模型；未登录调用者只能看到当前可用共享模型。
- `POST /api/nodes/chat`：指定节点或自动选点都执行供给检查和原子占位；自己的节点无需开启共享。

推理开始后，服务端每 30 秒更新一次执行租约。活动租约为 2 分钟；只有仍在租期内的任务可以续期，过期任务不能被迟到心跳复活。只有收到匹配请求编号的 `LlmStreamEnd` 才会冻结节点返回的实际 Token，并从 `started` 进入 `usage_received` 等待异步记账；该状态不再占执行并发，但实际 Token 继续占每日预算。

后台默认每 30 秒扫描过期执行。仍为 `started` 且租约过期的运行会原子标记为 `failed/expired_released`，并按调用编号幂等释放预授权。节点报错、流提前断开或服务重启同样失败关闭；服务重启还会关闭尚未完成记账的 `usage_received`。结算、失败或释放终态不可被迟到结果覆盖。扫描周期可通过 `NODE_LLM_LEASE_RECONCILE_SECS` 调整，最小为 5 秒。

## Availability

| 值 | 含义 |
|---|---|
| `owner_self_use` | 调用者是节点所有者，可自用 |
| `available` | 当前允许外部调度 |
| `sharing_disabled` | 所有者未开启共享 |
| `no_allowed_models` | 未配置模型白名单 |
| `model_not_allowed` | 目标模型不在白名单 |
| `concurrency_limit_reached` | 对外活动任务已达到并发上限 |
| `daily_token_limit_reached` | 今日实耗与有效活动预留之和达到预算 |
| `owner_mismatch` | 策略所有者与节点所有者不一致，失败关闭 |

调用接口在没有可用候选时返回服务不可用错误；策略接口分别使用 `401`、`403`、`404`、`400` 或 `500` 表达认证、归属、节点、输入和存储错误。

若节点仍显示可用，但本次保守预留大于剩余预算，准入返回 `daily_token_reservation_exceeds_limit`。该值是本次调用的失败原因，不会覆盖节点长期策略状态。
