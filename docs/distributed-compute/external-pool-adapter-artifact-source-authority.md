---
title: 外部矿池 Adapter Artifact Bytes Source 权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact Bytes Source 权威

## 1. 权威范围与当前结论

本文冻结 v227 的单一纵切面：平台管理员把原始字节绑定到一份 exact v222 `staged` admission；服务端把字节写入自己管理的 DATA_DIR quarantine 内容寻址文件，关闭后重新打开最终文件，全量重算 SHA-256 与长度；只有重算结果仍与 admission 的 `declared_implementation_sha256` 一致，Store 才保存不可变 source receipt。

这份 receipt 只证明“本服务实际接收并持有的一组字节，其重开后摘要和长度与 admission 的声明一致”。它不证明字节来自 `candidate_artifact_ref`，不证明签名、供应链、格式、安全性、conformance、六能力、verifier currentness 或外部矿池可连接，也不构造 verified/trusted/attested Adapter。

当前状态是 `implementation_partially_verified`：完整 `elon-server` 目标已编译，51 项 Windows 临时 DATA_DIR/SQLite 专项覆盖 HTTP/pre-CAS 输入失败关闭、超长路径 `.part` 清理、内容寻址复用/腐化拒绝、blob missing、目录 junction/reparse、CAS/DB saga 重试与 v229 currentness。该结果只证明本地测试 artifact，不是生产摄入或可信 Adapter。

v229 admission lifecycle 已把三层 currentness 接入领域、migration、Store、Service/HTTP 与 v227。专项证明终态后的 fresh/exact PUT 在 body 被 poll 前拒绝，既有 receipt 的历史 GET 仍可重开 bytes，并覆盖 terminal-first、CAS-first/DB-second、receipt-first 与双连接竞争；v229 不改写 v227 旧 migration。详见 [`external-pool-adapter-release-lifecycle-authority.md`](external-pool-adapter-release-lifecycle-authority.md)。

## 2. 唯一管理入口

首版只开放两项平台管理员 HTTP，不开放 MCP、PC、SDK、下载或 Provider owner 自助入口：

- `PUT /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-source`：摄入 raw bytes；
- `GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-source`：读取历史 receipt，并实时重开文件复核 current bytes。

PUT 必须先完成 Bearer 鉴权并确认角色是 `admin|owner`，再读取 body。请求固定为：

- `Content-Type: application/octet-stream`，拒绝任意 `Content-Encoding`；
- `Idempotency-Key` 长度 1..160；
- `X-Elon-Expected-Admission-Digest` 为 64 位小写十六进制；
- `X-Elon-Artifact-Source-Confirmation: confirm_external_pool_adapter_artifact_source_intake`；
- body 非空且最多 33,554,432 字节。

调用方不得提交 request/review/ref、声明摘要、observed 摘要、文件名、路径、storage key、actor 或时间；Service 必须从 exact staged admission、认证会话、服务端时间与实际 body 派生。v229 源码现要求只有派生 current status 仍为 `staged` 才能读取/流式消费 body 或进入 CAS；已有 terminal 时，包含旧 exact idempotent replay 在内的 PUT 都必须失败关闭。首写返回 201，仍 current 时的 exact replay 与 GET 返回 200。摘要或 lineage/currentness/幂等冲突返回 409，超限返回 413，媒体类型或编码不符返回 415，body 摘要与 admission 声明不符返回 422；receipt 已存在但 blob 缺失或漂移必须失败关闭。

GET 不返回 bytes、下载地址、绝对路径、candidate ref 或凭据。terminal 后 GET 仍可读取 immutable 历史 receipt，并重开最终普通文件复核 bytes；回执必须明确 `existing_artifact_source_effect=historical_only`。历史 receipt 不能替代实时 currentness：未来 registry 每次消费都必须同时重算 bytes 并确认 admission 仍无 terminal。

## 3. 服务端文件来源

内容寻址根固定为：

`DATA_DIR/compute-federation/external-pool-adapter-artifacts/v1/quarantine/blobs/sha256/<first2>/<sha256>.blob`

