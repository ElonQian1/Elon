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

当前专项验证包括三层：3 项内存 SQLite 事务测试证明收据与 authority 头原子提交、失败零残留和收据不可变；7 项恢复判定测试覆盖全字段前态、身份碰撞、当前/历史提交、合法后继、回滚与缺失 head 回执失败关闭；5 项真实 Ed25519 候选测试证明独立 Publisher/Control 签名并拒绝签名域、目标及公钥角色复用。测试密钥只在进程内生成；外层 `bind/adopt` 生产会话、root/keyring、Signer/KMS、磁盘 VFS 与 Host 接线仍未覆盖。

## Rollback checkpoint V2

V2 checkpoint与V1使用独立 schema、challenge、attestation、签名域、assessment、permit与witness，二者不能互转。V2额外绑定 catalog revision、catalog digest和binding receipt digest；相同revision下任一摘要分叉均失败关闭。witness摘要绑定anchor ID、sequence、checkpoint digest、attestation digest与signing-key fingerprint。

`assess_rollback_anchor_v2`只接受不可Clone、不可序列化、字段私有的本机checkpoint custody；裸远端attestation checkpoint不能冒充本机输入。本批故意不提供该custody的构造器。未来producer必须在同一个already-opened authority事务中重验meta、当前catalog head、完整receipt与live process fence后铸造；permit consumer还必须在同一authority/root/process custody下按checkpoint digest重读current head，或持续持有线性custody。

## Opened authority 边界

`OpenedComputePluginLocalAuthority`只定义“SQLite连接先于controller custody析构”的线性持有合同；custody再按终态撤销、namespace、root lock、NodeAgent instance lock的顺序释放。open intent已有仅消费sealed controller custody的内部构造核，但没有Host调用或生产者，生产`open()`固定返回`COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE`。调用方不得任意拼装lease或裸namespace。

Bootstrap现已增加独立于共享策略代次的authority-controller。生产instance lock只能从配置的node state路径获取；marker保存不保活的Weak身份，并在begin、finalize和转换时把当前witness与保留lease按同一Arc句柄逐项闭合。controller在状态锁内一次性移出dormant authority locator，锁外只做existing-root pin，随后回锁按Bootstrap实例、账号、installation、NodeDataPaths、authority path、nonce与controller epoch复核。凭据替换、数据根变化、instance witness失效、poison或Bootstrap析构都会终态撤销，失败不会恢复成Dormant。

controller现在可以线性转换为不可拆的open-intent custody：root lock会保存首次锁定目录的完整managed object binding，转换时从同一pinned root父句柄重pin `compute-plugin`，只有名称、文件身份摘要和父身份摘要全部相等才封存SQLite namespace。intent直接持有controller与sealed namespace，不接受调用方提供的directory、namespace、root/instance lease、witness或摘要；析构先发布controller终态，再释放namespace、root lock与instance lock。当前仍没有Host调用点，`open()`固定失败，也没有opened authority、process fence或Store权限。

managed-fs现已铺设sealed SQLite namespace内核：只接受already-pinned且未发生目录创建的父句柄，只能按枚举访问`compute-plugin-state.sqlite3`及其`-journal`、`-wal`、`-shm`四个单组件；Windows打开、access、delete均为父句柄相对操作，并保留identity复验、offset I/O、短读零填、truncate、full sync、delete后absence观察和可选父目录barrier。main数据库primitive按FileId共享进程内domain，精确记录Reserved/Pending/Shared/Exclusive事实并执行Windows固定字节区间锁；WAL SHM primitive按目录FileId唯一签发，覆盖DMS、8槽local mask与OS锁、固定region预算、稳定映射、SeqCst barrier、exact-range unlock、Main-EXCLUSIVE借用绑定的typed delete gate及失败后的永久进程tombstone。

普通、被拒、main及WAL-main句柄现在都有消费式显式关闭合同；SHM teardown也在释放view、mapping和DMS后显式关闭exact SHM句柄。成功返回不可复制的线性receipt；关闭未尝试时失败值保留live custody，Windows `CloseHandle`已调用但失败时只保留不可重试的terminal raw-handle quarantine，锁或SHM结果不确定时还永久保活对应FileId/domain tombstone。短生命周期access/delete/absence观察与SHM初始化失败也必须消费关闭结果或把失败custody留在typed failure/coordinator，不能把Rust `Drop`当成xClose成功证明。相关源码已随登记簿专项目标完成编译，但尚无live VFS/OS故障矩阵验证，也不能独立代表数据库权限。

本机另有一个完全惰性的SQLite安全策略内核：one-shot nonce只能投影成opaque主逻辑名及exact `-journal`/`-wal` 名；root `sqlite3_open_v2` flags与bundled SQLite 3.45实际传给VFS的main、WAL、hot-journal xOpen矩阵分别校验；NULL/temp、URI、memory、shared-cache、delete-on-close及未知对象固定拒绝。authorizer按Bootstrap、SchemaMigration、Runtime线性降权，固定启动PRAGMA及读回，拒绝ATTACH/DETACH、temp/virtual schema和未知action，Runtime不允许DDL且函数只走小白名单。15项纯策略测试覆盖逻辑名、root/xOpen精确flags、三阶段降权、函数白名单、raw action/UTF-8/参数形状，以及xAccess/xDelete/xFullPathname矩阵。

