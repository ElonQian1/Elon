---
title: "本人网格只读投影传输 V1 交付证据草稿"
version_status: current
status: in_progress
implementation_status: in_progress
reviewed_at: 2026-09-07
owners: [private-read-bridge]
authority: evidence
---

# 当前结论

主项目已实现本人网格的通用私密投影传输、持久待同步记录、主账号隔离和独立只读授权；主服务器 0.3.1736 已正式发布并通过健康与版本核验。可信 HTTPS、节点、Win 与 APK 工件仍待接通，未完成真实 Win → 私密接口 → 量化 APK 验收，不能据此宣称手机已读到本人网格。

需求以 [本人网格只读投影传输 V1](../requirements/private-grid-read-bridge-v1.md) 为准。本报告只记录实现和验证结果；币安字段转换及手机列表详情属于独立量化仓，本仓没有新增交易执行或收益计算规则。

既有资产授权仍为已测未接通，先前结果见 [统一账号与资产授权证据](unified-account-asset-access-v1.md)；量化交易模式仍为 Paper，签名上传、验收及资金分配未完成。

## 已实现范围

| 层次 | 实现与边界 | 源码入口 |
|---|---|---|
| 本机节点 | 当前已绑定账号领取不透明 binding；本地管理员保护的入口接收有界快照，事务写入 SQLite outbox 后才返回 queued/unchanged；状态区分 pending、synced、rejected，并包含当前修订、待同步修订及最后成功时间 | `server/src/node_agent_private_read_projection.rs` |
| 保密上行 | 独立显式 `ELON_PRIVATE_READ_HTTPS_ORIGIN`，默认关闭；只接受规范 HTTPS origin，禁止代理、重定向和 HTTP/WS 回退；当前节点凭据及 epoch 变化停止旧批次 | `server/src/node_agent_private_read_projection/sync.rs` |
| 云端保存 | HTTPS 路由从当前有效节点凭据及数据库绑定推导 owner/node/install，同事务复核后持久提交，再返回匹配修订及连接的 ACK | `server/src/private_read_projection_api.rs`、`server/src/store/node_credentials/private_projection.rs` |
| 数据合同 | `yilong.private_read_projection.v1`；固定首期 source，递归排序 JSON 的 SHA-256 修订；相同修订幂等，代次必须前进，列表观察时间不倒退，相同列表时间不能延长新鲜期 | `server/src/private_read_projection.rs`、其 `storage.rs` 与 `strict_json.rs` |
| 本人读取 | `/api/asset-access/grids` 返回按来源区分的 snapshots，以及 subject/client_id/grant_id/expires_at；grant、主账号会话、节点当前性和每份私密投影在同一事务中复核 | `server/src/store/node_credentials/grid_projection.rs` |
| 独立同意 | 精确 scope `grid.snapshot.read`、purpose `binance_grid_read`、独立确认文案和 `com.elon.quant:/grid-access/callback`；量化 Android 专用且最长 900 秒；旧 ESK grant 不增加新权限 | `server/src/esk_platform/access/`、`android/app/src/main/kotlin/com/elon/app/grid/access/` |
| 传输装配 | 复用正式原生 TLS listener 的服务内标记；代理请求头不能伪造 TLS 权威；上传局部 256KiB 限额与普通账号 16KiB 限额分开 | `server/src/node_endpoint_transport/asset_access.rs`、`server/src/account_security/https/` |

持久 schema 通过迁移 291 新建投影表；既有 ESK 授权表保持原结构。快照最多 32 个来源、每份最多 500 个机器人、响应总量不超过 256KiB，超限显式失败，不静默截断。GET 保留原始 status、时间及 revision，客户端依据 fresh_until_ms 派生过期状态。

私人正文不进入公共 `/quant` 代理、公开团队资产接口或旧明文节点 WebSocket。上行只携带版本化业务投影，不携带浏览器凭据和原始请求头。

## 来源账号绑定补充

跨端审查发现：本机入队 binding 单独只能证明接收端的当前账号，不能证明 UI 缓存 owner 所选研究会话属于同一人。

本批增加了来源证明：Win 原生研究会话摘要返回持久绑定的 `owner_hash`，节点 binding 同时返回 `research_owner_hash=SHA256(current owner_user_id bytes)`。worker 在状态读取前后比对二者，并保持启动时 binding；最终 POST 在同一次当前凭据锁与 epoch 校验中入队。缺少证明或账号变化时停止，不自动把旧资料重绑给新账号，也不导出原 owner。

此项需要发布含新摘要字段的 Win 版本；旧 Win 不返回证明时必须停止同步。原研究会话的一小时期限不被延长。源码补丁与跨端验证不能替代新 Win 工件及现场验收。

