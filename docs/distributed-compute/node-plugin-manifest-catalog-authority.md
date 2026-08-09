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

one-shot registry现包含状态核、私有generic owner、进程寿命包装及私有file-custody。owner用token、session ID和route epoch三元身份原子封存authority-open custody、policy与state，注册前重验真实open intent current，已用nonce永久禁用；进程包装只能显式泄漏为`'static`，提供`ring::SystemRandom` nonce源、零值/碰撞有限重试、互斥路由和RAII callback lease。exact route同时负责authorizer判断及Bootstrap→SchemaMigration→Runtime线性降权，错误阶段转换失败关闭且不释放policy或custody。callback未排空时关闭失败，显式完成或作用域退出才归还exact session。file-custody把真实`PinnedManagedSqliteFile`、`PinnedManagedSqliteMainFile`或`PinnedManagedSqliteWalMainFile`与process owner、exact route、main/sidecar/SHM lease不可拆地持有；其受控操作门面要求每次offset I/O、main lock、SHM map/lock/barrier/unmap及close先取得该route的`Io`、`Shm`或`Close` callback lease，并且不暴露文件句柄。只有消费managed-fs线性关闭receipt后才能生成exact close proof；普通析构、物理关闭失败或角色错配都会先永久保留句柄和lease再隔离route，物理关闭成功后的route锁中毒、身份失效或状态拒绝也会永久保留exact lease与证明。通用文件receipt不能替代main锁域专用关闭，成功路径可观察Connection关闭并消费route-removal proof退休。registry共42项测试覆盖完整生命周期、token防复用、随机失败/碰撞、精确回调和句柄路由、authorizer路由及失败保管、自动归还、关闭证据错配、main关闭绕过拒绝、锁中毒、关闭证据保留、隔离顺序，以及真实Windows rollback main+Journal和WAL-main+SHM关闭、路由I/O/锁/SHM操作、main提升幂等/解绑边界、终端中毒绑定失败保管与临时目录清理；测试随机源及仅测试可达的SHM终端故障可注入。file-custody内部另有泛型窄ABI适配器，只把结果、竞争状态及仍由同一SHM connection保活的映射地址投影到ABI层，不开放底层文件句柄；当前没有生产process-owner实例、Connection或live `sqlite3_file`构造器。

私有SQLite ABI外壳现定义VFS v1与IO methods v2表；生产表没有可变或注册入口，`pNext/pAppData`为空，`xOpen`只对SQLite提供的fresh storage执行不读取旧值的初始化，清空`pMethods`、state和out-flags后返回`SQLITE_CANTOPEN`。私有raw-state协议只允许向已初始化空槽安装一个exact方法表和类型擦除envelope，以运行时`TypeId`门控闭包内可变借用及消费式typed take；拒绝安装会原样返还新状态，take先关闭callback入口再清空slot。具体ABI状态可持有上述file-custody适配器，I/O表已严格转换offset/length、sync flags、lock level、SHM flags及delete/extend布尔值，并将短读、竞争和各阶段失败映射为SQLite结果；`xClose`先消费raw state再调用线性close，状态错配、Rust panic或无返回通道的SHM barrier失败都会清除callback入口并触发类型擦除的单次Drop。9项定向测试包含raw-state协议、受控假文件extern回调及3项复用同一泛型file-custody适配器的真实端到端矩阵，覆盖rollback main+Journal、WAL-main+SHM、短读零填、写/size/sync、主锁、SHM、参数拒绝、联合close、错误阶段`xClose`后的单次清理与route隔离。测试层保留两个用途分离、均非默认的命名VFS：原有进程寿命transport VFS只委托SQLite默认VFS并统计成功`xOpen`；Windows独占受管VFS覆盖rollback与WAL，先把唯一namespace消费进SHM runtime，再把main、Journal/WAL sidecar的真实`xOpen`、I/O、主锁、access/delete、SHM和`xClose`接入exact registry route、file-custody及同路由authorizer。首次`xShmMap`在SHM callback lease内消费普通main custody、领取唯一SHM lease并原位安装WAL-main；失败绑定会先永久保管main与两类lease再隔离route。Connection关闭后联合消费main/SHM close proof、退休route、注销VFS并允许删除测试根目录；默认VFS只承担随机数、睡眠和时间，动态加载固定不可用。两者都不是生产注册入口。

现有legacy path facade虽已不可Clone，但尚未退役；`connect/with_deferred/with_immediate`仍可能建目录、按路径开库、切WAL或运行迁移。因此它们禁止用于planning，并必须在VFS启用前迁移到opened-authority内核或永久门禁。真正的VFS还必须拥有SQLite main、journal、WAL、SHM及相关临时对象的句柄生命周期，路径重开、canonicalize或open后FileId复核都不能替代这一能力。

## 仍不可达

当前仍没有可注册或可成功打开的生产handle-bound SQLite VFS、生产process-owner实例、从生产SQLite ABI进入registry的live route、持有私有Rust file-custody的生产live `sqlite3_file`或生产非空`pMethods`、生产authorizer/PRAGMA持续门卫、生产trusted-time/rollback provider、opened snapshot producer或consumer。真实managed-fs句柄与registry租约的不可拆托管状态、受callback lease约束的安全操作门面、raw-state所有权协议、具体ABI回调桥、测试专用真实file-custody回调矩阵和两种真实Connection夹具均已存在；Windows测试已证明rollback及WAL/SHM可经受管VFS进入registry/file-custody，但尚无生产构造适配器或生产注册所有权。三个Connection测试都安装route-backed authorizer，回读五项db_config和两个连接limit，并真实拒绝ATTACH、temp表、运行期PRAGMA、扩展加载、非白名单函数和DQS字符串；这些证据不能证明生产VFS接线。受管夹具仍不覆盖临时文件、并发多Connection、SHM map/lock/unmap与联合close平台故障注入或跨重启恢复；现有locking/SHM/close、关闭receipt适配、请求投影、owner/状态核、file-custody和ABI表仍不可生产到达，controller与namespace仍只接到不可打开的sealed intent，生产`open()`继续固定unavailable。

测试可达的受管VFS现已覆盖rollback与WAL：Windows独占命名VFS以真实SQLite Connection打开受管main、Journal/WAL sidecar，exact route同时驱动请求投影、file-custody、authorizer、阶段转换、main→WAL-main提升、SHM及关闭退休；两条路径均完成schema/runtime读写、危险SQL拒绝、显式Connection关闭、route custody恰好释放一次、VFS注销及根目录删除。SQLite专项筛选现为69项通过，其中真实Connection 3项；新增两项registry证据验证提升幂等至显式解绑、解绑后拒绝，以及终端中毒绑定失败时先永久保管main句柄、main lease与SHM lease再隔离exact route。下一批应补SHM map/lock/unmap与联合close的可注入平台失败矩阵，并评估测试可达的多Connection同namespace竞争；这些证据稳定后再审查生产process-owner、注册/注销所有权和opened-authority接线，不得把任一测试VFS直接提升为生产入口。v11仍固定`context_ready=false`、`snapshot_ready=false`，并保持root/authority、PlanApply、work-admission、下载、安装和Sidecar标志为false。非空库存还缺installed/promotion/signed-manifest provenance、work-admission generation、Ready/Attempt撤销及signed `reauthorize_existing`，不得由本合同推断为已完成。