路径中的可变部分只能来自已校验格式的 64 位小写 SHA-256；不得把 `candidate_artifact_ref`、URL、请求文件名或用户输入拼入路径。服务端必须拒绝 symlink、junction、reparse point、非普通文件及越出固定根的目标。

部署信任锚必须明确：`AppState.data_dir` 及其父链由受信平台配置，不能被非受信本地 actor rename、replace 或改成 reparse；固定 quarantine namespace 只允许单一 service OS identity 写入，同 UID 的其他恶意 writer 不在本批隔离声明内。Windows 实现须从 DATA_DIR 到 shard 逐级保留 no-reparse 且拒绝 WRITE/DELETE sharing 的目录 handle，并把它们保留到 Store 消费 sealed evidence；Unix DATA_DIR 必须拒绝 group/other write，固定 namespace 必须为 0700。最终文件另以 no-follow handle 打开，并在哈希后再次核 canonical root 与平台 file identity。任一门卫失败均不能产生 receipt。

写入顺序固定为：

1. 流式读取并计算 intake SHA-256/长度，先与 admission 声明一致；
2. 在同一 shard 以 `create_new` 创建 `.part` 临时文件，写完后 `flush`/`sync_all`；
3. 采用同盘 no-clobber 原子安装；已有 final 绝不覆盖；
4. 同步父目录；
5. 重新打开最终 `.blob` 普通文件，流式重算 SHA-256/长度并复核 metadata；
6. 只把 sealed bytes evidence 交给 Store，不交付绝对路径或可由 DTO 构造的“verified”类型。

若 CAS final 已存在，只能重开并证明摘要与长度 exact 后复用；腐化或非普通目标必须失败，绝不覆盖。正常失败尽力删除本次 `.part`；异常断电留下的孤儿 `.part` 和未引用 CAS blob 由后续独立 GC/恢复批处理处理，绝不能被自动当作 authority。

## 4. 文件与 SQLite 的 saga 边界

文件系统与 SQLite 不是一个原子事务，不得宣称为单事务提交。首版采用 CAS-first、DB-second：

1. 在读取 raw body 前取得 exact、无 terminal 的 staged intake authority；
2. 完成最终文件 no-clobber 安装与重开重算；
3. 开启 SQLite `BEGIN IMMEDIATE`；
4. 对 fresh write 和 exact replay 都重审 admission currentness；
5. 对 fresh write 再由 v227 exact-source trigger 与独立 v229 currentness trigger 重审 request/review/admission lineage 和无 terminal 条件；
6. 插入一份不可变 receipt 并 exact readback；
7. 提交数据库事务。

崩溃窗口的裁决固定为：

- temp 写入前/中失败：无 DB；孤儿 `.part` 不可采用；
- CAS 已安装、DB 未提交：留下未引用 blob；同一请求重试必须重开复核后复用；
- DB 已提交、响应丢失：同 key exact replay 重开复核后返回既有 receipt；
- CAS 已安装、terminal 在 DB-second 前提交：receipt 写入失败，留下的未引用 blob 不能成为 authority；
- receipt 先提交、terminal 后提交：历史 receipt/blob 保留，PUT replay 与 future consumer 失败，GET 仍可读取历史；
- Store 结果不明：保留 CAS，返回失败并只允许读回或同 key 重试，不删除 final；
- receipt 存在但 blob 缺失、非普通或摘要/长度漂移：保留历史行，所有读取和 consumer 失败关闭，绝不改写 receipt。

全盘启动扫描、孤儿删除、成对备份/恢复与多机对象存储不属于 v227。本地 DATA_DIR receipt 只能称 server-local bytes source，不能称 HA/global custody。

## 5. v227 不可变账本

v227 只新增 `compute_external_pool_adapter_artifact_source_receipts`，一份 admission 最多一份 receipt。账本必须保存并投影：