当前真实节点已按持久模式的源码解析规则确认 `endpoint_required=false`，确认过程只输出布尔值。首版可使用其现有正式节点凭据走受信任 HTTPS；若进入 endpoint 模式，本机入口显式返回 `projection_endpoint_authority_required`，不能降级旧凭据。

## 已完成的验证

| 验证 | 实际结果 | 证据 |
|---|---|---|
| 旧 ESK 与网格独立授权、PKCE、scope/client/session 隔离 | 25 passed，0 failed，136 filtered | 官方日志 `private-grid-access-harness-final-20260907-155254-500`；validation fingerprint `8739aa7f1e73a5d8eb7db947af1707c591c145c2091d067842174f6442a1fb02` |
| 私有投影合同、持久 outbox、回执、重复/乱序、owner 隔离、Kotlin 完整 GET 互认 | 10 passed，0 failed，0 filtered；实际执行全部测试 | 官方日志 `private-read-projection-harness-path-fix-20260907-162119-964`；fingerprint `b7a6f34adaa772565b288ec8b1d52a78cbff462aa5c7082aba3838456cfa7702`；26.4 秒 |
| 最终 HTTPS policy：owner 撤销路由、请求边界与正文上限隔离 | 5 passed，0 failed，6 filtered；包含真实撤销路由测试与 256KiB/16KiB 隔离测试；非证据复用 | 官方日志 `private-grid-final-https-policy-after-crate-rebuild-20260907-164826-149`；fingerprint `1d0c14d89c1306e92848e971362bb4dab9b4f6bbee675347d900c25abbf920d0`；90.5 秒 |
| 主 APK 原生同意与交接定向验证 | `GridAccessRequestTest` 4 项、`AssetAccessRequestTest` 6 项通过；完整主 Kotlin 编译成功 | `:app:testDebugUnitTest` 定向两类；官方日志 `main-grid-consent-android-compiled-20260907-154517-206`，exit 0，167 秒；不混入量化仓测试数量 |
| 最终 production 节点与主服务器编译 | `elon-pc-node` 与 `elon-server` 两个真实 binary 的官方 check 通过，356.4 秒；包含最后的模块路径及 owner 撤销 policy 修复；非证据复用 | 日志 `private-grid-final-production-after-crate-rebuild-20260907-163733-996`；fingerprint `7800bf3d7b5ce1392882958a85ecc6670daa8258dc36d7f7e683573903689a42`；不是局部 harness 推断 |
| 最终 Win 原生来源证明与生命周期 | 真实 Tauri test binary 编译成功；`browser_research::tests::lifecycle` 2 passed、0 failed、196 filtered；验证 owner_hash 绑定及只有已确认 host 可进入观察状态、失败不可复活 | 日志 `private-grid-final-native-owner-20260907-165026-355`；fingerprint `c7754536b62d487d5d9b6cf484881d6e3589a3d80a554abb55a226e7a2f464ed`；fresh run，434.1 秒；测试不操作已登录 Win 窗口 |
| Rust 格式与源码体积 | 本批 Rust 按官方入口格式化；源码体积门禁通过 | `scripts/format-rust.ps1`、`scripts/check-source-size.ps1`；新源文件均小于 500 行 |
| 量化 APK 对照证据（独立仓） | 87 项通过、0 失败/错误，`assembleDebug` 成功；包含实际 Rust adapter 行 → Android 完整 GET 解析 | 日志 `quant-grid-android-adapter-interop-final-20260907-161624-476`；`GridAdapterInteropTest`；不计入主仓测试数量，不是正式包或真机验收 |

官方命令日志位于仓库共用 Git 目录的 `ai-command-logs/`，Rust 验证证据位于本机缓存根的 `validation-v1/evidence/`；旧缓存保留。

Kotlin 实际导出的合成完整 GET fixture 已纳入 `server/tests/private-read-projection-harness/fixtures/android-grid-read-wire.json`，修订为 `e9a1bd2c5dfd79b1e5e38fe7902ba5550439a93d060ad95178f75314308f9b31`。上述 10 项中的实际互认测试已验证 Rust `projected()` 完整相等、摘要相等及 `0.000000000000000001` 金额字符串不损失精度，未把重复断言另计为测试数量。

### 验证恢复记录

此前投影 harness 的正式构建遇 `libsqlite3-sys v0.28.0` 构建脚本 `0xc0000005 STATUS_ACCESS_VIOLATION`。主任务按原临界磁盘阈值保护构建，并通过本次 Cargo config 显式禁用被用户配置覆盖的 wrapper 后，进入实际 Rust 编译；编译暴露独立 harness 的子模块相对路径错误。生产模块现对 `storage/strict_json/transport/tests` 使用明确 `#[path]`，harness 与生产继续读取同一实现，没有复制源码。修复后经过官方入口取得上表 10 项通过结果，旧失败不再作为当前阻断。

