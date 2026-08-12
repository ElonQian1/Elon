---
title: 外部矿池 Adapter 惰性安装与撤销权威
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 惰性安装与撤销权威

## 1. 目的与真实性边界

V246 把一份当前 V244 采用授权、精确 V232 静态包回执和精确 V227 隔离 CAS 字节汇合成不可变的 `installed_inert` 安装事实。安装流程只从服务端内容寻址 CAS 的已复验文件句柄读取 ZIP，按签名证据链固定的 manifest 安全解包、逐文件复算长度与 SHA-256，并以不覆盖的原子发布形成内容寻址安装树；SQLite 回执只有在最终事务重新验证全部权威后才能落地。V247 在该事实之上追加不可变的安装撤销终态，并把“展示 currentness”与后续 consumer 才能取得的 sealed current authority 分开。

这是真实的“字节已安装”代码路径，但不是 Adapter 已运行。V246 不启动进程或 Sidecar，不执行 entrypoint，不解析或读取 credential locator，不联网下载，不探测外部矿池，不激活 Provider，不写 v213 Adapter、credential、route、seal 或 outbox，也不产生 ACK、任务执行、计量、结算或付款效果。Provider 继续保持 `registering`。

## 2. 精确来源与文件系统提交

安装目标只能由服务端从权威链派生，调用者只能提交采用回执、预期摘要、幂等键和显式 `installed_inert` 确认。最终事务逐项绑定：

- V244 adoption 的 admission、Provider、Adapter、release、配置、实现摘要和能力集；
- V232 package 的 package receipt、source receipt、archive SHA-256/长度、manifest 摘要、runtime 与 entrypoint；
- V227 source receipt 和同一份 CAS 对象；
- manifest 中完整、唯一且有序的普通文件集合、角色、SHA-256 与长度。

安装根固定在服务端数据目录的独立命名空间。实现以随机 staging 目录、`create_new` 普通文件、逐条流式摘要、文件与目录同步以及不覆盖的最终发布工作。绝对路径、反斜杠、`.`/`..`、重复或大小写冲突、目录条目、符号链接、reparse/hardlink、加密条目、未知压缩算法、额外或缺失文件均失败关闭。已有相同内容地址的目标只能在全量复验通过后复用，绝不覆盖或信任目录名。

文件系统先完成、SQLite 后提交。若发布前失败或并发落败，随机 `.part` 目录会作为无权威引用的惰性 orphan 留待后续独立清理；安装路径不会在可能发生目录替换竞争时递归删除它。若数据库提交失败，已发布目录同样只是没有权威引用的惰性内容树；重试必须重新复验后才能复用。目录存在本身不能替代 V246 回执，V246 回执也不能替代后续启动时的文件树复验。

## 3. 单一检查时间、终态与当前性

Store 在最终 `BEGIN IMMEDIATE` 事务内只生成一次规范 UTC 纳秒 `checked_at`。同一值用于：

- 重新取得并审计精确 V244 current authority；
- 严格判断 `checked_at < sandbox_report_expires_at` 与 `checked_at < credential_report_expires_at`，等于到期点即失效；
- 绑定 V246 `installed_at` 与 `recorded_at`。

V247 currentness 只在安装回执、文件清单、V244/V232/V227 精确根仍一致、没有安装 terminal，且安装树再次全量复验时可供后续同连接 consumer 使用。sealed authority 同时持有已审计回执、同一份已复验文件树能力和唯一 `checked_at`；未来 consumer 不能用 ID、摘要或 SQL 行重建它。SQL current view 和管理 GET 仅供展示，不能单独铸造 sealed authority。

公开 `current_status` 为兼容既有接口继续只使用 `installed_upstreams_current` 或 `historical_only`；新增 `terminal_status=none|revoked`，并可返回脱敏 terminal summary。上游撤销、到期、摘要漂移、磁盘树漂移或安装 terminal 都使安装不可用于后续 consumer；历史安装与 terminal 回执仍可读取和审计。

