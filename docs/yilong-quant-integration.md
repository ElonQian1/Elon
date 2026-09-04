# 一龙量化交易子项目接入与协作边界

## 结论

一龙量化交易使用独立仓库 `git@github.com:ElonQian1/yilong-quant.git` 开发，通过主项目的官方项目目录和项目落地清单进入项目广场。它不是主仓库内的源码子目录，也不沿用“一龙网游加速器”的历史默认加入逻辑。

这种结构让量化项目可以独立迭代、测试和发布，同时复用一龙的项目广场、身份授权协议、通用回执和 Rust 编译缓存。双方不共享业务数据库、Redis、交易密钥、客户资金或原始 Cargo target。

## 用户现在能看到什么

- 项目广场中的“一龙量化交易”标题、简介、目标用户、当前更新和 GitHub/文档入口。
- 登录用户在主项目账号页可以查看本人 ESK Paper 总额、可用额、卖回占用、量化分配申请占用和总占用，并可提交或取消尚未被量化端接收的 Paper 申请；在量化公开环境配置就绪后，用户可明确选择一笔申请进入量化页面，看到同一组余额的签名只读投影，并创建或释放独立模拟 binding。量化端返回的 accepted/released 回执只有经主项目验签后才改变状态。短期授权和投影不进入 URL 或浏览器持久存储。
- 项目主页当前更新会说明已经验证的 `BACKTEST / SIMULATION` 研究能力：行情来源、策略版本、费用、滑点、权益曲线、回撤和确定性摘要均可追溯。
- 明确的 `paper` 阶段：模拟委托、运营审核追加模拟份额、NAV 与用户部分/全额退出结算。
- 6% 仅为非保证目标；当前不导入真实付款名单、不移动资金、不连接实盘。
- Web、Windows 和 Android 尚未公开部署时继续显示“计划中”，不生成虚假下载入口。
- Android V6/V17 复用主项目现有项目发布存储和 Rust/Axum Web 托管：正式量化 APK 可由主服务器保存并由项目广场安装；无凭据 Paper PWA 由主服务 `8080` 下的 `/quant/` 提供，量化 API 仍是只监听 loopback 的独立进程。APK 已上传、页面可打开和本人 ESK/仓位可访问仍是三个独立验收状态。

主项目官方目录只为量化项目创建公开只读项目和首页快照，不给它绑定 ERP 蓝图安装动作。

## 仓库责任

| 责任 | 主项目 | `yilong-quant` |
|---|---|---|
| 项目广场、公开只读入口 | 负责 | 提供 `.elon/project-landing.json` 内容真源 |
| Android 安装包 | 保存正式发布、提供公开下载路由和主 APK 安装/更新/打开入口 | 负责 `com.elon.quant` 源码、独立签名、版本、Git SHA 和上传回执 |
| 用户身份、ESK Paper 账本、卖回/量化申请占用 | 负责，并验签量化回执后推进状态 | 消费版本化授权并维护独立模拟 binding；不从只读投影、客户端自报或旧 NET 创建仓位 |
| ESK 发行和可用余额 | 主项目是真源；Paper 与经审核正式平台登记独立，均不等于上链发行 | 不铸造、不直接修改、不保存余额副本 |
| ESK 服务效用、治理和团队利润参与 | 定义政策、服务支付账和不可变分配快照；当前真实能力未实现 | 只消费版本化证明，不另发团队权益币、不把 ESK 当基金份额 |
| 模拟份额、运营追加与部分/全额退出规则 | 不复制 | 负责 |
| `QSHARE`、NAV、申赎与交易所对账 | 不复制，不从 ESK 余额推导 | 负责；客户本金与团队费用收入分账 |
| 行情、策略、风控、OMS、交易所 | 不直接控制 | 负责 |
| 真实资金、NAV 和退出 | 未来提供准入/回执边界 | 未来在独立托管和会计门禁后负责 |
| 共享算力 | 调度无密钥研究任务 | 提交可复现回测/研究任务 |

## 远程 Git 协作

量化开发人员在子仓库按 `AGENTS.md` 和 `docs/remote-collaboration.md` 使用独立 worktree、需求认领、模块责任区、测试和 PR。跨仓库需求遵循以下顺序：

1. 先在量化仓库稳定 Manifest、API/回执版本、需求和能力状态。
2. 再在主项目更新官方目录或平台协议，不复制量化业务源码。
3. 两个仓库分别提交、验证和推送，提交/PR 互相引用 SHA。
4. 任一侧尚未部署时，另一侧失败关闭或显示“计划中”。

