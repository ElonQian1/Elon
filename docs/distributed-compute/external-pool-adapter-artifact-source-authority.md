---
title: 外部矿池 Adapter Artifact Bytes Source 权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_not_started
---

# 外部矿池 Adapter Artifact Bytes Source 权威

## 1. 权威范围与当前结论

本文冻结 v227 的单一纵切面：平台管理员把原始字节绑定到一份 exact v222 `staged` admission；服务端把字节写入自己管理的 DATA_DIR quarantine 内容寻址文件，关闭后重新打开最终文件，全量重算 SHA-256 与长度；只有重算结果仍与 admission 的 `declared_implementation_sha256` 一致，Store 才保存不可变 source receipt。

这份 receipt 只证明“本服务实际接收并持有的一组字节，其重开后摘要和长度与 admission 的声明一致”。它不证明字节来自 `candidate_artifact_ref`，不证明签名、供应链、格式、安全性、conformance、六能力、verifier currentness 或外部矿池可连接，也不构造 verified/trusted/attested Adapter。

当前状态是 `implementation_not_started`：本页只冻结实现合同，尚无 v227 源码、迁移、文件摄入、回执或运行证据。实际 artifact 测量数为 0。

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

调用方不得提交 request/review/ref、声明摘要、observed 摘要、文件名、路径、storage key、actor 或时间；Service 必须从 exact staged admission、认证会话、服务端时间与实际 body 派生。首写返回 201，exact replay 与 GET 返回 200。摘要或 lineage/幂等冲突返回 409，超限返回 413，媒体类型或编码不符返回 415，body 摘要与 admission 声明不符返回 422；receipt 已存在但 blob 缺失或漂移必须失败关闭。

GET 不返回 bytes、下载地址、绝对路径、candidate ref 或凭据。历史 receipt 不能替代实时 currentness：每次读取和未来每次消费都必须重新打开最终普通文件并重算摘要与长度。

## 3. 服务端文件来源

内容寻址根固定为：

`DATA_DIR/compute-federation/external-pool-adapter-artifacts/v1/quarantine/blobs/sha256/<first2>/<sha256>.blob`

路径中的可变部分只能来自已校验格式的 64 位小写 SHA-256；不得把 `candidate_artifact_ref`、URL、请求文件名或用户输入拼入路径。服务端必须拒绝 symlink、junction、reparse point、非普通文件及越出固定根的目标。

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

1. 完成最终文件 no-clobber 安装与重开重算；
2. 开启 SQLite `BEGIN IMMEDIATE`；
3. 重审 exact staged admission/request/approved review、摘要、actor scope 与幂等；
4. 插入一份不可变 receipt 并 exact readback；
5. 提交数据库事务。

崩溃窗口的裁决固定为：

- temp 写入前/中失败：无 DB；孤儿 `.part` 不可采用；
- CAS 已安装、DB 未提交：留下未引用 blob；同一请求重试必须重开复核后复用；
- DB 已提交、响应丢失：同 key exact replay 重开复核后返回既有 receipt；
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

declared、intake、reopened 与 content-address 四个 SHA 必须 exact 相等；长度必须为 1..33,554,432。receipt digest、admission id 和 `(idempotency_scope,idempotency_key)` 唯一。BEFORE INSERT trigger 必须 exact JOIN v222 staged admission、request 与 approved review；另有 JSON projection、no-update、no-delete 与 no-replace 门卫。

Store 的 exact replay 顺序是：先按 scope/key 查既有回执并逐字段比较，再重审 admission；相同 admission 使用不同 key，或同 key 任一 material 漂移，均冲突。不同 admissions 可以共享同一内容寻址 blob，但必须各自保存 lineage receipt。

## 6. 信任截止线与 P0 禁线

- 禁止把 `candidate_artifact_ref` 当作已解析来源；raw body 只证明管理员提交的 bytes。
- 禁止把 SHA 相等称为 verified/trusted/attested Adapter、供应链或签名证明。
- 禁止 URL fetch、网络访问、multipart/base64、调用方路径/文件名、SQLite BLOB、解压、加载、执行或 conformance 探测。
- 禁止创建或修改 v213 Adapter/version、credential、route authorization、capability、seal 或 source trigger。
- 禁止构造 verifier registry/currentness、service actor、credential proof 或 Provider activation。
- 禁止改变 Provider/Pool/Supply/Offer/Job/Reservation/Plan/outbox/Lease/usage/资金/结算。
- 禁止提供 bytes 下载、MCP/PC 二进制入口或把 server-local DATA_DIR 描述为共享对象存储。
- 禁止用 fixture、fake resolver、管理员 DTO 或历史 receipt 升级生产 trust。

后续 Adapter registry consumer 仍须另行闭合 artifact provenance/signature、sandbox conformance、verifier registry/currentness/revocation、平台级 release actor 与 Provider-specific route actor 分权，以及 exact v222/v227 source companion。v227 永远不跨越该截止线。

## 7. 实现与验收门槛

源码批完成后最多只能标记 `implementation_uncompiled/implementation_unrun`，并明确实际摄入和运行测量仍为 0。本阶段只允许 rustfmt、差异/文档/源码尺寸门卫和独立静态审计，不得把未执行路径写成已验证事实。

未来独立运行验收至少覆盖：fresh migration、升级与重复迁移、两次重开；Store/API 成功与 401/403/409/413/415/422；exact replay 与并发；CAS existing exact/corrupt/symlink；各崩溃窗 fault injection；receipt 后 blob missing/corrupt。只有这些执行证据完成后才可升级为 `implementation_partially_verified`，且仍不得声称生产 Adapter 已验证或可执行。
