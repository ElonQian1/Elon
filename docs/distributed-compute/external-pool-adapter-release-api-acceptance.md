# External Pool Adapter Release 管理员 API 验收

## 1. 结论

状态：`management_surface_verified`，整体仍为 `partially_verified`。

代码提交 `7b043a88f` 为 v222 Adapter release staging 增加管理员写入口；代码提交 `a37595e1a` 继续补齐管理员列表、详情和 actor-aware preflight。六个管理操作已经通过定向真实 Rust 编译、进程内接口测试及 Store 关闭重开测试；PC `/compute-external-pools` 管理员工作台也已通过跨层静态合同、严格类型、lint、生产构建和 bundle budget。该结论只证明受控 staging 管理面可调用且可审计读回，不证明候选 Adapter 已下载、验签、加载、执行或获得 v213 route authority。

v227 artifact source 与 v229 release-admission lifecycle 已通过完整 `elon-server` 编译和 51 项 Windows 临时 DATA_DIR/SQLite 专项，状态为 `design_frozen/implementation_partially_verified`。证据覆盖管理员 HTTP、三终态、迁移/重开、双连接竞争、terminal↔artifact 顺序、raw body 门卫、CAS custody、失败清理、恢复和路径安全；两个 v229 HTTP 操作仍没有 MCP/PC。冻结合同见 [`external-pool-adapter-release-lifecycle-authority.md`](external-pool-adapter-release-lifecycle-authority.md)。

## 2. 接口

- `GET /api/admin/compute/external-pool-adapter-releases`：按状态列出 release request，`limit` 收敛到 1 至 100；
- `POST /api/admin/compute/external-pool-adapter-releases`：提交候选 release；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id`：读取 request、review 与 admission 的组合详情；
- `GET /api/admin/compute/external-pool-adapter-releases/:request_id/preflight`：按当前管理员和账本状态返回 review/stage 可执行性及 blocker；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/review`：由另一名管理员独立复核；
- `POST /api/admin/compute/external-pool-adapter-releases/:request_id/stage`：按 exact request/review 摘要形成 staged admission。

六个入口均要求登录用户角色为 `admin` 或 `owner`。Service 从认证会话派生操作者 ID，请求体不接受提交者、复核者或执行者 ID。对应的 6 个 MCP 工具已经复用同一 Service 并通过角色隔离与治理链专项，见 `compute-management-mcp-acceptance.md`；PC 管理员工作台复用相同 HTTP 合同，不增加旁路权限。

v229 新增两个已通过进程内 HTTP 专项的管理员入口：`POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/terminal` 追加唯一的 `withdrawn|revoked|superseded` 终态，`GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/currentness` 读取派生 currentness。测试覆盖会话/角色、显式确认、幂等重放、冲突、持久 owner 与 `local-owner` actor、三终态及响应脱敏；`superseded` 必须绑定同 Adapter 的 exact current successor，旧 admission 不自动跟随或恢复。v229 不增加 MCP、PC、SDK 或 Provider 本人入口。

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

## 5. v227/v229 联合验证命令与证据

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-external-pool-adapter -- test --manifest-path server\Cargo.toml -p elon-server --bin elon-server external_pool_adapter -- --nocapture
```

- 结果：`CARGO_OK`；
- 测试：51 项通过、0 项失败、1675 项过滤，测试本体耗时 82.49 秒；
- validation fingerprint：`19e2c747c306a4ded01a02f4ef39e914a28d331d7e3fae43a034814f8821e740`；
- validation receipt：`0445dfa90206795ad3917f4892da4ca833d0c54dc8d2c93fc854202f6e931176`。

专项实际覆盖 fresh/repeat/v228 upgrade migration、两次重开、三终态、successor 正反路径、exact replay、双连接 terminal/successor/artifact 竞争、terminal-first、CAS-first/DB-second、receipt-first、response-loss replay、终态前后 PUT/GET、HTTP 输入失败关闭、`.part` 清理、existing CAS 复用/腐化拒绝、blob missing、目录 junction/reparse 与恢复。Windows custody 使用无覆盖硬链接安装，固定目录拒绝 DELETE sharing，并在超长路径和 Tokio 提前返回时显式关闭句柄后清理临时文件。

## 6. 未验证边界

- 未部署服务器，未对生产数据库或真实管理员会话调用；
- 未通过真实 TCP 或已登录浏览器会话验证 PC 页面；
- 未执行真实进程崩溃、断电、磁盘写满或目录替换时序的 fault injection；受控 saga 测试不能替代这些证据；
- 当前 Windows 会话没有创建文件 symlink 的权限，该测试只执行了能力检测；目录 junction/reparse 已通过。Unix 私有权限与 symlink 分支未在本机执行；
- v227/v229 没有 MCP、PC、SDK 或 Provider 本人入口，未通过真实 TCP、生产数据库副本、生产 DATA_DIR 或已登录浏览器验证；
- 未解析或下载 `candidate_artifact_ref`，未重算实现摘要；
- 未验证 verifier registry、签名、供应链或协议能力；
- 未生成 Adapter registry/version、credential、service actor、v213 route/seal；
- 未连接外部矿池网络、worker、ACK/event、派发、Runner 或结算。