## 当前技术接入

- `server/src/official_project_catalog/catalog.json` 保存官方公开项目元数据和净化首页快照。
- `server/src/official_project_catalog/mod.rs` 支持两类官方项目：带 ERP 蓝图的可安装项目，以及不带蓝图的普通公开产品项目。
- `GET /api/store/projects/:id/preview` 只为当前公开且存在于内置官方目录的项目返回 `yilong.official_project_preview.v1`。PC 项目广场在用户加入前按需展示能力、目标用户、最近更新、隐私边界、客户端计划和净化 Paper 说明；下载/资源 URL、工作区内容、grant、participant、密钥和仓位不会进入该响应。
- 量化项目属于第二类，因此不会显示 ERP 安装动作。
- 子仓库自身的 `.elon/project-landing.json` 仍是后续同步更新的内容真源；官方目录快照变更必须与它保持一致。
- Android 主服务器托管 V6/V17 要求公开下载路由只直接返回 `DATA_DIR/project-releases` 下且大小和 SHA-256 与发布记录一致的 APK；服务器启动同步官方目录后重新叠加最新 release。当前 `com.elon.quant 0.1.0 (1)` 已由 Server `v0.3.1721 / 725f91f0a` 托管，目录下载地址非空且真实下载摘要与发布回执一致；真机安装/打开仍待验收，详见 `docs/yilong-quant-android-main-hosting-v6-acceptance.md`。
- 2026-09-04 再次公开下载核对：量化实际 APK 为 `com.elon.quant 0.2.0 (2)`，
  SHA-256 `c17ab5abe800547f41acc95594021abb6cec92fc14cb6de3b2db202ce4b94b89`，
  与量化仓已发布回执一致；上述 V6/V17 是历史托管证据，不是最新 APK 版本。
  [官方量化 APK 身份校验 V1](requirements/yilong-quant-android-identity-v1.md)
  为主 APK 增加安装与打开的固定身份门禁；它不传账号凭据，不代表独立 APK 已有本人资产。
  验证、发布和剩余授权接缝见 [交付记录](yilong-quant-android-identity-v1-delivery.md)。