最终生产及 policy 验证随后分别遇 `num-traits v0.2.19` 与 `thiserror v1.0.69` 构建脚本的同类 AV。为排除工件因素，只通过官方验证入口在本项目 `validation-light-0` 注册分区对这两个已知失败 crate 分别定向 clean，再各做一次重建，取得上表最终通过结果；未清理目录、未知缓存或量化分区。AV 本身不能证明 EXE 损坏，本报告不据此认定故障根因。原 8% 临界磁盘门槛保持，验证前平台 doctor 为 healthy，磁盘约 10% 可用且无活动编译者。

上述生产、policy 和 Win native 检查串行执行，最后 native 检查同样复用此注册分区并明确关闭 wrapper，两个目标测试实际通过；完成时缓存盘可用空间约 9.35%。没有在通过后追加整仓测试或重复构建。

## 尚未完成的验证与外部条件

- 主服务器、Win 及主 APK 已发布；Win 新版加载及真实同步仍待验证。
- 量化 APK 已形成正式签名候选；商店发布、手机安装及真实授权交接仍待验收。
- 公网 HTTPS 已于 20:16 通过下文的 443/8443 路线激活并完成直连边界验收，80 保持关闭。真实 Win 上传与 APK 授权读取仍未验收；现有 8080 兼容路径不承载私人网格正文。

## 主线与服务器发布后续证据

- 主项目业务提交 `e9c3ff4705fa925baa54245fc191f72069058705` 已进入 main。
  正式发布器重试时按其既有流程快进到 `d5e3e3d829df589bbd6e15394adfb69dac15d944`；
  两个后续上游提交仅更新 10 个 ChatGPT Android 测试文件，与本批源文件无交集，
  服务器和网格实现未变更，没有重复已通过的相关验证。
- 第一次正式发布尝试被日志入口的 900 秒无输出门槛终止（exit 125）。终止前已
  观察到编译 CPU 持续增长，未把日志停更当作编译通过或代码错误。保留缓存，
  通过同一正式发布入口重试，仅将日志等待时间改为 2700 秒、总时限仍为 3600 秒。
- 重试实际完成编译、上传、锁内替换与服务重启，发布 **0.3.1736**；
  `/health` 为 200，版本接口 SHA 精确为上述 `d5e3e3d...`。正式日志
  `grid-read-publish-server-retry-20260907-173153-146`，exit 0，1658.2 秒；
  结果文件 SHA-256 为 `8973d46a9da08c3e7c243a25760ab97cd0b3685a3c44e26b5c8624bf9e88b990`。
  没有使用跳过构建或强制覆盖参数，PC 前端未变更并按正式参数跳过其发布。
- 量化 PR #3、#4 已依次合并，最终主线 `f28f9ca6302917d48201c449a3804c4192235e49`。
  实现的 Android 与 market CI 均通过；最终干净主线原样执行官方 Paper E2E，
  8 项测试通过、回执包含 21 项检查且 `worktree_dirty=false`。回执 SHA-256
  `5939da78ecd2274d2b8379326e3a764d4382a09a8d174921302e170e52a4accd`。
- 原量化签名及配套 DPAPI 加密配置已找回，私钥解锁与旧商店 APK 证书一致性
  已核验；原材料保留，未重新生成签名或将密码写入项目。正式 APK 尚未发布。
- 主项目文档中的免密入口已用于 SSH 部署。已检查的标准云 CLI 配置、环境字段、
  项目配置及实例角色入口未提供腾讯云控制面凭据；不能由 SSH 登录成功推导出
  云安全组管理权限。80/443 在服务器临时监听时，外部 TCP 连接仍超时，8080 可达。
- 当前正式 Win 安装身份与工件哈希已核对，其更新器有桌面使用中延期保护。
  本批未触发 Win 发布或激活，原窗口保持。现场后台守护器多次重启不健康节点，
  又曾恢复 7799 监听；持久生命周期将退出标为 unexpected_exit，尚未确定根因。
  已安装版本的隐藏启动路径未保存运行 stdout/stderr，不能据此猜测数据库或端口错误。

## 本批现场边界

2026-09-07 主任务通过 `/api/status` 的正式原生 bootstrap 在内存取得本机管理 token，仅输出 `native_bootstrap_available=true`。带正确管理 token 读取 `/api/private-read-projections/binding` 得到 404，证明当前安装节点尚缺本批新 API，需要正式发布升级；此结果不是缺少 token 的 403。该检查未提交投影正文、未输出 token 或私密内容、未读取过期研究会话，Win 登录窗口保持。

