# External Pool Adapter Release 管理员 API 验收

## 1. 结论

状态：`management_surface_verified`，整体仍为 `partially_verified`。

代码提交 `7b043a88f` 为 v222 Adapter release staging 增加管理员写入口；代码提交 `a37595e1a` 继续补齐管理员列表、详情和 actor-aware preflight。六个管理操作已经通过定向真实 Rust 编译、进程内接口测试及 Store 关闭重开测试。该结论只证明受控 staging 管理面可调用且可审计读回，不证明候选 Adapter 已下载、验签、加载、执行或获得 v213 route authority。

## 2. 接口

- `GET /api/admin/compute/external-pool-adapter-releases`：按状态列出 release request，`limit` 收敛到 1 至 100；
- `POST /api/admin/compute/external-pool-adapter-releases`：提交候选 release；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id`：读取 request、review 与 admission 的组合详情；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id/preflight`：按当前管理员和账本状态返回 review/stage 可执行性及 blocker；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/review`：由另一名管理员独立复核；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/stage`：按 exact request/review 摘要形成 staged admission。

六个入口均要求登录用户角色为 `admin` 或 `owner`。Service 从认证会话派生操作者 ID，请求体不接受提交者、复核者或执行者 ID。当前没有对应 MCP/PC 管理工具。

## 3. 已验证行为

- 未登录返回 `401`，普通用户返回 `403`；
- `deny_unknown_fields` 拒绝外部注入 `submitted_by_admin_user_id`；
- 未显式确认的 submit/review/stage 失败关闭；
- submit 固定 `server_adapter` 与 `external_pool`，并在服务端计算 capability set digest；
- 提交者不能复核同一 request，独立管理员可作 `approved` 复核；
- request、material 与 review 摘要必须精确匹配；
- exact submit/review/stage 重放返回原 ID 和 `replayed=true`；
- 改变历史字段的同幂等键重放被拒绝；
- `changes_requested` 不能 stage；
- HTTP 回执不暴露 artifact ref 或 verifier 详情；
- 列表支持 `submitted`、`approved`、`changes_requested`、`rejected`、`staged` 五种状态，未知状态失败关闭；
- 详情组合 exact-audited request/review/admission 脱敏回执，且不绕过现有摘要和投影审计；
- preflight 能区分提交者不可自审、待复核、可 stage、需重新提交、已拒绝和已 stage；
- 列表、详情和 preflight 均执行管理员鉴权，普通成员不能读取发布账本；
- Store 关闭重开后仍可按状态读取 exact request/review/admission 投影；
- 最终效果保持 `staged_admission_only`，三张账本各只产生一行。

## 4. 验证命令与证据

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-external-pool-adapter-release-management -- test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_release -- --nocapture
```

- 结果：`CARGO_OK`；
- 测试：7 项通过，包含原写链回归、管理 API 和 Store 重启读取；
- validation fingerprint：`2b55e579b08b89acf2e6bc1065914755bba1f7bbfad10c9917d61c3c32dbea3d`；
- validation receipt：`873f84be88b448fa00b7f9b15195ebcba439ee1cee558df5c41a6793f017ecc6`。

## 5. 未验证边界

- 未部署服务器，未对生产数据库或真实管理员会话调用；
- immutable release request 在 review 后关闭，当前未提供撤回或 supersede 运维入口；
- 未解析或下载 `candidate_artifact_ref`，未重算实现摘要；
- 未验证 verifier registry、签名、供应链或协议能力；
- 未生成 Adapter registry/version、credential、service actor、v213 route/seal；
- 未连接外部矿池网络、worker、ACK/event、派发、Runner 或结算。
