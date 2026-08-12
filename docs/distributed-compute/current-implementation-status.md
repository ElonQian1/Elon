---
title: 分布式算力当前实现状态
status: current
reviewed_at: 2026-08-13
owners: backend, pc, node-agent, ai-economy
implementation_status: mixed
---

# 分布式算力当前实现状态

本文承接 `AI_CURRENT.md` 的分布式算力细节，只记录已实现边界、验证强度和仍缺生产闭环。领域合同与验收细节以各链接专题文档为准。

## 供给与市场控制面

- API Token 保管、远程节点、计算使用台账，以及默认关闭的节点模型算力共享策略已实现但边界有限。节点所有者可选择共享模型、最大并发和每日 Token 预算；指定与自动调度会在同一事务中检查今日实耗、活动租约预留和本次保守预留，所有者自用不受共享限制。可信终态先冻结实际 Token，再进入异步结算；提前断流、真正租约过期和服务重启会失败关闭并幂等释放预授权，最终状态不接受迟到覆盖。所有者控制面会从持久化记录派生近 24 小时失败、预留超出和过期租约健康快照。该快照不是 SLA 裁决或自动赔付，当前仍不是包含异构任务、竞价、故障赔付、真实提现或链上结算的开放算力市场。
- 联邦基础合同已定义 Provider、Workload、Offer、SKU、Price Snapshot、Job、Reservation、Attempt Lease、用量与结算回执，但尚未完整接线。旧 LLM `NodeComputeRun` 已接入认证 `/api/me/node-usage` 的附加只读兼容投影，只认 `server_node_llm/node_llm`，固定 `partial/provider_reported_unverified`。PC 账本的兼容观察增量未编译、未运行或做浏览器验证，不生成 Job、Lease、Receipt 或可信计量。共享 CapacityPool 已形成 v165-v168 Store 与追加式多 meter Claim/账本；Reservation Claim 强制绑定 Offer、Job 和 Reservation，公开 standalone 入口拒绝绕开 Broker。
- 本人 Provider、Pool、Bucket 和 Supply 控制面已通过 Rust/SQLite、进程内 HTTP/MCP 和 PC 静态构建专项，可完成双 meter 供给追加、撤回、幂等重放、显式确认、鉴权、本人隔离、失败原子性、账本审计及数据库重开。真实 TCP、PC 浏览器交互和发布仍未验证；self-declared/available 不等于硬件 verified、市场报价、可预留容量或收入。见 `supply-control-plane-acceptance.md` 与 `pc-compute-build-acceptance.md`。
- v177-v181、v203-v205 激活、隔离和恢复已通过 Store/Service、本人/管理员 HTTP/MCP、角色隔离、幂等及文件重开专项；PC `/compute-supply` 与 `/compute-activation` 已静态生产构建。并发压力、真实 TCP、浏览器和生产库副本仍未验证；流程不连接节点、不派发、不发布 Offer 或移动资金。见 `activation-control-plane-acceptance.md`。
- v170、v182-v184 Offer 已通过 Store/Service、本人和管理员 HTTP/MCP、角色隔离、幂等重放及文件重开专项；PC `/compute-supply` 与 `/compute-offers` 已静态生产构建。并发压力、真实 TCP、浏览器、生产库副本和发布仍未验证；流程不生成 Snapshot、不取消已有预留、不派发或移动资金。见 `offer-control-plane-acceptance.md`。
- v171 Snapshot 与 v172-v176 Broker 已通过 HTTP/MCP、并发和文件重开专项；v223/v224 reference fallback 又通过四眼 HTTP/MCP、原子 Snapshot、拒绝零副作用、旧 TTL 触发器升级和重开专项，固定 `fallback_curve/sample_count=0`。PC `/compute-reference-curves` 已通过跨层契约、严格类型、lint、生产构建和 bundle budget；真实 TCP、浏览器、生产库副本、真实价格、撮合、派发和结算仍缺。见 `compute-management-mcp-acceptance.md`、`platform-reference-price-curve-api-acceptance.md` 与 `broker-control-plane-acceptance.md`。
- v225/v228 维持既有结论。v234 server-owned worker 可按持久 keyset checkpoint 公平恢复已行权但到期的 Reservation，每 tick 最多 100 项；它不是 admin actor，不新增经济权威，成功仍只退款、归还 child Claim 并结束 Job/Reservation。v238 CapacityInstrument 已通过完整服务端编译和全新文件数据库迁移冒烟，但专属 lifecycle/adoption、下游门卫和管理员 HTTP 行为仍为 `passed=0`。Retirement 不锁死历史退款或容量归还；这些能力都不代表真实市场、价格、计量、执行或结算。见 `delivery-allocation-acceptance.md` 与 `capacity-instrument-acceptance.md`。