本批没有创建网格、修改仓位、结束策略、签名交易或移动资金；没有关闭已登录币安窗口、清理或复制登录 Profile。主服务器及后续客户端工件的发布证据分列如下；没有把本地 queued 回执当作云端持久成功，也没有把私有 API 源码发现当作 APK 已经读到真实数据。

完成可信 HTTPS、正式工件发布及真实 Win → 云端 → APK 验收前，交付状态保持 `in_progress`。

## 自定义 HTTPS 入口准备

后续只读核对确认主服务当前 `PUBLIC_URL` 和监听仍为 IP 的 HTTP 8080，
未配置自有域名、DNS 提供器或可用账号证书。8443、9443 在服务器临时监听时，
本机连接均失败，8080 对照成功；临时监听已关闭。另一个已监听的公网端口
由其他服务占用，未改动它。不能把“换成 8443”当作公网条件已解决。

本次仅补部署入口，不修改 Rust、8080 服务或客户端运行配置：

- `configure-account-https.sh` 支持旧 IPv4 调用、自定义 HTTPS origin、
  外部管理的证书/私钥路径，以及无部署副作用的 `plan` 和 `check`。
- 地址解析独立在 `account_https_target.py`；原生 HTTPS 已支持自定义端口，
  本次不重复实现 TLS 或加入代理，也不自动签发域名证书。
- 激活前校验证书信任、目标域名/IP、剩余有效期及密钥匹配；保护旧 8080
  和其他监听，并沿用配置备份、失败恢复与原生证书重载。
- 目标解析与合成证书检查均在本机离线执行；测试证书不进入生产配置，
  测试目录中的材料在结束时删除。`plan` 不是连通、签发或真实 APK 验收证明。

当时已有 IP 证书签发器仍依赖公网 80；外部证书路径可来自 DNS 验证等独立
提供器，但当前没有相应域名或证书。仍需匹配固定 HTTPS 地址的可信证书
和公网可达的空闲业务端口，才能进入激活及后续 Win/APK 发布。

最终验证：14 项 Python 参数测试、15 项离线 shell 检查通过。独立审查
发现的 dotenv 路径解释差异及空密码加密私钥与原生解析器不兼容已修复，
专项用例通过并完成复核。正式日志
`account-https-external-port-reviewed-20260907-192323-019`，exit 0，10.3 秒；
结果文件 SHA-256 为
`ed4ced3dea961c7c63d5d6cbe1d5f0012f6f7a6ca3df0590fd484a1700a32a0a`。
测试不能证明生产激活/回滚、Android 信任库兼容或真实续期成功；本批未
启用 HTTPS、重启主服务或更改浏览器会话；以下后续部署证据取代这项运行状态。

## 443/8443 正式部署证据（20:16）

用户放行 443、8443 后，两端口的临时 TCP 监听均被本机直连成功，8080
对照仍正常。此前失败测试已明确禁用代理；不能归因于 Windows 系统代理。
临时监听已关闭。提交 `331d0e97f` 已推送 main，新增固定版本 lego 5.4.1
安装器及独立 TLS-ALPN-01 证书生命周期，443 仅用于验证，8443 承接业务。
官方归档校验固定 SHA-256 后才安装，未更改 80 或复用其他服务端口。

- 真实 staging 签发通过，状态与生产隔离：正式日志
  `account-acme-alpn-staging-20260907-201404-158`，exit 0，15 秒。
- 真实生产签发及信任链、IP、有效期、私钥匹配检查通过：正式日志
  `account-acme-alpn-production-20260907-201445-150`，exit 0，12.9 秒。
  发行者 Let’s Encrypt YE1，到期时间 `2026-09-14T03:16:24Z`；证书 SHA-256
  `27a9e3ca5e5b831dc827fd2e166c754cc47511dec1b6c7695b5e2ffee25c10ba`。
- `elon-account-acme-renew.timer` 已 active，每六小时检查、随机延迟不超过
  十分钟。一次手动续期检查确认现有证书未到续期窗口，正确跳过；日志
  `account-acme-renew-check-20260907-201548-946`。这不是未来定时续期成功证明。
- 正式激活 `https://43.139.149.158:8443`，仅更改账号 TLS 四项配置，
  重启主服务器一次；日志 `account-https-8443-activate-20260907-201603-023`，
  exit 0，4.9 秒。未重建或替换已经发布的服务器二进制。
