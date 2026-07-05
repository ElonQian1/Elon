# Codex 共享账号用量百分比估算

OpenAI Codex 官方只返回当前窗口的 `usedPercent` / `remainingPercent`、重置时间和 token activity 摘要，不公开“1% 等于多少 token”的固定换算。因此平台不能把 1% 写死成某个 token 数；只能在同一个官方窗口内做观测估算。

## 计量原则

1. **官方百分比是账号容量信号**：来自 Codex `app-server` 的 `account/rateLimits/read`。
2. **平台 token 是机器人归因信号**：来自平台已记录的 `shared_codex` token usage、租约、billing event 和节点收益流水。
3. **按窗口结算**：以 `limit_id + resets_at` 分组；不同 5 小时窗口不能混算。
4. **优先用官方 lifetime token 校准**：同一窗口前后快照如果有 `lifetimeTokens` 增量，用它估算“百分比 / token”。
5. **没有 lifetime token 增量时退化为比例分摊**：把窗口百分比消耗按已知 `shared_codex` token 占比分给 A/B/C，并标记低置信度。
6. **月费只做摊销估算**：$200 Pro 是包月权益，不是按 token 售卖。平台按窗口时长摊销月费，例如 300 分钟窗口约等于 `20000 cents * 300 / (30*24*60)`。

## 数据流

1. 机器人 A 借用全嘉 auth 前后，节点采集官方快照。
2. 机器人 B、C 随机加入时，也在开始/结束或定时采集官方快照。
3. 各节点把快照 POST 到本机节点：

```text
POST /api/codex-vault/sharing/usage-snapshots
```

本机节点会用自己的云端登录态转发到：

```text
POST /api/me/codex-vault/sharing/usage-snapshots
```

4. 服务器把快照写入 `codex_vault_usage_snapshots`，按 `provider_user_id` 集中到全嘉账号。
5. 查询估算报告：

```text
GET /api/me/codex-vault/sharing/usage-estimate?provider_user_id=<全嘉用户ID>&limit_id=codex&days=7&monthly_usd_cents=20000
```

## 分摊公式

对同一窗口：

```text
窗口百分比消耗 = after.usedPercent - before.usedPercent
官方 token 增量 = after.lifetimeTokens - before.lifetimeTokens
```

如果官方 token 增量有效：

```text
机器人百分比 = 窗口百分比消耗 * 机器人 shared_codex tokens / max(官方 token 增量, 窗口 shared_codex tokens)
```

如果没有官方 token 增量：

```text
机器人百分比 = 窗口百分比消耗 * 机器人 shared_codex tokens / 窗口 shared_codex tokens 总和
```

报告会给出 `confidence`：

| 值 | 含义 |
|---|---|
| `official_lifetime_calibrated` | 有官方 lifetime token 增量，可信度最高 |
| `official_delta_below_shared_tokens` | 平台 token 大于官方增量，说明两边 token 口径可能不同 |
| `shared_token_proportional` | 无官方 token 增量，只能在已知共享用量内分摊 |
| `insufficient_token_data` | 快照或 token 不足，不能估算 |

## 为什么这比只看 token 更合理

A 在 100% 剩余时开始、B 在 70% 左右加入、C 更晚加入时，三者的 token 不在同一时间段。按窗口快照切片后，A/B/C 只分摊自己实际发生 token 的窗口百分比，不会把 B 加入前 A 消耗的百分比算给 B，也不会把 C 来之前的消耗算给 C。

## 不能保证的事

- 不能把官方百分比精确反推成绝对 token 价格。
- 如果全嘉本人同时在别处使用同一个 Codex 账号，官方百分比会包含外部用量；报告会把这部分作为 `unattributed_percent`，不能强行摊给医疗机器人。
- OpenAI 可能有额外 weekly limits 或模型级限流，报告只按可观测字段估算。
