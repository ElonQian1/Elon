# External Pool Onboarding API 验收

## 1. 结论

状态：`partially_verified`。

本验收对应的 v221 onboarding 增加 owner submit、管理员 review 与 immutable application 三条 Service/HTTP 写入口；合并远端 v223 后同步修复其价格曲线 read helper 的模块可见性，并通过定向真实 Rust 编译和 2 项进程内接口测试。该结论只证明受控来源登记可调用，不证明 Provider 已激活、可路由、可供给容量或可执行任务。

## 2. 已实现接口

- `POST /api/me/compute/external-pool-onboarding-requests`；
- `POST /api/admin/compute/external-pool-onboarding-requests/:request_id/review`；
- `POST /api/admin/compute/external-pool-onboarding-requests/:request_id/application`。

submit 要求任意已登录 owner；review/application 要求 `admin` 或 `owner` 平台角色。Service 从会话派生 owner、reviewer 与 applier，不接受外部 actor 字段。Provider owner、settlement account、`external_pool/registering/self_declared`、revision 1、无 endpoint 及 Adapter ref 均由服务端构造。

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
- 最终效果固定为 `provider_registered_only`，不写 v213 route。

## 4. 验证命令与证据

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-external-pool-onboarding-api -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation::external_pool_onboarding_api::tests -- --nocapture
```

- 结果：`CARGO_OK`；
- 测试：2 项通过；
- validation fingerprint：`94c1604e56f6a70e167118845fba5cb3d6b00b9599348a8c30672a12dcf73e01`；
- validation receipt：`1ede1b0535fcc13192de3b9e85859d015575531986eab377b97adf7c7b413f7a`。

## 5. 未验证边界

- 未部署服务器，未调用生产数据库或真实外部矿池；
- 未实现 owner/admin 列表、详情、cancel 或 preflight；
- 未验证并发提交/复核/application；
- 未验证 Adapter release admission、artifact、verifier 或 credential；
- 未生成 v213 route/credential/service actor/seal；
- 未创建 CapacityPool、Supply、Offer、Job、派发、Runner、ACK/event 或结算。