该策略新增的安全ABI投影只接受未来raw边界已转换出的借用字节：unknown action、非法UTF-8或不符合bundled 3.45的NULL/参数形状直接拒绝；ALTER的effective database取精确arg1，DROP COLUMN的arg3只作列名；transaction/savepoint只接受固定操作词。VFS请求投影只允许exact sidecar的`SQLITE_ACCESS_EXISTS`、Journal/WAL删除矩阵和Main opaque逻辑名原样full-path输出。投影层没有raw pointer、`extern`、Connection、文件系统或注册调用，不能自行执行SQLite操作。

one-shot registry现包含状态核、私有generic owner、进程寿命包装及私有file-custody。owner用token、session ID和route epoch三元身份原子封存authority-open custody、policy与state，注册前重验真实open intent current，已用nonce永久禁用；进程包装只能显式泄漏为`'static`，提供`ring::SystemRandom` nonce源、零值/碰撞有限重试、互斥路由和RAII callback lease。callback未排空时关闭失败，显式完成或作用域退出才归还exact session。file-custody把真实`PinnedManagedSqliteFile`、`PinnedManagedSqliteMainFile`或`PinnedManagedSqliteWalMainFile`与process owner、exact route、main/sidecar/SHM lease不可拆地持有；只有消费managed-fs线性关闭receipt后才能生成exact close proof，普通析构、物理关闭失败或角色错配都会先永久保留句柄和lease再隔离route。通用文件receipt不能替代main锁域专用关闭，成功路径可观察Connection关闭并消费route-removal proof退休。状态10项、owner 7项、process-owner 13项及file-custody 2项共32项测试覆盖完整生命周期、token防复用、随机失败/碰撞、精确回调和句柄路由、自动归还、关闭证据错配、main关闭绕过拒绝、锁中毒、隔离保留顺序，以及真实Windows rollback main+Journal和WAL-main+SHM关闭及临时目录清理；测试随机源可注入，当前没有生产process-owner实例、Connection、live `sqlite3_file`或SQLite ABI callback接线。

私有SQLite ABI外壳现定义VFS v1与IO methods v2表；表没有可变或注册入口，`pNext/pAppData`为空，`xOpen`总是先清`pMethods`、state和out-flags再返回`SQLITE_CANTOPEN`。其余I/O、lock、SHM和动态加载回调统一初始化out参数、捕获Rust unwind并返回保守错误，IO表从未安装到任何`sqlite3_file`。这只证明失败关闭ABI形状，不是live callback、registry或数据库打开能力。

现有legacy path facade虽已不可Clone，但尚未退役；`connect/with_deferred/with_immediate`仍可能建目录、按路径开库、切WAL或运行迁移。因此它们禁止用于planning，并必须在VFS启用前迁移到opened-authority内核或永久门禁。真正的VFS还必须拥有SQLite main、journal、WAL、SHM及相关临时对象的句柄生命周期，路径重开、canonicalize或open后FileId复核都不能替代这一能力。

## 仍不可达

当前仍没有可注册或可成功打开的handle-bound SQLite VFS、生产process-owner实例、从SQLite ABI进入registry的live route、持有私有Rust file-custody的live `sqlite3_file`或非空`pMethods`、从live xClose故障值进入terminal custody的callback接线、authorizer/PRAGMA安装与持续门卫、生产trusted-time/rollback provider、opened snapshot producer或consumer。真实managed-fs句柄与registry租约的不可拆托管状态已经存在并通过Windows测试，但惰性ABI没有构造、安装、取回或清空它的路径。临时文件仍不实现；URI、ATTACH/DETACH等只在纯策略中定义拒绝，尚未接入SQLite执行面。现有locking/SHM/close、关闭receipt适配、请求投影、owner/状态核、file-custody和ABI表都不可达，controller与namespace仍只接到不可打开的sealed intent，生产`open()`继续固定unavailable。

下一批必须为私有file-custody增加受exact callback lease约束的I/O、lock和SHM操作入口，并定义raw ABI state指针唯一的安装、借用、取回与清空协议；失败关闭值仍须通过现有先保留后隔离路径进入process owner。之后才能在同一Connection生命周期安装authorizer、关闭extension loading并设置/回读DEFENSIVE、TRUSTED_SCHEMA、DQS、ATTACHED和worker限制。完成真实回调故障矩阵审计后才能另批创建生产process-owner、注册非默认VFS并打开SQLite；不能把当前私有失败表直接注册。v11仍固定`context_ready=false`、`snapshot_ready=false`，并保持root/authority、PlanApply、work-admission、下载、安装和Sidecar标志为false。非空库存还缺installed/promotion/signed-manifest provenance、work-admission generation、Ready/Attempt撤销及signed `reauthorize_existing`，不得由本合同推断为已完成。
