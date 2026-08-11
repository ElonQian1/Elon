# External Pool Onboarding API 验收

## 1. 结论

状态：`partially_verified`。

本验收对应的 v221 onboarding 已形成 owner submit/list/detail/cancel/preflight 与 admin list/detail/preflight/review/application 共 10 个 Service/HTTP 操作，并通过定向真实 Rust 编译、4 项进程内接口测试和 3 项 Store 测试。该结论只证明受控来源登记及其管理面可调用，不证明 Provider 已激活、可路由、可供给容量或可执行任务。

## 2. 已实现接口

- `POST /api/me/compute/external-pool-onboarding-requests`；
- `GET /api/me/compute/external-pool-onboarding-requests?status=...&limit=...`；
- `GET /api/me/compute/external-pool-onboarding-requests/:request_id`；
- `POST /api/me/compute/external-pool-onboarding-requests/:request_id/cancel`；
- `GET /api/me/compute/external-pool-onboarding-requests/:request_id/preflight`；
- `GET /api/admin/compute/external-pool-onboarding-requests?status=...&limit=...`；
- `GET /api/admin/compute/external-pool-onboarding-requests/:request_id`；
- `GET /api/admin/compute/external-pool-onboarding-requests/:request_id/preflight`；
- `POST /api/admin/compute/external-pool-onboarding-requests/:request_id/review`；
- `POST /api/admin/compute/external-pool-onboarding-requests/:request_id/application`。

owner 入口要求登录并按会话 owner 隔离；admin 入口要求 `admin` 或 `owner` 平台角色。Service 从会话派生 owner、reviewer 与 applier，不接受外部 actor 字段。Provider owner、settlement account、`external_pool/registering/self_declared`、revision 1、无 endpoint 及 Adapter ref 均由服务端构造。列表限制收敛到 1 至 100，详情只组合脱敏 request/review/application 回执。

## 3. 已验证行为

- 未登录 submit 返回 `401`，普通用户调用管理员入口返回 `403`；
- 未知 actor 字段被拒绝，owner 不能伪造另一主体；
- owner 即使具有平台管理员角色，也不能复核自己的 request；
- submit/review/application 均要求显式确认；
- Service 对 capability 数组排序去重，并生成 exact request envelope/digest；
- exact 三段重放返回原 ID 和 `replayed=true`；
- 改变历史内容的同幂等键重放被拒绝；
- 错误 review digest 与非 `approved` request 均不能 application；
- 成功 application 在同一事务登记 exact `external_pool/registering/self_declared` Provider；
- owner 回执只返回 credential presence/hint，不返回 non-bearer ref；
- owner 只能列出和读取本人 request，普通用户不能读取他人详情；
- owner 仅能取消 `submitted` request，取消要求显式确认和 exact digest，重复取消返回同一状态且 `replayed=true`；
- owner/admin 列表支持固定状态筛选，非法状态失败关闭；
- preflight 随 `submitted→approved→applied` 或 `canceled` 状态返回可取消、可复核、可应用布尔值和稳定 blocker；
- admin 详情 exact 组合 review/application，列表和详情均不泄露 non-bearer credential ref；
- canceled 状态、取消时间及 owner/admin 查询在关闭并重开 Store 后保持一致；
- 最终效果固定为 `provider_registered_only`，不写 v213 route。

## 4. 验证命令与证据

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-external-pool-onboarding-management -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_onboarding -- --nocapture
```

- 结果：`CARGO_OK`；
- 测试：7 项通过；
- validation fingerprint：`6badc408ce9d915be01dace62bc29a468b074bcc7d85c3cd7c821cd0445671f1`；
- validation receipt：`ab387a054601aab2d14d96fabb41cc259d49b583e8aee11295190dd235cb8b33`。

## 5. 未验证边界

- 未部署服务器，未调用生产数据库或真实外部矿池；
- 未验证并发提交/复核/application；
- 未验证 Adapter release admission、artifact、verifier 或 credential；
- 未生成 v213 route/credential/service actor/seal；
- 未创建 CapacityPool、Supply、Offer、Job、派发、Runner、ACK/event 或结算。