撤销只要求精确 installation ID/digest、认证管理员、原因、幂等键和显式确认；它不要求上游仍当前，也不复验或删除安装树。因此在采用授权到期或撤销后，管理员仍能终止遗留安装权威。terminal 是追加式单终态，精确重放返回同一回执，改变历史或第二次撤销失败关闭。

## 4. 回执与接口

管理接口仅对平台 `admin` 或 `owner` 开放：

- `POST /api/admin/compute/external-pool-adapter-installations`
- `POST /api/admin/compute/external-pool-adapter-installations/:installation_id/revoke`
- `GET /api/admin/compute/external-pool-adapter-installations/:installation_id/currentness`

回执公开安装 ID/摘要、Provider/Adapter/release/config 身份、archive/manifest/tree 摘要、文件数、总字节、runtime kind、entrypoint path digest、安装时间和固定效果。响应不返回服务端路径、ZIP/文件正文、entrypoint 原文、candidate reference、credential locator 或 commitment、签名、公钥、幂等材料或 confirmation。

`installation_effect=adapter_bytes_installed_inert`；`provider_effect`、`credential_effect`、`route_effect`、`execution_effect` 与 `settlement_effect` 均为 `none`。撤销 terminal 固定 `terminal_kind=revoked`、`installation_effect=installed_instance_revoked`，其余五项效果仍为 `none`；它不删除内容寻址安装树。同一 adoption 只能形成一份安装回执；同一全局 release 可以被不同 Provider 的独立 adoption 安装，不产生全局 v213 Adapter revision 冲突。

V247 migration 新增 `compute_external_pool_adapter_installation_terminal_receipts`，canonical schema 固定为 `compute_federation.external_pool_adapter_installation_terminal_receipt.v1`。触发器拒绝 update、delete、replace、非精确 installation root 和 JSON/标量投影漂移；既有 current view 保留原有列与状态词，只在末尾追加 `terminal_status`。这些 SQL 对象提供持久化与展示约束，不绕过 Store canonical audit。

## 5. 后继与禁线

V247 不新增独立 activation plan，也不直接激活 external-pool Provider。现有 `active` Provider 已被 Offer/Job/quote 路径当作可调度、可出售资格；而 V221 external-pool Provider 只有登记态 Adapter 身份，V243-V247 又绑定精确 `registering` Provider revision。若先单独激活，会制造可出售但没有 v213 route、credential 和真实 service actor 的窗口，并立即令前置 current authority 历史化。因此直接 activation 是 NO-GO。

未来 activation 必须在同一 `BEGIN IMMEDIATE` 事务中消费 V247 sealed current authority，建立精确 service actor、Adapter/credential/route/capability/seal 与 Provider-version/installed-instance companion binding，并最后推进 Provider。在此之前必须先用 provider-neutral Adapter registry v2 或等价 companion 修正当前 v213 全局 release actor 与 Provider-specific actor 的域耦合，不能把 external-pool 直接写进现有 v213 v1。不能把 installer、Provider owner、管理员或 credential verifier 冒充 dispatch service actor；任何纸面 activation plan 都不能替代这笔原子事务。

真实 Sidecar host、credential resolver/KMS/gateway、网络 transport、authenticated ACK/event、Runner、可信计量、市场交割和结算仍是独立后续纵切面。节点 Plugin Host 和 CLI Sidecar 属不同安全域，不能作为 V246 已拥有 external-pool runtime 的证据。

## 6. 威胁模型

V246/V247 防止普通 API/Store 调用省略采用、包、CAS、文件清单或 terminal 权威，也防止 SQLite 更新、删除、替换和列/JSON 漂移。它不声称抵抗拥有任意数据库和文件系统写权限的主机管理员，也不提供密码学上的进程隔离、运行证明或远端矿池身份证明。生产安装、权限/ACL、备份恢复、并发崩溃恢复和真实恶意 ZIP 仍须单独运行验收。