- 子仓库 `77ff21c6ce3d0984273ad6e3ad526faff0d46b7c` 已验证版本化研究行情和单机可复现回测 V1；主项目只同步净化后的能力摘要与文档入口，不接收行情数据、策略代码、订单或回测结果。
- 子仓库 `25ee7df69fecabb696e7b8329fd388bc8a19da11` 已新增只读、脱敏的 Paper 部署配置预检：`scripts/check-paper-deployment.ps1` 检查本机绑定、非零固定端口、绝对数据库路径、主项目 HTTPS 来源、grant 验签公钥和操作员令牌等必需配置。输出不包含密钥或令牌，执行时不绑定端口、不打开数据库、不访问外部网络，因此只证明配置是否就绪，不证明目标环境已部署。
- 子仓库 `effef80b65647b2341cb902e64bd9aa171fdb613` 已实现受独立运营令牌保护的 `yilong.quant.paper_operations_snapshot.v1`：它只汇总模拟仓位状态、精确金额、NAV 修订、SQLite 事件头和导入批次，不返回参与者标识。主项目不复制或直接修改该账本，快照也不证明真实付款、NET 锁定、官方 NAV 或可提现余额。
- 子仓库实现提交 `7e943798050a63929e439a342cefa9b4f589028c`（交付证据 `520d15bb6b88d4f281efb4dd3ac46fc71877e847`）已完成 Paper 模拟仓位操作 V6：运营令牌可为已有活跃仓位追加模拟 NET 配额并按当前 NAV 增发内部份额；用户既有 `paper.redemption.request` scope 可申请部分或全额退出，部分结算后剩余份额继续按模拟 NAV 承担盈亏。新事件追加写入 SQLite 并兼容历史全额退出事件；用户不能自行增加余额，所有数据仍是 `simulated=true`。
- 子仓库提交 `2c9327ec94ff2a80e1a94115de9aa784eb63b5f7` 已完成 Paper 授权密钥轮换 V7：量化 verifier 固定信任 1–8 个公开 Ed25519 key，支持新 active 与旧 retiring key 按签发时间窗重叠验证，并让 revoked key 立即失败关闭；旧单 key 配置保持兼容但不能与 managed keyring 混用。主项目仍只持有一个当前签名私钥，双方不共享 seed、subject secret 或动态网络信任。
- 子仓库提交 `ba56940dffdffda6e487a5ae8ea2a577431119fc` 已完成 Paper 单次授权持久撤销 V8：用户只能撤销当前已验签 grant；量化服务只在本地 SQLite 保存 `grant_id`、过期时间和撤销时间，重启后继续拒绝该 grant，不保存完整 token、participant、账户或仓位。该动作只结束本次访问授权，不退出模拟仓位、不修改主项目会话，也不构成真实提现。主项目 grant 载荷、五分钟有效期、scope、签发和 exact-origin 内存传递协议保持不变。
- 子仓库提交 `0b87604e9105d7b0c1e4ba0da6b8b2c3c43d6ddc` 已完成 Paper 公开部署合同 V9：API `/api/health` 绑定编译时 40 位 Git SHA，仓库提供不含秘密的 loopback systemd、标准 HTTPS 443 Nginx 和环境模板，以及只发送 PWA `/`、`/api/health`、`/api/v1/runtime` 三个无凭据 GET 的公网验收器。它会拒绝 HTTP/loopback、错误提交、宽松 CSP/CORS、响应秘密标记、非 Paper runtime、`live_trading_enabled=true`、`funds_moved=true` 或错误 parent origin。离线 fixture 已验证但没有访问目标环境，因此当前只达到 `deployment_contract_verified`。
- V9 固定区分三种状态：`configuration_ready` 只证明量化进程配置形状，`deployment_contract_verified` 只证明模板和离线合同，`environment_deployed` 才表示批准的量化 HTTPS origin 已由 `scripts/check-paper-public-deployment.ps1` 真实读取并返回 `scope=public_https_read_only`、`network_calls_made=true`、`status=ready`。主项目只能在第三种状态后配置量化 Paper Web URL；不得用当前 HTTP 主站、未知 HTTPS 主机、关闭证书校验或放宽 exact-origin/CSP 临时上线。
- 子仓库提交 `e3359e4f0075163c30f2d54717a351530f1daa5d` 新增 V17 显式无凭据 HTTP Paper 预览。当前测试环境不使用 Nginx：主项目 Rust/Axum 在 `/quant/` 托管构建后的 PWA，并只转发 `/api/health`、`/api/v1/runtime`、研究快照与回测四个白名单接口到 `127.0.0.1:8787`。此例外只允许显示公开模拟研究结果，页面必须提示明文传输风险；登录、主项目 bearer、Paper grant、ESK 投影、本人仓位、订单、运营、导入、交易所密钥与真实资金全部禁止进入该通道。V9 的 HTTPS 要求继续约束未来的本人授权功能，不能从 V17 的公开预览状态推导为已满足。
- 主项目提交 `68bd2a314` 为 V19 新增精确的 `GET /quant/api/v1/markets/spot/overview` 白名单：查询参数按原字节保留但限制为 2 KiB，上游 origin 与 path 固定为 loopback，不转发入站 header、Cookie 或 Authorization，失败日志只记录固定错误类别。`HEAD`、写方法、相邻路径和既有敏感量化 API 继续拒绝。该提交只证明主项目代理代码与定向测试通过；量化 API/PWA、APK 和主服务器运行版本仍须分别发布及在线取证。
- `contracts/quant/net-balance-lock-receipt-v1.schema.json` 已定义主项目未来锁定 NET 后交给量化项目消费的版本化回执形状；详细语义见 `docs/yilong-quant-net-lock-receipt-v1.md`。
- `POST /api/me/quant/paper-access-grants` 已复用主项目现有 bearer 会话，可在独立签名配置启用后签发最多五分钟的 Ed25519 paper grant；量化项目只获得项目专用脱敏 subject 和明确 scope，不获得主项目 bearer 或用户资料。契约见 `docs/yilong-quant-paper-access-grant-v1.md` 与 `contracts/quant/paper-access-grant-v1.schema.json`。
- `GET /api/me/quant/paper-launch` 与 `POST /api/me/quant/paper-launches` 提供失败关闭的 readiness 和一次性启动票据；PC 项目主页通过 exact-origin iframe、`event.source`、nonce、attempt ID 和过期时间绑定，把 grant 只传给当前量化子页面。双方契约见 `docs/yilong-quant-paper-launch-v1.md` 与 `contracts/quant/paper-launch-v1.schema.json`。
- 主项目从唯一 ESK Paper 账本签发最长五分钟的只读资产投影：量化页面优先声明并接收 `yilong.esk.asset_projection.v2` / `yep2`，显示总额、可用额、卖回占用、量化申请占用、总占用、源修订和同步时间；旧页面仅在量化占用为零时安全回退 V1。投影与同次 grant 的 grant ID、脱敏 participant、key、签发和到期边界精确绑定，既不是申购回执，也不会创建仓位、交易或收益。主项目已随 `v0.3.1715` 发布，回执见 `docs/esk-paper-quant-allocation-request-v2-release-v0.3.1715.md`；量化子仓库实现提交 `1210e8b`、功能登记发布提交 `bc9d3de` 已推送，双方 V2 Schema SHA-256 均为 `ba3748fe22122e99271b5b6a0aeaa7fd61206557f22e67c807c26a0a97036c57`；契约见 `contracts/quant/esk-paper-asset-projection-v2.schema.json`，V1 继续兼容。
- ESK 申请绑定 V3/V13 已建立双向签名闭环：主项目只为用户明确选择的 submitted 申请签发与同次 grant 精确绑定的 `yeqa1` 授权；量化端验证后创建独立 `esk_paper_allocation_binding`，accepted/released 事件追加写入其 Paper SQLite，并用独立 Ed25519 域签发 `yqar1` 回执。主项目以 `YILONG_QUANT_ESK_RECEIPT_KEYRING_JSON` 验签后追加原请求状态，只保存回执摘要和 key/binding 元数据，不保存完整 token。submitted/accepted 继续占用 ESK，canceled/released 释放占用；该 binding 不调用 legacy NET 账本、不发行 QSHARE、不参与 NAV、不启动交易或收益。主项目代码与 PC 页面已随 `v0.3.1716` 发布，量化子仓 `origin/main@424b530` 已含 V13；独立 HTTPS 与生产密钥仍按失败关闭。合同见 `contracts/quant/esk-paper-allocation-authorization-v1.schema.json`、`contracts/quant/esk-paper-allocation-receipt-v1.schema.json`，操作与恢复说明见 `docs/yilong-quant-esk-allocation-binding-v1.md`。
- ESK 双仓互操作 V4/V14 已在主项目 `701457943501f270983410daed4382688387200a` 与量化项目 `c08b5af571a20ee84472779ad8ede17a6a942095` 的干净工作树完成离线验收：两仓共享 fixture 原始 SHA-256 为 `9499dcb52d911a0b999568c02d5ae1b8a13e9ca791481746abb6507550d82c97`，主项目真实 `ypg1/yeqa1` 可被量化 verifier 接受，量化真实 accepted/released `yqar1` 可被主项目 verifier 接受；篡改与 revoked key 会被拒绝。该验收无外网、生产秘密或资金移动，不代表链上发行或量化公网部署；证据见 `docs/esk-paper-quant-cross-repo-interoperability-v4-acceptance.md`。
- ESK 双仓可见余额 V5/V15 已在主项目 `73b6f09f1e50307c7e490e768e1206e92c54489b` 与量化项目 `6253f4eaedbf690698e064e5f582a3da5abc6537` 的干净工作树完成离线验收：主项目生产签名/序列化源码复算 `yep2`，量化真实 verifier 绑定同次 grant 后只输出脱敏视图，前端运行时校验并实际渲染总额 `1250.000000`、可用 `900.000000`、卖回占用 `100.000000`、量化占用 `250.000000`、总占用 `350.000000 ESK`。共享 fixture 规范 LF SHA-256 为 `3984e97f16ce83ec401dcf4494f10f11f5f7e8379d26f367fa68a4a8fd9ddf8e`；该固定数值只属于公开测试向量，不是用户余额。验收仍为 Paper-only，不上链、不移动资金、不创建仓位、不开始交易或收益；证据见 `docs/esk-paper-quant-cross-repo-asset-view-v5-acceptance.md`。
- 加入前公开预览与 Paper 启动严格分离：预览是匿名 GET，只展示内置目录白名单；它不登录、不加入、不签发 grant，也不触发量化页面或任何写接口。需求与验收边界见 `docs/requirements/yilong-quant-public-project-preview-v1.md`。
- 既有 `net-balance-lock-receipt-v1` 是 ESK 定名前的 legacy Paper 合同，不会自动改名、兑换或并入 ESK。当前 ESK Schema 也只用于 Paper 只读联调；主项目尚未部署链上 ESK、真实锁定/结算、价格或生产准入，因此不能把投影当作真实发行或量化申购回执。
- 子仓库分支 `codex/fund-accounting-v18-20260903` 的实现提交 `43d82e68125ec6fc3f7e8c7db57b5e3151e3553e` 已用核心类型和测试固定 ESK 市场定价、消费后分配权重下降、协议余额排除、`QSHARE` NAV/申赎、稳定储备隔离和逐级上线门卫；证据提交为 `b394e4f`。只读 `GET /api/v1/fund/readiness` 仍返回 Paper、客户资金关闭、实盘关闭和生产凭据不存在，不代表该分支已合并或部署。

