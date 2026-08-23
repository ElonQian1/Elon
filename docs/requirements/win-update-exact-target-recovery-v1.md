---
version_status: current
reviewed_at: 2026-08-23
implementation_status: proposed
---

# Win 精确版本更新状态收敛 V1

## 目标

当 Win 节点已经运行请求中的精确发布身份时，重复的云端广播或 Codex 更新请求必须立即收敛为 `runtime_online`，不得重新等待历史监督任务，也不得重复下载安装或重启。

## 验收标准

1. `schedule_update` 在任务分类和下载前识别当前 `release_identity` 与精确目标完全匹配，并返回 `already_current=true`。
2. 同目标旧检查点转为 `runtime_online`，清空仅属于待重启事务的阻塞、恢复和陈旧任务列表。
3. 不同目标的旧检查点被当前已在线目标显式替代，并保留 `superseded_update_id` 审计关系。
4. 正在排空的后台循环观察到检查点不再是 `draining` 后立即退出，不得把旧任务重新写回已收敛检查点。
5. Rust 定向测试和 Win 节点发布合同验证通过；不修改 PWA、任务正文或用户凭据。

## 非目标

- 不自动取消、恢复或删除历史本机任务。
- 不放宽精确发布身份、签名清单、安装路径、下载或回滚门禁。
- 不改变 ChatGPT / Google AI 页面、富内容协议或会话数据。

## 预计实现范围

- `server/src/node_agent_restart_drain.rs`