- 服务器检查及本机 `curl --noproxy '*'` 严格信任直连均通过：健康接口 200，
  服务标识 `elon-account-https`；`/api/me`、`/api/auth/security` 与
  `/api/asset-access/grids` 未登录均 401；管理、MCP、节点列表均 404。
  8080 健康仍 200，没有以关闭证书校验或降级 HTTP 获得成功。
- Win 的正式持久配置入口已保存 `ELON_PRIVATE_READ_HTTPS_ORIGIN`，供下一次
  启动使用；当前运行进程、已登录币安窗口均未改动。

本项只证明可信传输已可用，不证明 Android 信任库、真实授权、网格同步或
续期后热重载已验收。节点候选、两端正式 APK 与完整私人读取流程继续推进。

主 APK 的 HTTPS 默认地址现保存在版本化 `android/gradle.properties`，
构建脚本直接读取仓库默认，非空环境变量可显式覆盖；不会被合并后的陈旧
用户 Gradle 属性悄悄覆盖。三项实际 `generateDebugBuildConfig` 检查通过：
环境覆盖、仓库默认与陈旧属性优先级、HTTP 地址拒绝。成功场景均核实本轮
生成文件与构建成功标志；正式日志
`main-asset-origin-persistent-config-verified-20260907-202349-512`，48.1 秒。
本机 JDK 17 需复用任务内短 Unix socket 临时目录；仅设置构建进程参数，
未更改全局 Java、Gradle wrapper 或启动完整 APK 构建。

## 正式客户端工件与当前缺口（21:03）

上述配置验证后，主 APK 经正式发布入口发布 **1.1.1546 / 1546**，源提交
`abc3e6618d1b550d2a2db0b5822a4583de8d6597`。线上版本和 SHA-256 与本地
验签包一致：`a000ace4833fc5f4f94b03b8da5579868a128835a3765d8f6a79cd2bba3ed819`。
实际 DEX 的配置和网格授权 Activity 均使用 8443；原签名、调用方身份校验
及遮挡触摸保护保留。正式日志 `grid-read-main-apk-publish-20260907-202704-347`，
exit 0，513.8 秒。发布器的工作树自动清理提示 `Branch` 属性缺失；没有将
该本机清理问题当作线上 APK 发布失败，也未据此绕过统一收尾。

量化主线保持 `f28f9ca6302917d48201c449a3804c4192235e49`，正式原签名候选
**0.5.0 / 5** 已完成构建。10 套件、87 项 Release 测试通过，失败、错误及
跳过均为零；实际 APK 验证非 Debug、唯一原签名、源 SHA、8443 地址及最低
主 APK 1546。包 SHA-256：
`e81c836bc912d93122443f400b20beebdcbb2261e3c1b4e22b23d6258a8a6e4d`。
正式日志 `quant-private-grid-release-build-20260907-203640-390`，101.7 秒。
候选和收据已另存为稳定工件，**尚未商店发布或手机安装验收**。

服务器指定环境文件和当前运行进程均未配置 `OWNER_TOKEN`。用户确认 Win
主项目已登录，页面确有正常项目上传功能；但当前旧 Win MCP 无文件选择、
填表或上传代理。没有导出登录令牌、创建管理员令牌或伪造应用发布身份，
也没有为了导航发布页而卸载当前币安 WebView。上传仍需现有授权页面或以后
补充的受控发布能力。

Win 正式构建完成，候选身份 **0.3.69+abc3e6618d1b550d2a2db0b5822a4583de8d6597**。
正式日志 `grid-read-win-node-publish-20260907-203959-922`，exit 0，1233.9 秒。
客户端包 SHA-256：
`88b0335b73e46e76a274a535b59e93d6914f662ae152ce3a4da935eef266b229`；
异步发布回执为 `synced`、attempt 1，远端版本及包哈希一致。
本批使用 `-SkipLocalActivation`，激活器 PID 为空；虽然旧脚本/状态固定显示
`restart_scheduled`，不能据此宣称本机已经重启或新能力生效。21:03 旧桌面
仍是 14:28 启动的同一进程，新版运行加载和完整私人读取链路未验收。

构建空间通过官方缓存工具回收，不降低 8% 门槛：先回收已结束的量化验证
缓存，再依据构建元数据和正式日志核实两份本批服务器编译隔离缓存的来源，
连同本批共享验证缓存回收。两次实际释放约 2.58 和 4.47 GiB；源码、测试
回执、已有发布工件及登录资料均保留。缓存回执为 `gc-20260907-122101.json`
和 `gc-20260907-123852.json`。上述客户端工件不证明真实网格上送、授权读取、
撤销或账号切换已完成，功能继续保持 `in_progress`。