## 尚未完成

### 正式登记原生摘要提供端 V1

主项目已另设 [正式摘要授权需求](requirements/esk-platform-native-provider-v1.md) 和
[21 字段合同](contracts/esk-platform-android-snapshot-v1.md)，复用正式账本本人读取，
不放宽下述 Paper 协议。它只向新的官方量化原生组件返回本次确认账户的短期总量/笔数，
不传身份、凭据或流水，不自动绑定量化网页登录账户，也不授予交易或持续读取权。
量化接收端 V28 已推送：实现 `e009c164bd04076d97d9481bf77b88110758fd62`，
证据 `72ed8eac1edd06f2be5e7ab08bf5c622600d55cf`；`com.elon.quant 0.4.0 (4)`
签名构建完成但仍缺项目发布凭据，尚未上传，不能当作项目广场已可安装。
量化仓交接入口为 `docs/delivery/v28-formal-esk-native-consumer.md`；
提供端发布见 [交付记录](reports/esk-platform-native-provider-v1-delivery-20260904.md)。
两端本人授权联调、受保护主服务连接及已核对付款审批仍分别验收。

主项目正式账户 V1 保持最近记录摘要；完整审核流水另设
[分页 V1](requirements/esk-platform-history-v1.md) 与 [交付证据](delivery/esk-platform-history-v1.md)。
翻页绑定本人完整账本摘要，账本变化要求重读；全历史不进入任何双 APK 摘要协议。

