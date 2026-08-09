# 节点插件目录 Authority 与回滚 V2

本文只定义节点本机 Manifest catalog binding、catalog-aware rollback checkpoint V2，以及未来 handle-bound authority 打开能力的安全边界。共享策略、下载、候选清理和 Ready/Attempt 分别由同目录其它文档维护。

## 当前已铺设合同

本机 LocalAuthority `user_version=6` 新增 append-only Manifest catalog binding。目录 revision 必须为正且单调；首个 binding 可用 Control-signed envelope 为旧 scalar 补足内容证明，后继必须严格前进。Store 只接受空库存、无 `owned/cleanup_pending` candidate owner、无 prepared fetch/verification 的 authority。

一次 binding 在同一 `BEGIN IMMEDIATE` 中完成：

- 从当前 root-revalidated keyring 读取精确 Publisher/Control ring binding；
- 逐个重验完整 Publisher-signed Manifest 来源，并按 release identity 规范排序、拒绝重复；
- 使用当前 Control ring 与独立 `ELON-COMPUTE-PLUGIN-MANIFEST-CATALOG-V1` 域验证精确目录 payload；
- 将 catalog JSON、Control envelope及摘要、signing key/fingerprint、完整 source envelopes/set digest和 binding receipt逐列封存、回读；
- 原子推进 state revision、authority epoch与可信时间，不推进 inventory revision。

catalog binding 的 trusted-time observation 带60秒进程内单调有效期，写入、签名复核后、INSERT前、事务返回前、commit后和Durable recovery边界均会重检。当前能力只证明本机目录头，不授予InstallPlan、下载、安装或任务执行。

## Rollback checkpoint V2

V2 checkpoint与V1使用独立 schema、challenge、attestation、签名域、assessment、permit与witness，二者不能互转。V2额外绑定 catalog revision、catalog digest和binding receipt digest；相同revision下任一摘要分叉均失败关闭。witness摘要绑定anchor ID、sequence、checkpoint digest、attestation digest与signing-key fingerprint。

`assess_rollback_anchor_v2`只接受不可Clone、不可序列化、字段私有的本机checkpoint custody；裸远端attestation checkpoint不能冒充本机输入。本批故意不提供该custody的构造器。未来producer必须在同一个already-opened authority事务中重验meta、当前catalog head、完整receipt与live process fence后铸造；permit consumer还必须在同一authority/root/process custody下按checkpoint digest重读current head，或持续持有线性custody。

## Opened authority 边界

`OpenedComputePluginLocalAuthority`只定义“SQLite连接先于root lock和NodeAgent instance lock析构”的线性持有合同。当前open intent没有构造入口，生产`open()`固定返回`COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。未来只能由Bootstrap把同一NodeDataPaths/root、authority instance和真实NodeAgent instance-lock见证原子绑定，调用方不得任意拼装lease。

现有legacy path facade虽已不可Clone，但尚未退役；`connect/with_deferred/with_immediate`仍可能建目录、按路径开库、切WAL或运行迁移。因此它们禁止用于planning，并必须在VFS启用前迁移到opened-authority内核或永久门禁。真正的VFS还必须拥有SQLite main、journal、WAL、SHM及相关临时对象的句柄生命周期，路径重开、canonicalize或open后FileId复核都不能替代这一能力。

## 仍不可达

当前没有handle-bound SQLite VFS、Bootstrap联合root/instance-lock见证、生产trusted-time/rollback provider、opened snapshot producer或consumer。v11仍固定`context_ready=false`、`snapshot_ready=false`，并保持root/authority、PlanApply、work-admission、下载、安装和Sidecar标志为false。非空库存还缺installed/promotion/signed-manifest provenance、work-admission generation、Ready/Attempt撤销及signed `reauthorize_existing`，不得由本合同推断为已完成。
