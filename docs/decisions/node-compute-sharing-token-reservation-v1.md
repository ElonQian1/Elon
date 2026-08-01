---
title: "共享节点每日 Token 预算预留 V1"
status: accepted
reviewed_at: 2026-08-02
implementation_refs:
  - "file:server/src/node_compute_reservation_migration.rs"
  - "file:server/src/store/node_compute_sharing.rs"
  - "file:server/src/node_compute_sharing.rs"
  - "file:server/src/node_router.rs"
---

# 共享节点每日 Token 预算预留 V1

## 决定

共享节点对外推理不再只按已完成 Token 判断每日上限。服务端在派发前根据消息的 UTF-8 序列化大小、固定结构余量和最大输出 Token 计算保守预算，并在取得并发名额的同一事务中持久化预留。

原子准入必须满足：

```text
今日已产生的实际 Token
  + 当前有效租约的预留 Token
  + 本次请求预留 Token
  <= 节点每日共享 Token 预算
```

终态仍以节点结束事件返回的实际 Token 为计量事实。任务结束、失败、服务重启或租约过期后，其预留不再占用预算；过期租约不能被迟到心跳重新激活。同一调用编号重放必须保持相同预留预算。节点所有者自用不受共享预算影响。

## 边界

- 预留值用于供给准入，不是价格、扣费、收益或链上凭证。
- 最大输出统一归一到 1 至 1,000,000 Token；未传时采用 1,024。
- 实际用量与预留值同时保留在执行记录中，便于识别节点未遵守输出上限或模型模板异常造成的估算偏差。
- 本决定消除并发任务共同穿透剩余额度的竞态，不承诺异构模型的估算绝对等于最终 Token。
- 本期不加入竞价、质押、自动赔付、提现或 Sui 网络结算。

## 失败关闭

- 本次预留大于剩余预算：拒绝派发；
- 同一调用编号改变预留：拒绝重放；
- 预算字段非法或数据库事务失败：拒绝派发；
- 租约已过期：心跳返回失败，不恢复并发或预算占用。
