# Windows 节点升级兼容与事故处置

最后更新：2026-08-04

本文是 Windows EXE 节点升级时按需读取的兼容门禁。项目数据架构合同见 `docs/pc-node-data-root.md`；发布命令引用 Git/发布手册，不在这里重复。

## Desktop review v3 升级边界

- Windows NodeAgent 默认自动启动 `desktop_review_broker_v1`：RSA-3072 私钥仅驻留当前节点进程内存，更新/重启自动轮换；安装包、`node-agent.env`、诊断包和 helper 均不得出现私钥。broker 只为真实 pipe client PID 能回溯到受保护 Codex Desktop 包且不经过 Elon executor 的进程链签名，同 SID executor 直接调用也必须 fail-closed。
- `/api/status.desktop_review_broker` 只报告非秘密 pipe 名、可用性和隔离策略。helper 协商到 broker 后无需 StateRoot 或环境变量；broker 连接/身份校验失败不得降级成 PC operator review 或旧共享凭据。
- 发布包与安装更新只分发 signer、公钥和 nonce 账本路径等非秘密配置；不得复制 Desktop 私钥或 Desktop `StateRoot`。
- 凭据 `Commit` 默认写入 v3 公钥集合、`ELON_DESKTOP_REVIEW_NONCE_LEDGER` 与 `ELON_DESKTOP_REVIEW_ALLOW_V2=0`。轮换窗口同时保留新旧公钥；确认 Desktop helper 全部升级后移除旧公钥。
- v2 兼容必须由运维显式设置 `ELON_DESKTOP_REVIEW_ALLOW_V2=1`，且不会重新启用共享 secret v1。v3 公钥存在时任何 v1 ticket 都拒绝。
- 更新保留已有 `node-agent.env` 和 nonce 账本，不创建生产凭据、不替换 Desktop 身份，也不自动重启 Desktop。

## 用户能听懂的一句话

新版的数据根是帮 AI 整理以后新产生的大文件，不是把旧项目判为不合格。升级必须记住原项目和原缓存；自动整理失败时，原项目仍应照常运行。

## 本次问题溯源

2026-07-13 由 Git 作者“一龙ai助手”提交的数据根治理逐步引入了硬门禁：`9bc1e68b` 建立统一数据目录，`0d5ed675` 接入容量治理，`5414f312` 拒绝旧节点绕过，`9d8adcf0` 阻止工作区与存储回落，`10bc5d34` 锁定 target 并预留容量。概念本身有价值，事故原因是把“新托管数据的推荐标准”错误扩大成了“既有项目的运行前提”，同时没有用旧版本配置与共享缓存 fixture 验证升级兼容。

后续复盘以提交和行为证据为准，不以作者归责代替系统修复；兼容分类、自动降级、回归 fixture 和发布门禁必须一起落地。

## 升级不变量

- Desktop review v2 的 NodeAgent 持久化内容只能是 InstallRoot 下 `node-agent.env` 的公钥；不可导出私钥与显式 StateRoot 由独立 Desktop 身份持有，不属于节点安装或更新包。更新必须保留公钥行，不得复制 Desktop StateRoot、私钥、放宽 ACL 或把它写入诊断包。旧版本不支持 v2 时 review fail-closed，已有任务、journal 与恢复检查点继续保留。

