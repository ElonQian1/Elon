# Codex 保险箱授权共享验证 Runbook

最后更新：2026-07-06

本文用于验证机器人之间的 Codex `auth.json` 保险箱授权共享。共享不限定紧急场景，但必须满足显式授权、短租约、本机托管 `CODEX_HOME`、可撤销、可审计和可计费。

## 安全边界

- 不读取、不打印、不让用户粘贴 `auth.json` 明文。
- 不覆盖默认 `~/.codex/auth.json`。
- consumer 节点必须用本机节点 secret 证明身份。
- provider 必须在平台上显式授权 consumer。
- 共享凭据只写入节点托管的临时 `CODEX_HOME`。
- 所有共享用量记为 `shared_codex`，并能追溯 provider、consumer、lease、token usage、billing event 和 node transaction。

## 一键真实诊断

在 consumer 机器人 PC 上运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-codex-vault-sharing-switch.ps1 `
  -ProviderUserId "<provider_user_id>"
```

脚本会自动执行：

1. 创建临时坏 `CODEX_HOME/auth.json`。
2. 清除 Codex 子进程里的 `OPENAI_API_KEY` / `CODEX_API_KEY` 等旁路变量。
3. 运行真实 `codex exec --ignore-user-config --ignore-rules --ephemeral`，坏凭据必须失败。
4. 通过本机节点申请 provider 的授权共享租约。
5. 用共享 `CODEX_HOME` 运行同一条 Codex 命令，共享凭据必须成功。
6. 调用本机 `/api/codex-vault/clear` 清理租约。
7. 输出脱敏 JSON 结果。

通过条件：

- `bad_probe.success=false`
- `shared_probe.success=true`
- `shared_home_under_managed_vault=true`
- `final_active_home=null`
- `final_default_auth_present=true`

## 无额度 fake 验证

独立脚本不调用真实 Codex，不消耗额度，只启动 fake 子进程验证 `CODEX_HOME` 传递：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-codex-vault-fake-cli-env.ps1
```

通过条件：

- 有效托管共享目录会传给 fake 子进程。
- 过期托管共享目录会回落默认 `~/.codex`。
- 子进程没有收到 `OPENAI_API_KEY` / `CODEX_API_KEY` 旁路变量。

Rust 单元测试也覆盖节点启动 Codex 前的 `CODEX_HOME` 选择逻辑：

- 有效托管共享目录优先传给 Codex 子进程。
- 过期托管共享目录会被忽略并回落默认 `~/.codex`。
- fake 子进程收到的 `CODEX_HOME` 来自节点真实使用的 env assignment。

推荐命令：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml codex_child_home
```

## 回归测试矩阵

| 场景 | 预期 |
|---|---|
| 撤销授权后申请共享租约 | 失败，不能下发 provider auth |
| 授权过期后申请共享租约 | 失败，不能下发 provider auth |
| 租约过期后启动 Codex | 不再使用托管共享 `CODEX_HOME` |
| 清理租约后继续挂账 | store 层拒绝继续 attach 用量 |
| 两个机器人互相共享 | 每条租约独立记录 provider/consumer，账单不能串 |
| 共享用量成功结算 | lease 关联 token usage、billing event、node transaction |
| 共享用量缺少结算链 | sharing health 返回 `shared_codex_accounting_anomaly` |
| provider 没有可用保险箱 | sharing health 返回 `shared_provider_vault_unavailable` |
| 失败事件 | sharing health 返回 `recent_sharing_failure` |

## 生产监控

PC 页面和云端 `/api/me/codex-vault/sharing` 返回 `health`：

- `status=ok`：没有共享告警。
- `status=warning`：存在过期未清理、provider 缺少保险箱或最近失败。
- `status=critical`：存在 `shared_codex` 用量但缺少完整计费/结算链路。

运维应重点关注：

- `expired_uncleared_count`
- `accounting_anomaly_count`
- `unavailable_grant_count`
- `recent_failed_event_count`

## 医疗服务演练

1. 确认两台机器人都在线，且各自保险箱已备份。
2. 在 PC 页面建立双向授权共享。
3. 在 consumer 机器人运行一键真实诊断脚本。
4. 发起一次真实平台任务，确认 `billing_source=shared_codex`。
5. 查账：consumer 扣费，provider 获得收益。
6. 清理租约，确认本机 `active_codex_home=null`。
7. 模拟 provider 撤销授权，确认 consumer 不能再申请共享租约。
8. 模拟租约过期或节点重启，确认不会继续使用过期共享凭据。
9. 模拟网络断开，确认不会误判共享成功，也不会重复扣费。
10. 演练结束后检查 sharing health 必须回到 `ok`。

## 故障处理

- 坏 auth 仍然成功：检查是否有 API key、默认 auth、旧 session 或错误 Codex binary 绕过 `CODEX_HOME`。
- 共享 auth 失败：检查 provider vault 是否可用、授权是否 active、consumer 节点 secret 是否匹配。
- 清理失败：先在 PC 页面点“清理本机”，再查云端是否还有 active 未清理租约。
- 计费缺链：查 `token_usage_events`、`billing_events`、`node_transactions` 和对应 lease 的三个外键。
