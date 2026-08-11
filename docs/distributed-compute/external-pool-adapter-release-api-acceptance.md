# External Pool Adapter Release 管理员 API 验收

## 1. 结论

状态：`partially_verified`。

代码提交 `7b043a88f` 为 v222 Adapter release staging 增加管理员 Service/HTTP 入口，并通过定向真实 Rust 编译和 2 项进程内接口测试。该结论只证明受控 staging 入口可调用，不证明候选 Adapter 已下载、验签、加载、执行或获得 v213 route authority。

## 2. 接口

- `POST /api/admin/compute/external-pool-adapter-releases`：提交候选 release；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/review`：由另一名管理员独立复核；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/stage`：按 exact request/review 摘要形成 staged admission。

三个入口均要求登录用户角色为 `admin` 或 `owner`。Service 从认证会话派生操作者 ID，请求体不接受提交者、复核者或执行者 ID。当前没有对应 MCP/PC 写工具。

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
- 最终效果保持 `staged_admission_only`，三张账本各只产生一行。

## 4. 验证命令与证据

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-external-pool-adapter-release-api -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation::external_pool_adapter_release_api::tests -- --nocapture
```

- 结果：`CARGO_OK`；
- 测试：2 项通过；
- validation fingerprint：`e21798874a16f25eb9e2364ecb01a866c7b6f5d2ce0eb85c020b557392c85db8`；
- validation receipt：`a38e67770c6a55870f000b7a781e7ad3afac622c031664b3d344b8733e54eea0`。

## 5. 未验证边界

- 未部署服务器，未对生产数据库或真实管理员会话调用；
- 未提供列表、详情查询、撤回或 supersede 运维入口；
- 未解析或下载 `candidate_artifact_ref`，未重算实现摘要；
- 未验证 verifier registry、签名、供应链或协议能力；
- 未生成 Adapter registry/version、credential、service actor、v213 route/seal；
- 未连接外部矿池网络、worker、ACK/event、派发、Runner 或结算。