1. 保留 `install_id`、凭证、登录状态、本地授权、任务 journal、项目绑定、持久化路径和已观察缓存目录。
2. 新增字段必须带 schema 默认值、幂等迁移和兼容降级；旧配置缺字段不等于配置错误。
3. 已验证可运行的外部项目继续使用原 cwd、环境和缓存，普通写任务与构建也不因新数据根缺失而阻断。
4. 只有新建平台托管项目可以选择推荐数据根；推荐容量和项目数量只告警。
5. 自动准备遵守“先校验、再持久化、最后发布内存状态”；失败保留旧状态并继续外部项目。
6. 外部项目、共享缓存和 Git 现场不得被自动认领、移动、清理或改写。
7. 服务器按任务类型判断能力：已有项目 CLI/Exec 保持向后兼容；只有显式新建/清理托管 workspace 协议可要求新 capability。
8. 更新或重启不得静默丢失活跃本机任务：先持久化检查点；检查点完整且没有待审批、未完成不可重复操作时，允许节点重启并自动重连，不要求活动执行器先退出。新进程把 recovery v1 回执、restart checkpoint 和任务 journal 按单调状态合并；安全任务必须自动回到 `running`，继续增长原 journal 游标并正常终止，不需要手动 Resume。启动关键路径只同步处理当前版本恢复事务，或仍有真实进程、可验证进程身份、新鲜心跳的 sidecar；没有执行所有权的旧版本票据和历史 sidecar 必须在节点心跳、云连接和管理端点恢复后后台收敛，不得因逐条 Git 指纹或 journal 补账造成升级后假死。restart checkpoint 在启动后还必须按真实执行句柄、进程身份和新鲜恢复心跳重新核对：无执行所有权的旧 `running` 标签转入可恢复历史，不再占用当前更新的 `active_task_ids` 或继续显示阻塞性的 `resume_required`。只有缺少恢复证据且仍有真实执行所有权，或身份、租约、工作区、恢复事务无法闭合时才显示 `resume_required` 和一键 Resume；已经确认的 `resumed` 或任务终态不得被旧 checkpoint 回写覆盖。检查点、URL 缓存与状态 API 都不得持久化 admin token。
9. 受监督 worktree 的唯一权威 Git lease 是 `elon-supervision:<root_task_id>`：prepare、恢复、任务运行、合并和发布都不得用通用锁抢占或删除。`done`、`failed`、`canceled` 等可信终态在执行句柄退出后幂等释放精确匹配的 root lease；`cancel_requested` 还必须证明取消副作用已持久化且无 live prompt/sidecar。启动与周期维护只回收满足同样终态证据且 lease identity 精确匹配的陈旧锁，运行任务、通用锁、陌生 lease 和脏现场内容均不触碰；Desktop accepted review 保留为兼容的幂等释放入口，但不再是唯一入口。历史节点若把 lease 写成监督后代任务 ID，只能在当前节点/安装/owner/项目/基础工作区一致、持久契约逐代回溯到同一 `requirement` 根且现场无活跃 prompt/sidecar 时迁移一次；迁移必须持有跨进程 Resume 准入锁并原子替换 Git `locked` 文件，绝不经过 unlock/relock。通用锁、陌生任务 lease、断裂或身份不明谱系继续 fail-closed。超时或取消先同步回收执行器后代进程。可信且独占、Git 注册仍有效的脏 worktree 可原位 Resume，staged、unstaged、untracked 全部保留；resume-of-resume 只有在父任务为当前协议的 `resume_original`、父子 root identity 一致且继承分支/路径/Git 身份仍吻合时才复用原现场。若目录保留但 Git 注册丢失，不得移动、删除、覆盖或原位修补；仅在记录 common-dir/remote/path/branch、分支 HEAD 后继且已进主线、目录零差异及所有同根 registry/lease 均终态可回收时，新建 conversation worktree、迁移根 provenance 并收敛为唯一 root lease。
10. 多代同 root 续跑必须继承同一份已验证的基础仓库、平台路径、分支、Git 身份和 root lease。旧项目别名先映射为节点当前项目绑定再做授权比较；映射只消除历史命名差异，不能放宽跨项目、跨 root、身份漂移、脏而不可信或并发占用拒绝。
11. `post_task_improvement` 必须与用户任务解耦：父任务先终止并释放执行资源，改进进入独立低优先 task/conversation/worktree；实际执行 worktree/branch 必须在派发前验证并持久化。前台、发布、更新或构建压力出现时自动 pause/yield，门禁解除后自动 resume；配额类失败自动退避重试，完成后等待带来源的 Desktop 审查。
12. 取消事件向后兼容地保留 `requested_by`、`source`、`reason`、`requested_at_ms` 四元组；系统中断另带可空的可信枚举 `interruption_source`。sidecar/journal 审计必须先于取消且写失败时拒绝取消；升级读取旧记录不得因缺少新字段而失败。
13. 三端发布共享同一个 `batch_id + immutable SHA`；server、PC 前端和 Windows 节点阶段必须有持久 heartbeat/attempt/owner/error 记录。损坏 ledger、未知阶段状态、过期 owner 或批次 SHA 漂移都 fail-closed；崩溃后的同批次接管幂等且不能破坏 FIFO 防饿死。
14. `node.json` 的 `install_id` 同时锚定真机唯一调试包 `com.elon.app.uituner_<节点指纹>`；首次升级可幂等补写 `debug_package_fingerprint`，后续更新和重启必须保持一致。已有指纹与当前安装身份不符时拒绝调试部署并保留原状态，不能换用 `.uitest`、`.uitest_anim` 或新随机身份创建第二套 Launcher 包，也不能自动卸载手机应用。
15. Debug Runtime source proof 必须同时记录固定集成槽 generation、已部署 integration Git revision、原业务 worktree Git revision、generation 内容指纹、原业务 workspace 内容指纹和 runtimeBuildId。创建证明时先核对项目、物理设备、包、期望/已安装代次与 `DEPLOYED` 状态；FitRun 复用时再回查当前槽状态、generation worktree HEAD/内容指纹和原业务 HEAD。任一身份漂移或非零 Patch 都 fail-closed，不能要求两个因 HEAD 不同而必然不同的 workspace fingerprint 直接相等。
16. 显式隔离模拟器包可以复用零 Patch 的新鲜 Runtime source proof，但只在 generation、installed generation、integration revision、generation HEAD/内容指纹、原业务 Git/workspace revision、包和 runtimeBuildId 全部一致时允许 FitRun `ACCEPT_BEST`；这不放宽真机固定包、签名、物理设备或来源 worktree 校验。
17. `CROSS_PLATFORM_STYLE_WRITEBACK` 支持 `NO_WEB_COUNTERPART` 正式分支：仍需真实 Android 工件、当前 source revision、源码写回和无 Patch 构建；Web 侧改为扫描调用者声明的仓库内 Android 跟踪来源、Web 跟踪源码根和在 Android 来源中实际出现的搜索词。只有检查到 Web 跟踪文件且零匹配时通过；若存在匹配、证据越界、非跟踪来源或空扫描则拒绝，禁止用伪造 Web 截图代替。
18. 普通视觉和设备分类任务默认直接使用 PWA 或明确的隔离模拟器槽，不探测物理设备。只有用户反馈修改结果不正确或明确要求真机复核时才启用 `realDeviceRequired`；同一 MCP 会话只做一次最长 30 秒准备，离线、占用、遮挡、授权/安装确认、Runtime 失败或超时立即延期且禁止回退冒充。同一节点按设备 serial 独占写入，package 后缀不能绕过；未绑定明确 Runtime 身份时不能只按 projectRoot 猜测。Node-local operation lease 只能作为本机阶段证据，不能冒充跨 PC 全局 lease。
19. 后台多端设计必须显式能力协商：旧节点没有 `ui_get_design_capabilities` 时，现有 CLI/Exec 和旧 UI 工具继续兼容，新持久浏览器、草稿预览、源码绑定候选、Tauri 行为和验证矩阵显示“节点待升级”并失败关闭；不得从仓库版本、PC 前端版本或工具文档推断节点已经安装。v1.12 节点本机回归比较器必须回读 `runtimeSchema=yilong-ui-live@1.12.0` 和 `NODE_LOCAL_REGRESSION_COMPARATOR`；旧节点仍可使用外部比较器的完成契约，不得冒充本机已执行。更新不迁移、删除或跨项目复用 `.elon/ui-tuner/headless-design` 会话、草稿、工件或 fixture。预览只改变当前页面且可恢复，源码候选只能在 session 声明的项目源码根内产生并保持 `CANDIDATE`，升级不得替用户自动确认 `BOUND`。
20. 项目记忆 Hook 与双身份证据必须向后兼容：旧 SQLite 候选缺少 `git_identity`、provenance、conflicts 或 ingest action 时按空值读取并继续用工作区 SHA-256；旧节点没有 `/api/project-docs/native-context/health` 时 PC 收件箱继续加载候选，只隐藏 Memory CI。升级不得导入、复制或删除 `~/.codex/memories`，不得把聊天、prompt、transcript、命令或工具输出写入候选；Hook 临时路径账本不属于节点持久数据或 Git 项目。新节点只能通过 Codex 正式非托管 Hook 信任执行，不能在升级时替用户写入信任记录或绕过 Hook trust。receipt/context/governance 会话 profile 必须与 token 固定，旧 URL 不能通过查询参数提升权限。
21. 项目记忆生命周期与 Memory CI 必须向后兼容：旧 manifest/建议/SQLite 候选缺少 `owner`、`scope`、`review` 或 `expires_at` 时使用 serde 默认值继续读取，规范化时空 scope 降级为 repository；不得因为升级而批量改写旧 manifest。缺失治理字段只由 advisory health 标记，`fail_on_drift` 只对证据漂移和过期给出失败建议，只有调用方显式选择 `strict` 才把复核逾期、治理缺口或共享事实冲突纳入失败。旧 health 响应缺少 v2 计数、repair plan、capabilities 或 policy outcome 时 PC 按零值/隐藏处理，候选审核仍可用。
22. 本机发布激活器只能把由正式安装路径 `LocalAppData\ElonNode\一龙开发平台.exe` 持有的 loopback Admin listener 作为安装节点；读取 `/api/status` 后必须再次确认同端口仍由同一 PID 和安装路径持有。调试节点、隔离候选、release fixture 或其它端口即使回报目标 `release_identity`，也不得把正式发布状态推进为 `activated`，不得跳过 repair、精确 health check 或 rollback。正式 listener 缺失、所有权变化或路径无法读取时保持 `status_unavailable` 并失败关闭。

