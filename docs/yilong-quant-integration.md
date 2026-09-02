# 一龙量化交易子项目接入与协作边界

## 结论

一龙量化交易使用独立仓库 `git@github.com:ElonQian1/yilong-quant.git` 开发，通过主项目的官方项目目录和项目落地清单进入项目广场。它不是主仓库内的源码子目录，也不沿用“一龙网游加速器”的历史默认加入逻辑。

这种结构让量化项目可以独立迭代、测试和发布，同时复用一龙的项目广场、身份授权协议、通用回执和 Rust 编译缓存。双方不共享业务数据库、Redis、交易密钥、客户资金或原始 Cargo target。

## 用户现在能看到什么

- 项目广场中的“一龙量化交易”标题、简介、目标用户、当前更新和 GitHub/文档入口。
- 登录用户在主项目账号页可以查看本人 ESK Paper 总额、可用额、卖回占用、量化分配申请占用和总占用，并可提交或取消尚未被量化端接收的 Paper 申请；在量化公开环境配置就绪后，从量化项目主页一键进入还会看到同一组余额的签名只读投影。短期授权和投影不进入 URL 或浏览器持久存储。
- 项目主页当前更新会说明已经验证的 `BACKTEST / SIMULATION` 研究能力：行情来源、策略版本、费用、滑点、权益曲线、回撤和确定性摘要均可追溯。
- 明确的 `paper` 阶段：模拟委托、运营审核追加模拟份额、NAV 与用户部分/全额退出结算。
- 6% 仅为非保证目标；当前不导入真实付款名单、不移动资金、不连接实盘。
- Web、Windows 和 Android 尚未公开部署时继续显示“计划中”，不生成虚假下载入口。

主项目官方目录只为量化项目创建公开只读项目和首页快照，不给它绑定 ERP 蓝图安装动作。

## 仓库责任