### 原生 ESK 快照 V1 / 量化 V24

双 APK 原生只读快照已实现：[需求](requirements/esk-native-snapshot-v1.md)、
[17 字段合同](contracts/esk-android-snapshot-v1.md)。量化原生页面不接收主登录凭据，
确认页只读取 HTTPS 主服务，HTTP 在读取 token 和创建请求前拒绝。
双方使用当前正式签名、固定组件与一次性 nonce；快照只在内存展示最多 60 秒，
后台/重建清空，金额守恒，当前只允许 Paper 平台登记、未上链、未移动资金。
它不是网页登录，也不接通 HTTP 公共页面的本人仓位/投影接口。
构建、发布和用户验收分别见 [交付记录](esk-native-snapshot-v1-delivery.md)。
Android 签名认证及本机 Activity result 没有 Web 镜像；既有 PWA 资产卡与受控投影协议保持原样。

- 在真实 Android 设备完成项目广场下载、系统安装、签名兼容、打开 `/quant/`、返回/重试和公开 Paper 回测验收；V24 原生快照仍需双正式 APK 与受保护主服务联调，V17/V20 已发布版本不传主项目 bearer、Paper grant 或 ESK 投影。
- 本人 ESK/仓位、量化申请 accepted/released 回执等敏感能力仍需批准的加密传输和应用绑定方案；当前 HTTP `/quant/` 不得启用这些功能。随后还需完成主项目签名私钥托管和一次真实轮换演练。代码已支持量化端多 key 重叠、密钥级撤销和单 grant 跨重启撤销；当前用户 grant 仍只绑定 paper 模拟参与者，不证明付款、KYC、钱包或真实准入。
- 经来源与许可评审、版本化落盘并用于可复现回测的真实公开历史行情、多策略比较、回测结果持久化和共享算力分片；V19 的 Binance Spot REST 数据只是按需公开展示快照，不写入研究 `market-data`、不进入回测或 NAV。
- 将已付款用户数据经审核、脱敏、对账后导入生产系统；paper 子项目只允许脱敏标识和模拟锁定回执，继续禁止把聊天或付款截图写入代码。
- 用户自助追加模拟配额、真实申购或真实 NET 锁定；当前 V6 追加能力只接受独立运营令牌，主项目 grant 不获得增加余额的 scope。
- 官方 NAV、交易所 sandbox/live、托管、KYC/地区准入和真实提现。
- Feature Registry 工具可用后，需分别登记项目广场接入、paper 参与账本、身份授权和 NET 锁定，不把未完成范围提前标记为 verified。
