# External Pool Adapter Release 管理员 API 验收

## 1. 结论

状态：`management_surface_verified`，整体仍为 `partially_verified`。

代码提交 `7b043a88f` 为 v222 Adapter release staging 增加管理员写入口；代码提交 `a37595e1a` 继续补齐管理员列表、详情和 actor-aware preflight。六个管理操作已经通过定向真实 Rust 编译、进程内接口测试及 Store 关闭重开测试；PC `/compute-external-pools` 管理员工作台也已通过跨层静态合同、严格类型、lint、生产构建和 bundle budget。该结论只证明受控 staging 管理面可调用且可审计读回，不证明候选 Adapter 已下载、验签、加载、执行或获得 v213 route authority。

v229 release-admission lifecycle 的领域、migration、Store、Service/HTTP、v227 currentness 与分组测试源码已经写入，但状态严格为 `design_frozen/implementation_uncompiled/implementation_unrun`。新增测试 passed=0，未编译、未执行 migration 或运行，无 validation fingerprint/receipt，实际 artifact/terminal=0；两个新 HTTP 操作不属于下述六个已验证操作，也没有 MCP/PC。冻结合同见 [`external-pool-adapter-release-lifecycle-authority.md`](external-pool-adapter-release-lifecycle-authority.md)。

## 2. 接口

- `GET /api/admin/compute/external-pool-adapter-releases`：按状态列出 release request，`limit` 收敛到 1 至 100；
- `POST /api/admin/compute/external-pool-adapter-releases`：提交候选 release；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id`：读取 request、review 与 admission 的组合详情；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id/preflight`：按当前管理员和账本状态返回 review/stage 可执行性及 blocker；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/review`：由另一名管理员独立复核；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/stage`：按 exact request/review 摘要形成 staged admission。

六个入口均要求登录用户角色为 `admin` 或 `owner`。Service 从认证会话派生操作者 ID，请求体不接受提交者、复核者或执行者 ID。对应的 6 个 MCP 工具已经复用同一 Service 并通过角色隔离与治理链专项，见 `compute-management-mcp-acceptance.md`；PC 管理员工作台复用相同 HTTP 合同，不增加旁路权限。

v229 源码新增两个尚未验证的管理员 HTTP 入口，不把它们计入当前已验证接口：`POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/terminal` 追加唯一的 `withdrawn|revoked|superseded` 终态，`GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/currentness` 读取派生 currentness。未执行测试源码拟断言 `401/403/404/409`、fresh `201`、replay/GET `200`、unknown field、确认语、持久 owner 与 `local-owner` actor、三终态及响应脱敏；`superseded` 仍须绑定同 Adapter 的 exact current successor，旧 admission 不自动跟随或恢复。v229 不增加 MCP、PC、SDK 或 Provider 本人入口。

## 3. v222 已验证行为

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

## 4. v222 验证命令与证据

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
- admission terminal/currentness 运维入口及 migration/Store/HTTP/并发/currentness 测试源码已写，但新增测试 passed=0，未编译、未执行 migration 或运行，无 fingerprint/receipt，实际 artifact/terminal=0；v229 仍为 `design_frozen/implementation_uncompiled/implementation_unrun`；
- 未执行断言拟检查三终态、exact successor、fixed effects、fresh/replay/currentness 状态码、管理员/owner/`local-owner` actor、双连接竞争，以及 terminal 后 v227 PUT 在 raw body 被 poll 前拒绝、历史 GET 保留；v227 新增测试源码再覆盖 pre-CAS 鉴权/header/大小/摘要拒绝、临时文件清理、CAS 复用/腐化拒绝、blob missing、路径/权限/reparse 门卫；这些不构成接口或运行证据；
- test source 也表达 terminal-first、CAS-first/DB-second、receipt-first、response-loss replay 与 artifact/terminal 竞争，但实际执行、精确进程崩溃/断电 fault injection、目标平台动态 handle 证据和生产升级仍缺；不回改 v227 旧 migration；
- 未解析或下载 `candidate_artifact_ref`，未重算实现摘要；
- 未验证 verifier registry、签名、供应链或协议能力；
- 未生成 Adapter registry/version、credential、service actor、v213 route/seal；
- 未连接外部矿池网络、worker、ACK/event、派发、Runner 或结算。