2026-08-05 正式发布复验已覆盖本条合同：7800 候选节点在线且回报目标身份时，激活帮助程序仍选中 7799 正式安装 listener；关闭桌面壳后发布从 `0.3.69+067ec391…` 激活到 `0.3.69+e30818e6…`，状态为 `activated`、`rollback_state=not_required`。重开桌面壳后只有一个安装目录 `elon-desktop.exe`，7799 仍由安装目录 `一龙开发平台.exe` 持有并保持目标 `build_git_sha`、`release_identity` 和云连接。

## 配置与缓存迁移合同

升级读取旧版 `node.json` 时：

1. 未知字段保留，缺失新字段使用兼容默认值；迁移成功后以同目录临时文件和原子替换写回。
2. 缺少推荐数据根时，可从已绑定项目同盘位置选择独立目录；名称冲突时使用稳定的安装实例后缀。
3. 拒绝磁盘根、项目内嵌套、符号链接、junction、重解析点和其他节点 marker。
4. 自动创建失败只记录“推荐数据根暂不可用”，明确“原项目未移动或删除，任务继续使用原路径”。
5. 不创建空缓存来替换旧项目已经使用的共享缓存；旧项目保留原环境，新托管项目才使用新缓存。
6. 体检器可以登记历史共享、开发检查、Win 发布、服务器发布和仓库旧缓存，但所有外部候选默认 `automatic_action=none`。
7. 后续迁移必须先预览空间峰值与回滚路径，逐个作用域验证；旧缓存保留观察期，清理需再次授权。

