# External Pool Adapter Release 管理员 API 验收

## 1. 结论

状态：`management_surface_verified`，整体仍为 `partially_verified`。

代码提交 `7b043a88f` 为 v222 Adapter release staging 增加管理员写入口；代码提交 `a37595e1a` 继续补齐管理员列表、详情和 actor-aware preflight。六个管理操作已经通过定向真实 Rust 编译、进程内接口测试及 Store 关闭重开测试；PC `/compute-external-pools` 管理员工作台也已通过跨层静态合同、严格类型、lint、生产构建和 bundle budget。该结论只证明受控 staging 管理面可调用且可审计读回，不证明候选 Adapter 已下载、验签、加载、执行或获得 v213 route authority。

v229 release-admission lifecycle 已完成文档冻结，但状态严格为 `design_frozen/source_not_written`：它不属于上述六个已验证操作，也没有 Rust、迁移、HTTP、MCP、PC 或运行证据。冻结合同见 [`external-pool-adapter-release-lifecycle-authority.md`](external-pool-adapter-release-lifecycle-authority.md)。

## 2. 接口

- `GET /api/admin/compute/external-pool-adapter-releases`：按状态列出 release request，`limit` 收敛到 1 至 100；
- `POST /api/admin/compute/external-pool-adapter-releases`：提交候选 release；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id`：读取 request、review 与 admission 的组合详情；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id/preflight`：按当前管理员和账本状态返回 review/stage 可执行性及 blocker；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/review`：由另一名管理员独立复核；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/stage`：按 exact request/review 摘要形成 staged admission。

六个入口均要求登录用户角色为 `admin` 或 `owner`。Service 从认证会话派生操作者 ID，请求体不接受提交者、复核者或执行者 ID。对应的 6 个 MCP 工具已经复用同一 Service 并通过角色隔离与治理链专项，见 `compute-management-mcp-acceptance.md`；PC 管理员工作台复用相同 HTTP 合同，不增加旁路权限。

v229 只冻结两个未来管理员 HTTP 入口，不把它们计入当前接口：`POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/terminal` 追加唯一的 `withdrawn|revoked|superseded` 终态，`GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/currentness` 读取派生 currentness。`superseded` 必须精确绑定同一 Adapter 的另一条、当时仍为 current staged 的 successor admission；旧 admission 永不自动跟随或恢复。v229 不增加 MCP、PC、SDK 或 Provider 本人入口。

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

PC 静态验证复用 `npm run test:compute-external-pools`、`typecheck`、`lint`、`build` 与 `check:bundle-budget`。页面固定六项 capability revision，独立展示 submit/review/stage 阶段及 blocker，不采信自由 JSON，也不把 staged admission 描述为已下载、已验签或可路由 Adapter。

## 5. 未验证边界

- 未部署服务器，未对生产数据库或真实管理员会话调用；
- 未通过真实 TCP 或已登录浏览器会话验证 PC 页面；
- 当前没有 admission terminal/currentness 运维入口；v229 仅把 `staged -> withdrawn|revoked|superseded`、唯一追加式 terminal receipt、派生 current view、幂等与并发合同冻结为 `design_frozen/source_not_written`；
- v229 计划令 terminal receipt 固定报告 `currentness_effect=admission_terminal`、`artifact_intake_effect=blocked`、`existing_artifact_source_effect=historical_only`、`adapter_effect=none`、`route_effect=none`；terminal 后的 v227 PUT（含 exact replay）失败关闭，历史 receipt 仅可 GET；
- v229 计划在 v227 raw-body/CAS 前、Store fresh/exact replay 事务内和新的 v229 `BEFORE INSERT` trigger 重审 currentness；不回改 v227 旧 migration，也尚无竞争或故障注入证据；
- 未解析或下载 `candidate_artifact_ref`，未重算实现摘要；
- 未验证 verifier registry、签名、供应链或协议能力；
- 未生成 Adapter registry/version、credential、service actor、v213 route/seal；
- 未连接外部矿池网络、worker、ACK/event、派发、Runner 或结算。