## Attempt 与外部 Adapter

- Attempt v175-v215、v221/v222 管理面及 v227-v252 Adapter 前置门已形成分层权威，生产成功链仍未运行。V239 形成限时动态沙箱签名收据，V241/V242 固定凭据验证器实现与公钥，V243 形成短时凭据验证回执，V244 汇合为可撤销采用授权；V244 migration 定向测试及 2 项真实 HTTP 链路测试通过。V245 legacy 投影加固 3 项迁移测试通过；V246 从 V227 CAS 同一复验句柄按 V232 manifest 安全解包、原子发布内容寻址目录并登记 `installed_inert`，V247 追加安装撤销终态与 sealed current-authority seam，二者文件系统/迁移 10 项及安装/撤销 HTTP 3 项通过，并修复 Windows 深层命名空间的原子发布长路径失败。主线 V248 属于 ERP；V249 用同一 V247 sealed 文件树登记 Provider-neutral release 与每个 Provider 独立的 installed-inert companion，V251 原位修复凭据验证器 JSON 合法键序被误判为冲突的问题；registry/companion migration 6 项与 HTTP 4 项通过。独立的 V250 为 exact V249 release 增加可续签漏洞情报 re-attestation，已通过 migration 合同 4 项与进程内 Axum HTTP 5 项，覆盖签名 genesis、重放、续签、stale predecessor、撤销、脱敏、scanner key 撤销和 intelligence 到期失败关闭。V252 以新 ABI 建立 release 级 durable challenge、RSA verifier 签名、sequence/predecessor、currentness 与撤销的六能力沙箱 re-attestation；源码已写入但未编译、未执行 migration、未运行测试或服务（`passed=0`）。V250/V252 只验证签名声明，不运行扫描或真实沙箱；V252 不接收 Prepared、不证明 live-FS/installed instance current。V239 保持历史不变。生产库原位升级、V252 专项运行验收、可续签 Provider-specific credential、同事务 Provider activation/service actor/v213 compatibility/route、生产 Adapter、Runner、派发、计量结算、生产 TCP/部署/MCP/PC 仍缺。见 `external-pool-adapter-sandbox-reattestation-authority.md`、`external-pool-adapter-vulnerability-reattestation-authority.md`、`external-pool-adapter-registry-authority.md` 与对应验收文档。
- 算力 Attempt PC 的 12 个角色路由已通过跨层合同、严格类型、lint、生产构建和 bundle budget；Gateway 未接线，人工 Start、Renew、no-start Abort 仍关闭。v226 final-usage fence 源码已写但未编译、迁移或运行；它不新增账本或经济效果。操作级后端、真实 TCP/浏览器、生产库和发布仍未验收。见 `pc-compute-attempt-workbenches-acceptance.md` 与 `attempt-final-usage-fence-authority.md`。

## 节点插件与端点会话

- 节点插件已有 V1 policy、v5/v6 catalog/rollback、sealed SQLite/VFS 与线性 custody；69 项 SQLite 专项及 v216 的 11 项版本链测试已有历史证据。A1/A2b2 仍未编译或运行。A2c test-only Windows 源码已接 route-exact `ShmMap`/`ShmLock` callback-before、隔离 VFS unregister before/after 和四条隔离 direct `xShmUnmap(false)` physical-subset bridge，但这些桥不是完整 Case，仍为 `implementation_uncompiled/implementation_unrun`、`passed=0`、`WindowsDynamic=0`。native failure、完整 custody、竞争和逐 case 动态验证仍缺；生产 VFS/open、A1、v15、Runtime/Ready/派发继续不可达，v14 永久 blocked-only。见 `node-plugin-planning-snapshot-authority.md` 与 `node-plugin-vfs-fault-authority.md`。
- 服务端 v216-v218 已铺 endpoint credential/session、目标绑定 owner 重认证与单次消费账本；默认关闭的 direct-TLS owner API、Windows DPAPI bootstrap、legacy no-downgrade 与独立 v13 auth-only WSS 也已写入。v14 固定为 `planning_snapshot_bootstrap_only`，终点仍为 `snapshot_ready=false`；连接不进入 `AgentEntry`/`NodeRegistry`，不产生 signed Plan、work-admission、Runtime、Ready、route、outbox、Lease 或派发。新增仍未编译、测试、运行或执行迁移。见 `node-endpoint-session-authority.md`。