## 发布门禁

Windows PC 节点是默认交付目标。`publish-node-agent.ps1` 只有显式传入
`-IncludeLinux` 才构建、上传 Linux PC 节点包；没有 Linux 客户时不得让
Linux 交叉编译延长 Windows 发布。该开关不影响 `publish-server.*` 的 Linux
服务器构建，两者是不同产物。

自动更新等待恢复检查点的单次尝试默认最多 90 秒。超时后保持旧 runtime
运行并让持久远端队列稍后重试，不得持有发布锁等待数小时；日志必须列出仍
缺恢复证据的任务身份。

远端 outbox 的单次执行最多 10 分钟，广播接口的响应最多等待 20 秒。广播
超时按“请求可能已送达”进入有界握手核验，不得盲目重放非幂等 POST，也不得
让一个失联响应把后续发布串行阻塞一小时。

全量发布前至少验证：

- 最近三个已发布版本的 `node.json` fixture：缺字段、空字段、未知字段、旧 workspace/storage、自定义路径、损坏显式配置。
- 缓存 fixture：环境变量共享 target、`.env.local`、Windows 默认开发/发布目录、项目祖先 `shared`、仓库内部 target。
- 空盘、低空间、只有 C 盘、项目在 D/E 盘、名称占用、junction、脏 worktree、未 push 提交和多项目绑定。
- 更新中断、原子写失败、EXE 重启和重复迁移；原子写还要覆盖并发 writer、临时文件名冲突、Windows 目标文件短时占用、低空间/磁盘写满、主文件损坏与备份恢复。任何失败都不能丢失身份、绑定、task journal、sidecar 输出或覆盖项目，持续损坏必须保留完整 IO/Win32 错误链并显式失败。
- 活跃监督任务的延期更新、排空完成后重启、排空期间异常重启恢复；`/api/status.restart_recovery` 必须能区分 `draining`、`restart_scheduled`、`runtime_online`、`resume_required`、`failed`。
- 两个 recovery v1 回执已到 `resumed` 后，旧 restart checkpoint 不得把任务降级回 `resume_required`；真实更新 fixture 必须证明新 release 上任务为 `running`、journal 游标继续增长并正常终止，全程无需人工 Resume。
- 同 root 多代续跑、旧项目别名、跨项目/跨 root、脏而不可信现场和并发占用 fixture；仅前两类安全继承，后四类显式拒绝。
- 全局 publish lease 并发 fixture：owner/FIFO waiters 可观察，相同 kind + SHA coalescing，release SHA claim 后不可变，排队期间 `main` 前进不会改写待发布 SHA 或造成饿死；同批次跨阶段 heartbeat、崩溃接管、未知状态和损坏 ledger 也必须覆盖。
- 取消四元组与可信 `interruption_source` 的新旧 sidecar/journal/API fixture，以及 PC UI 的更新恢复、自进化队列/暂停/审查、发布 batch/owner/waiters/stages 和取消来源测试。
- 低优先自进化在前台任务、发布、更新和构建压力下 pause/yield，门禁解除后自动 resume；必须证明实际派发目录是已持久化的独立 worktree，并覆盖 action intent 重放、审查 provenance 和配额自动重试。
- 远程监督 v1 的身份、能力、live lease 和断线恢复 fail-closed fixture；本地可信任务优先，远程证据缺失不得降级绕过。
- PWA Runtime 捕获的 Windows Edge/Chrome 标准路径与 `ELON_PWA_BROWSER_PATH` 探测、浏览器缺失诊断、真实 loopback HTML/PWA fixture 精确 viewport PNG、SHA-256/route/revision 元数据、认证失败不误报、SSRF/秘密门禁，以及成功/超时/启动失败后的浏览器进程树和临时 profile 回收。发布包不得新增 Desktop Browser 或人工可见浏览器依赖。
- 后台设计 v1.12 fixture：旧节点不识别能力工具时 PC 只显示待升级且旧 CLI/Exec 正常；新节点能力回读精确。持久浏览器覆盖同 session 状态保留、项目/origin/auth/fixture/viewport 绑定、4/15m/60m/128 上限、秘密键值和 password/hidden/file 拒绝、停止/过期进程回收；草稿预览覆盖属性和值白名单、首次内联值恢复、外部资源/脚本拒绝和“非源码证据”；源码候选覆盖 UI tree 哈希、sourceRoots 越界/ignore/文件大小/数量上限、秘密行跳过、SHA-256/range 以及不得自动 BOUND。本机比较器覆盖输入 SHA 漂移拒绝、像素 PASS/FAIL、允许/非预期 selector 变化、紧凑 artifact 哈希和项目越界拒绝。Tauri 覆盖后代窗口、菜单/对话框分层、插桩与未插桩 trace，证明工具没有任意菜单点击或任意 command 执行；四端验证矩阵不得在缺少平台回执时标记 PASSED。
- Android 调试身份 fixture：旧 `node.json` 首次补写、连续更新/重启包名不变、身份漂移 fail-closed、三个会话按序合并、新代次淘汰旧构建、USB/无线端点共用物理设备部署锁、所有兼容后缀无法绕过固定真机包、正式包不受影响、历史杂包仅报告不自动卸载；另须覆盖 LKG 缺字段/默认关闭时不记录且不阻塞安装，以及任务显式启用后仍保留同文件冲突保护、最近成功 APK 保留和签名钉扎语义。
- Android Renderer 调度 fixture：同项目不同空闲模拟器槽允许并行，不同项目不能写同一槽，同真机跨 PC 的全局 lease 冲突拒绝，新 generation 在安装前 fencing 旧 generation，断线/过期 lease 仅在进程和心跳证据闭合后回收；显式离线 deviceId 且 `fallbackToEmulator=false` 不换机，多个在线 emulator 必须跳过已租用实例。第一阶段至少覆盖节点内 busy serial 排除、设备级部署锁、多个 Runtime 时 fail-closed 路由和 60 秒回退；跨 PC TTL/heartbeat/fencing、独立 AVD 数据目录与 FIFO 队列未落地时必须在发布报告中列为未覆盖边界。
- Debug Runtime/FitRun fixture：原业务 Git/workspace revision 到合成 integration revision 的映射可复核；隔离模拟器包仅在 generation、integration、Git、runtimeBuildId 与零 Patch 全匹配时接受；generation、installed generation、integration HEAD、业务 HEAD、包、Runtime 或 Patch 任一漂移都拒绝。
- 跨端验收 fixture：真实 Android/Web 视觉分支继续要求独立截图与 loss 阈值；`NO_WEB_COUNTERPART` 使用跟踪仓库来源与零匹配扫描通过，存在 Web 匹配、非跟踪 Android 来源、空 Web 根或伪造 Web 截图均拒绝。
- 无新 capability 的旧节点执行已有项目 CLI/Exec；有新 capability 的节点创建托管 workspace。
- 外部项目、托管项目、只读任务、普通写任务和真实构建分别验证路径与环境策略。
- 超容量建议、项目数建议和无法读取磁盘空间时仍可派单；无自动压力清理。
- PC 页面用通俗文案解释“建议模式”，不能显示误导性的“必须配置”或“项目空间不足”。