- JCS identity：schema、receipt id/digest/json、`rfc8785_jcs`、`sha256`；
- exact lineage：admission/request/review IDs 与 digests、request material digest、Adapter id/release version、candidate ref 与 declared SHA；
- observed bytes：intake/reopened SHA、字节长度与 SHA-256 content address；
- custody：`server_data_dir`、固定 namespace、`quarantined`；
- fixed effects：`admin_authenticated_raw_body`、`byte_digest_match_only`、`artifact_ref_resolution_effect=none`、`adapter_effect=none`、`route_effect=none`；
- actor/confirmation/server time、稳定 intake material digest、idempotency scope/key。

declared、intake、reopened 与 content-address 四个 SHA 必须 exact 相等；长度必须为 1..33,554,432。receipt digest、admission id 和 `(idempotency_scope,idempotency_key)` 唯一。v227 BEFORE INSERT trigger 必须 exact JOIN v222 staged admission、request 与 approved review；v229 已新增独立 BEFORE INSERT currentness trigger 源码，拒绝任何已有 terminal，而不修改 v227 旧 migration。另有 JSON projection、no-update、no-delete 与 no-replace 门卫。

Store exact replay 顺序是先按 scope/key 查既有回执并逐字段比较，再以 v229 sealed current authority 重审 admission；fresh 与 replay 的同一 IMMEDIATE 事务都要求“无 terminal”。terminal 后 PUT exact replay 失败，历史 receipt 只能由 GET 读取。相同 admission 使用不同 key，或同 key 任一 material 漂移，均冲突。不同 admissions 可以共享同一内容寻址 blob，但必须各自保存 lineage receipt。

## 6. 信任截止线与 P0 禁线

- 禁止把 `candidate_artifact_ref` 当作已解析来源；raw body 只证明管理员提交的 bytes。
- 禁止把 SHA 相等称为 verified/trusted/attested Adapter、供应链或签名证明。
- 禁止 URL fetch、网络访问、multipart/base64、调用方路径/文件名、SQLite BLOB、解压、加载、执行或 conformance 探测。
- 禁止创建或修改 v213 Adapter/version、credential、route authorization、capability、seal 或 source trigger。
- 禁止构造 verifier registry/currentness、service actor、credential proof 或 Provider activation。
- 禁止改变 Provider/Pool/Supply/Offer/Job/Reservation/Plan/outbox/Lease/usage/资金/结算。
- 禁止提供 bytes 下载、MCP/PC 二进制入口或把 server-local DATA_DIR 描述为共享对象存储。
- 禁止用 fixture、fake resolver、管理员 DTO 或历史 receipt 升级生产 trust。

v231 已另行闭合 exact Artifact RSA signed provenance，但不改变 v227 的 bytes-source 截止线。后续 Adapter registry consumer 仍须闭合 Artifact 格式与 sandbox conformance、verifier registry/currentness/revocation、平台级 release actor 与 Provider-specific route actor 分权，以及 exact v222/v227/v231 companion。v227 永远不跨越该截止线。见 [`external-pool-adapter-artifact-signed-provenance-authority.md`](external-pool-adapter-artifact-signed-provenance-authority.md)。

v229 terminal receipt 固定 `currentness_effect=admission_terminal`、`artifact_intake_effect=blocked`、`existing_artifact_source_effect=historical_only`、`adapter_effect=none` 与 `route_effect=none`。该合同已纳入 v227/v229 联合编译、migration、重开和运行专项，但仍无真实 TCP、生产数据库升级或部署证据。

## 7. 实现与验收门槛

当前源码批标记为 `implementation_partially_verified`。实际命令、51 项结果、fingerprint 和 receipt 见 [`external-pool-adapter-release-api-acceptance.md`](external-pool-adapter-release-api-acceptance.md)；不得把临时 fixture 摄入写成生产 artifact 或可信 Adapter。

后续仍须补齐真实进程崩溃/断电/磁盘故障注入、Windows 有 symlink 权限时的 leaf reparse 分支、Unix 权限与 symlink 动态证据、生产数据库升级、真实 TCP 和生产 DATA_DIR。完成这些证据后仍不得声称生产 Adapter 已验证或可执行，除非 provenance、signature、conformance、verifier、registry 与 route 另行闭合。
