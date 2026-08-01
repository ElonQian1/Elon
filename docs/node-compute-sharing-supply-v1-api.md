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
    "available": false,
    "availability": "sharing_disabled"
  },
  "observed_models": []
}
```

`observed_models` 来自当前在线节点上报，只用于辅助选择。策略中的模型白名单是所有者保存的授权事实；节点仍必须在线并实际上报目标模型才可调度。

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
- 每日阈值按已完成的对外推理 Token 判断，不包含所有者自用；在途任务可能使最终值略高于阈值。

## 发现与调用

- `GET /api/me/nodes`：返回本人节点及完整 `compute_sharing` 状态。
- `GET /api/nodes`：返回在线节点及供给状态，供节点市场展示。
- `GET /api/nodes/models`：登录用户可看到自己的在线模型与当前可用共享模型；未登录调用者只能看到当前可用共享模型。
- `POST /api/nodes/chat`：指定节点或自动选点都执行供给检查和原子占位；自己的节点无需开启共享。

推理开始后，服务端每 30 秒更新一次执行租约。活动租约为 2 分钟；只有收到匹配请求编号的 `LlmStreamEnd` 才会按节点返回的 Token 用量进入结算。节点报错、流提前断开或服务重启都会将任务标记为失败并释放尚未结算的预授权。

## Availability

| 值 | 含义 |
|---|---|
| `owner_self_use` | 调用者是节点所有者，可自用 |
| `available` | 当前允许外部调度 |
| `sharing_disabled` | 所有者未开启共享 |
| `no_allowed_models` | 未配置模型白名单 |
| `model_not_allowed` | 目标模型不在白名单 |
| `concurrency_limit_reached` | 对外活动任务已达到并发上限 |
| `daily_token_limit_reached` | 今日已完成 Token 达到阈值 |
| `owner_mismatch` | 策略所有者与节点所有者不一致，失败关闭 |

调用接口在没有可用候选时返回服务不可用错误；策略接口分别使用 `401`、`403`、`404`、`400` 或 `500` 表达认证、归属、节点、输入和存储错误。