发布采用小批量灰度，并保留服务端停止派单、停止升级和版本回滚开关。重点监控升级后第一次 CLI、Exec、写任务和构建的成功率，以及“数据根自动回填失败但旧项目继续运行”的降级成功率。

## 事故处置

1. 立即冻结继续扩量，保存客户端版本、服务端版本、节点配置 schema、任务类型和路径判定证据。
2. 分开统计明确报错、满足故障条件但尚未发起任务的潜在节点、自动降级成功节点，不能只看错误条数。
3. 优先发布零手工前向修复：取消错误硬门禁、恢复旧路径和环境、让配置回填变为 best effort。
4. PC 页面和任务错误先说明“原项目与缓存安全”，再说明推荐功能和自动恢复状态，不要求新手手动迁移。
5. 修复后先在自有节点复现五类缓存和旧版本 fixture，再按小批量扩大。
6. 复盘必须落到代码分类器、配置迁移器、fixture、协议能力、遥测和发布门禁；不能只补一段说明文档。

## 变更评审检查表

任何 PR 若新增“required”“reject”“capacity”“cleanup”“migration”逻辑，评审者必须回答：

1. 这条规则针对新建托管数据，还是会影响已有外部项目？
2. 失败能否保留旧路径、旧环境和旧缓存继续运行？
3. 指标是建议还是不可恢复的安全错误？为什么必须阻断？
4. 是否可能创建第二份大缓存？是否展示磁盘峰值？
5. 是否会触碰平台数据根之外的目录？
6. 是否有旧版本 fixture 和“升级后首次任务”回归测试？

没有明确答案时，不得把新治理标准升级为硬门禁。
