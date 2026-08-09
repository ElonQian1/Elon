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

`OpenedComputePluginLocalAuthority`只定义“SQLite连接先于controller custody析构”的线性持有合同；custody再按终态撤销、namespace、root lock、NodeAgent instance lock的顺序释放。open intent已有仅消费sealed controller custody的内部构造核，但没有Host调用或生产者，生产`open()`固定返回`COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。调用方不得任意拼装lease或裸namespace。

Bootstrap现已增加独立于共享策略代次的authority-controller。生产instance lock只能从配置的node state路径获取；marker保存不保活的Weak身份，并在begin、finalize和转换时把当前witness与保留lease按同一Arc句柄逐项闭合。controller在状态锁内一次性移出dormant authority locator，锁外只做existing-root pin，随后回锁按Bootstrap实例、账号、installation、NodeDataPaths、authority path、nonce与controller epoch复核。凭据替换、数据根变化、instance witness失效、poison或Bootstrap析构都会终态撤销，失败不会恢复成Dormant。

controller现在可以线性转换为不可拆的open-intent custody：root lock会保存首次锁定目录的完整managed object binding，转换时从同一pinned root父句柄重pin `compute-plugin`，只有名称、文件身份摘要和父身份摘要全部相等才封存SQLite namespace。intent直接持有controller与sealed namespace，不接受调用方提供的directory、namespace、root/instance lease、witness或摘要；析构先发布controller终态，再释放namespace、root lock与instance lock。当前仍没有Host调用点，`open()`固定失败，也没有opened authority、process fence或Store权限。

managed-fs现已铺设sealed SQLite namespace内核：只接受already-pinned且未发生目录创建的父句柄，只能按枚举访问`compute-plugin-state.sqlite3`及其`-journal`、`-wal`、`-shm`四个单组件；Windows打开、access、delete均为父句柄相对操作，并保留identity复验、offset I/O、短读零填、truncate、full sync、delete后absence观察和可选父目录barrier。main数据库primitive按FileId共享进程内domain，精确记录Reserved/Pending/Shared/Exclusive事实并执行Windows固定字节区间锁；WAL SHM primitive按目录FileId唯一签发，覆盖DMS、8槽local mask与OS锁、固定region预算、稳定映射、SeqCst barrier、exact-range unlock、Main-EXCLUSIVE借用绑定的typed delete gate及失败后的永久进程tombstone。非Windows固定失败关闭；这些源码尚未编译或测试，也没有VFS ABI调用点，不能独立代表数据库权限。

现有legacy path facade虽已不可Clone，但尚未退役；`connect/with_deferred/with_immediate`仍可能建目录、按路径开库、切WAL或运行迁移。因此它们禁止用于planning，并必须在VFS启用前迁移到opened-authority内核或永久门禁。真正的VFS还必须拥有SQLite main、journal、WAL、SHM及相关临时对象的句柄生命周期，路径重开、canonicalize或open后FileId复核都不能替代这一能力。

## 仍不可达

当前仍没有可注册的handle-bound SQLite VFS、`sqlite3_io_methods`及其panic/错误码适配、one-shot token registry、可报告且保留失败custody的xClose、临时文件与URI/ATTACH策略、生产trusted-time/rollback provider、opened snapshot producer或consumer。现有locking/SHM只是一组不可达managed-fs primitives，不是可用VFS；controller与namespace仍只接到不可打开的sealed intent，生产`open()`继续固定unavailable。下一批必须先补temp/URI/ATTACH与explicit-close合同，再铺惰性ABI/token callbacks，最终才能单独注册非默认VFS并打开SQLite。v11仍固定`context_ready=false`、`snapshot_ready=false`，并保持root/authority、PlanApply、work-admission、下载、安装和Sidecar标志为false。非空库存还缺installed/promotion/signed-manifest provenance、work-admission generation、Ready/Attempt撤销及signed `reauthorize_existing`，不得由本合同推断为已完成。