| 责任 | 主项目 | `yilong-quant` |
|---|---|---|
| 项目广场、公开只读入口 | 负责 | 提供 `.elon/project-landing.json` 内容真源 |
| 用户身份、ESK Paper 账本、卖回/量化申请占用和未来锁定 | 负责 | 只消费版本化授权/只读投影/未来接收回执，不从投影创建仓位 |
| ESK 发行和可用余额 | 主项目是真源；当前只完成 Paper 登记 | 不铸造、不直接修改、不保存余额副本 |
| 模拟份额、运营追加与部分/全额退出规则 | 不复制 | 负责 |
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
- 子仓库 `77ff21c6ce3d0984273ad6e3ad526faff0d46b7c` 已验证版本化研究行情和单机可复现回测 V1；主项目只同步净化后的能力摘要与文档入口，不接收行情数据、策略代码、订单或回测结果。
- 子仓库 `25ee7df69fecabb696e7b8329fd388bc8a19da11` 已新增只读、脱敏的 Paper 部署配置预检：`scripts/check-paper-deployment.ps1` 检查本机绑定、非零固定端口、绝对数据库路径、主项目 HTTPS 来源、grant 验签公钥和操作员令牌等必需配置。输出不包含密钥或令牌，执行时不绑定端口、不打开数据库、不访问外部网络，因此只证明配置是否就绪，不证明目标环境已部署。
- 子仓库 `effef80b65647b2341cb902e64bd9aa171fdb613` 已实现受独立运营令牌保护的 `yilong.quant.paper_operations_snapshot.v1`：它只汇总模拟仓位状态、精确金额、NAV 修订、SQLite 事件头和导入批次，不返回参与者标识。主项目不复制或直接修改该账本，快照也不证明真实付款、NET 锁定、官方 NAV 或可提现余额。
- 子仓库实现提交 `7e943798050a63929e439a342cefa9b4f589028c`（交付证据 `520d15bb6b88d4f281efb4dd3ac46fc71877e847`）已完成 Paper 模拟仓位操作 V6：运营令牌可为已有活跃仓位追加模拟 NET 配额并按当前 NAV 增发内部份额；用户既有 `paper.redemption.request` scope 可申请部分或全额退出，部分结算后剩余份额继续按模拟 NAV 承担盈亏。新事件追加写入 SQLite 并兼容历史全额退出事件；用户不能自行增加余额，所有数据仍是 `simulated=true`。
- 子仓库提交 `2c9327ec94ff2a80e1a94115de9aa784eb63b5f7` 已完成 Paper 授权密钥轮换 V7：量化 verifier 固定信任 1–8 个公开 Ed25519 key，支持新 active 与旧 retiring key 按签发时间窗重叠验证，并让 revoked key 立即失败关闭；旧单 key 配置保持兼容但不能与 managed keyring 混用。主项目仍只持有一个当前签名私钥，双方不共享 seed、subject secret 或动态网络信任。
- 子仓库提交 `ba56940dffdffda6e487a5ae8ea2a577431119fc` 已完成 Paper 单次授权持久撤销 V8：用户只能撤销当前已验签 grant；量化服务只在本地 SQLite 保存 `grant_id`、过期时间和撤销时间，重启后继续拒绝该 grant，不保存完整 token、participant、账户或仓位。该动作只结束本次访问授权，不退出模拟仓位、不修改主项目会话，也不构成真实提现。主项目 grant 载荷、五分钟有效期、scope、签发和 exact-origin 内存传递协议保持不变。
- 子仓库提交 `0b87604e9105d7b0c1e4ba0da6b8b2c3c43d6ddc` 已完成 Paper 公开部署合同 V9：API `/api/health` 绑定编译时 40 位 Git SHA，仓库提供不含秘密的 loopback systemd、标准 HTTPS 443 Nginx 和环境模板，以及只发送 PWA `/`、`/api/health`、`/api/v1/runtime` 三个无凭据 GET 的公网验收器。它会拒绝 HTTP/loopback、错误提交、宽松 CSP/CORS、响应秘密标记、非 Paper runtime、`live_trading_enabled=true`、`funds_moved=true` 或错误 parent origin。离线 fixture 已验证但没有访问目标环境，因此当前只达到 `deployment_contract_verified`。
- V9 固定区分三种状态：`configuration_ready` 只证明量化进程配置形状，`deployment_contract_verified` 只证明模板和离线合同，`environment_deployed` 才表示批准的量化 HTTPS origin 已由 `scripts/check-paper-public-deployment.ps1` 真实读取并返回 `scope=public_https_read_only`、`network_calls_made=true`、`status=ready`。主项目只能在第三种状态后配置量化 Paper Web URL；不得用当前 HTTP 主站、未知 HTTPS 主机、关闭证书校验或放宽 exact-origin/CSP 临时上线。
- `contracts/quant/net-balance-lock-receipt-v1.schema.json` 已定义主项目未来锁定 NET 后交给量化项目消费的版本化回执形状；详细语义见 `docs/yilong-quant-net-lock-receipt-v1.md`。
- `POST /api/me/quant/paper-access-grants` 已复用主项目现有 bearer 会话，可在独立签名配置启用后签发最多五分钟的 Ed25519 paper grant；量化项目只获得项目专用脱敏 subject 和明确 scope，不获得主项目 bearer 或用户资料。契约见 `docs/yilong-quant-paper-access-grant-v1.md` 与 `contracts/quant/paper-access-grant-v1.schema.json`。
- `GET /api/me/quant/paper-launch` 与 `POST /api/me/quant/paper-launches` 提供失败关闭的 readiness 和一次性启动票据；PC 项目主页通过 exact-origin iframe、`event.source`、nonce、attempt ID 和过期时间绑定，把 grant 只传给当前量化子页面。双方契约见 `docs/yilong-quant-paper-launch-v1.md` 与 `contracts/quant/paper-launch-v1.schema.json`。
- 主项目从唯一 ESK Paper 账本签发最长五分钟的只读资产投影：量化页面优先声明并接收 `yilong.esk.asset_projection.v2` / `yep2`，显示总额、可用额、卖回占用、量化申请占用、总占用、源修订和同步时间；旧页面仅在量化占用为零时安全回退 V1。投影与同次 grant 的 grant ID、脱敏 participant、key、签发和到期边界精确绑定，既不是申购回执，也不会创建仓位、交易或收益。主项目已随 `v0.3.1715` 发布，回执见 `docs/esk-paper-quant-allocation-request-v2-release-v0.3.1715.md`；量化子仓库实现提交 `1210e8b`、功能登记发布提交 `bc9d3de` 已推送，双方 V2 Schema SHA-256 均为 `ba3748fe22122e99271b5b6a0aeaa7fd61206557f22e67c807c26a0a97036c57`；契约见 `contracts/quant/esk-paper-asset-projection-v2.schema.json`，V1 继续兼容。
- 加入前公开预览与 Paper 启动严格分离：预览是匿名 GET，只展示内置目录白名单；它不登录、不加入、不签发 grant，也不触发量化页面或任何写接口。需求与验收边界见 `docs/requirements/yilong-quant-public-project-preview-v1.md`。
- 既有 `net-balance-lock-receipt-v1` 是 ESK 定名前的 legacy Paper 合同，不会自动改名、兑换或并入 ESK。当前 ESK Schema 也只用于 Paper 只读联调；主项目尚未部署链上 ESK、真实锁定/结算、价格或生产准入，因此不能把投影当作真实发行或量化申购回执。

## 尚未完成

- 为量化项目分配并批准独立 HTTPS origin、证书和服务器权限，在目标环境执行 Paper 配置预检与脱敏运营快照检查，发布由提交身份绑定的 PWA/API，再运行 V9 公网只读验收；通过前官方目录 Web 继续为 `planned`，主项目不得配置 Paper Web URL。随后还需完成主项目签名私钥托管和一次真实轮换演练。代码已支持量化端多 key 重叠、密钥级撤销和单 grant 跨重启撤销；当前用户 grant 仍只绑定 paper 模拟参与者，不证明付款、KYC、钱包或真实准入。
- 经来源与许可评审的真实公开历史行情、多策略比较、回测结果持久化和共享算力分片；当前只有仓库内置 CC0 确定性研究 fixture 与单机基准策略。
- 将已付款用户数据经审核、脱敏、对账后导入生产系统；paper 子项目只允许脱敏标识和模拟锁定回执，继续禁止把聊天或付款截图写入代码。
- 用户自助追加模拟配额、真实申购或真实 NET 锁定；当前 V6 追加能力只接受独立运营令牌，主项目 grant 不获得增加余额的 scope。
- 官方 NAV、交易所 sandbox/live、托管、KYC/地区准入和真实提现。
- Feature Registry 工具可用后，需分别登记项目广场接入、paper 参与账本、身份授权和 NET 锁定，不把未完成范围提前标记为 verified。
