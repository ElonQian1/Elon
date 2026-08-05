# PC 工作台前端迁移路线

> 本文记录 `/pc` PC 工作台从原生静态 HTML/CSS/JS 迁移到 `Vite + React + TypeScript` 的路线、边界和模块状态。硬规则见 `.github/instructions/pc-frontend-migration.instructions.md`。
>
> 最后状态校准：2026-08-05

## 当前判断

- `/pc` 已切到 `pc-frontend/` 的 Vite + React + TypeScript 前端，构建产物由服务器 `$DATA_DIR/pc-next-dist/` 托管。
- 旧 `server/src/assets/pc_*.html/js/css` 源码和旧 `/assets/pc_*` 路由已经删除；`/pc-legacy` 只作为发布脚本从历史提交导出的只读对照快照。
- 本文现在主要用于记录 PC 前端模块真实状态、剩余缺口和后续协作边界；新 PC 功能继续进入 `pc-frontend/src/features/`。

## 目标形态

```text
pc-frontend/
  src/
    app/
    routes/
    features/
    components/
    api/
    styles/
  package.json
  vite.config.ts
  tsconfig.json

server/src/
  web.rs      # 托管 /pc、/pc-next 和构建产物
  router.rs   # 注册路由
```

迁移主线已经完成：`/pc` 指向新前端；旧 `server/src/assets/pc_*.html/js/css` 已删除；`/pc-legacy` 仅保留为只读对照，不恢复旧源码。

## 分阶段迁移计划（长远路线图）

> 每阶段完成后更新下方状态表。单次 AI 任务只迁移一个明确模块。

### 阶段 P1 — 对话核心补全

目标：让 `/pc` 的消息体验与旧版对齐。

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| Markdown 渲染（代码块/表格/列表/引用/链接） | 🔴 高 | `src/features/markdown/` | ✅ P1.1 完成 |
| OneNote 式项目文档工作区（按需读写、Markdown 编辑/预览、低 token 项目图谱） | 🔴 高 | `src/features/project-docs/` | ✅ 已实现知识首页、最多四层主/副主题树、四维治理总览和可执行健康中心；项目图谱以 Rust 后端机器契约分别展示产品功能、技术架构和文档主题，文档覆盖与 `file:/test:/route:/symbol:` 实现证据分开；独立“讨论推理”分区可导入聊天、按根主题和分支展示推理过程，并从节点继续、分叉或晋升文档。讨论图现已具备 Git 语义版本时间轴、旧版只读回看、版本差异、节点生命周期追踪、确定性质量审查和可逆修正入口；Windows 登录账号的 AI 可通过 MCP 直接审查、生成安全修正提案、保存并发布新版本，无须人工点击网页。网页和供应商无关 MCP 共用同一事实来源，目录/图谱阶段零模型 token、零正文读取；聊天原文先以低权威来源保存，再由 AI 按需读取与编译。原生理解候选收件箱支持批量审核、摘要/topic CAS 修订、漂移项拒绝清理；只有审核并应用的短导航记忆才经 Git 跨 PC 共享。右键、`⋯`、`Shift+F10` 与触摸长按共用分区命令，个人排序留在浏览器，共享结构和审计写清单；健康问题支持筛选、分派、期限、原因化忽略/延期、选中后 AI 建议、趋势与评分解释，普通 Git 和托管知识库都可查看差异并安全恢复；默认整理前/后双提交，可安全 rename/move Markdown，不夹带代码、不自动 push |
| 频道 WebSocket 实时消息推送 | 🔴 高 | `src/features/conversation/useChannelSocket.ts` | ✅ P1.2 完成 |
| 消息流式输出（AI 打字指示器 + 智能滚动）| 🟡 中 | `src/features/conversation/` | ✅ P1.3 完成 |
| 统一实时刷新架构（事件标准化 + 资源 key + 共享刷新 hook） | 🔴 高 | `src/features/realtime/` | ✅ P1.8 完成 |
| 消息附件上传（图片/文件） | 🟡 中 | `src/features/conversation/AttachmentButton.tsx` | ✅ P1.4 完成：支持不限扩展名选择、整会话区拖放、粘贴、多附件预览，并随 AI 任务发送结构化 attachments |
| 任务状态实时更新（task_done 事件） | 🟡 中 | `src/features/dev/` | 🟡 P1.7 已补过程覆盖诊断、当前卡点阶段、结构化命令/文件/测试过程卡、侧栏心跳陈旧提示、运行中停止入口、恢复控制面、项目级多任务现场总览；task_done 细化仍继续 |
| 多端后台微调画布 | 🔴 高 | `src/features/ui-tuner/headless-design/` | 🟡 已接入 Web/PWA/Tauri/Android designSession、revisioned DesignIntentPlan 与动作回执/暂停/重规划、AI task lease、cursor + checkpoint、DraftOperation v2、绑定漂移健康、分平台写回审批、source patch 二次审批/原子应用/回滚计划、修改前后回归任务、Tauri 分层证据、验证矩阵和右侧默认对话；画布跟随指定任务。代码尚未编译或平台运行验收，安装节点仍待升级到 MCP schema v1.11 |
| 本机任务桌面监督证据 | 🔴 高 | `src/features/local-tasks/` | ✅ 识别 `elon.desktop_pc_supervision.v1`，展示执行/监督分工、验收条件、精确命令退出码/文件证据/失败摘要和最新桌面验收结论；运行卡显示 phase、脱敏 current command、last progress、heartbeat、idle duration 与 timeout policy，并识别 `resume_required`；普通本机任务保持兼容 |

### 阶段 P2 — 项目管理

目标：项目详情、成员、设置可以在新前端操作。

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 项目详情页（概览/频道/成员/工作区/发布/设置/群体 AI Tab）| 🔴 高 | `src/features/projects/ProjectDetailPage.tsx` | ✅ 已产品化：详情页已拆为 tab 编排器，频道、成员、工作区/Git、发布/APK、设置、群体 AI 都有入口 |
| 成员列表 + 邀请 + 移除 | 🟡 中 | `src/features/projects/ProjectDetailPage.tsx::MembersTab` | ✅ 已完成第一版：列表、搜索、筛选、排序、邀请、单个/批量移除、禁言/解禁已在详情页内联实现 |
| 项目任务进度面板 | 🟡 中 | `src/features/projects/ProjectReadinessCard.tsx`、`src/features/dev/` | 🟡 已有工作区就绪度卡、AI 开发频道入口和项目级任务现场；任务历史/多任务进度仍继续在 dev 面板细化 |
| 项目工作区设置（Git/Node） | 🟢 低 | `src/features/projects/WorkspaceAccessPanel.tsx`、`ProjectGitSettingsPanel.tsx` | ✅ 已补本机目录选择、节点重绑、Route A 开发期完全访问自动登记、健康检查、Git remote/branch 配置和 Deploy Key 管理；自动登记只接受当前登录账号下与本机节点、项目和目录一致的云端绑定，显式选择目录仍走注册与授权 API，不重复弹窗 |
| 频道管理（新建/改名/删除）| 🟢 低 | `src/features/projects/ProjectChannelsTab.tsx`、`server/src/project_channels.rs` | ✅ 已完成：支持频道新建、改名、删除自定义频道；默认频道有删除保护，改动后刷新项目空间 |
| 发布历史 / APK 管理 | 🟢 低 | `src/features/projects/ProjectReleasesTab.tsx`、`server/src/project_releases.rs` | ✅ 已完成第一版：项目详情页可查看 releases、下载 APK、上传 APK 并记录版本/渠道/变更说明 |
| AI 原生开放商业网络 | 🔴 高 | `src/features/open-commerce/`、`server/src/open_commerce_*` | ✅ 已形成五工作区；消费者沙盒可按有限能力 Schema 生成表单，动作修改后重新确认。公共网络、真实调度、真实平台适配器和链上提交仍按各自边界推进 |
| 算力结算页 | 🟡 中 | `src/features/compute-settlement/`、`server/src/compute_federation_settlement_*` | 🟡 `/my-compute-settlement` 已写入 Provider 自助登记、本人余额、提款状态、申请和 pending 取消源码；管理员 `/compute-settlement` 已写入平台账户、到期释放、全局队列、拒绝退款和外部已付款证明登记源码。登记只创建 self-declared 记录，两页不执行真实付款，尚未构建、视觉验收或发布 |
| 算力供给工作区 | 🟡 中 | `src/features/compute-supply/`、`server/src/compute_federation_capacity_*` | 🟡 `/compute-supply` 已写入本人 Provider 下的 Pool/Bucket/Supply、账本审计，以及激活证据本人申请、历史、预检和取消源码。证据入口只收引用和摘要，不保存凭据正文；申请或 approved 均不自动激活，普通页面没有管理员审核权。不发布 Offer、不派发任务或结算收入，尚未构建、视觉验收或发布 |
| 算力激活审核工作区 | 🔴 高 | `src/features/compute-activation/`、`server/src/compute_federation_activation_*` | 🟡 仅 `admin/owner` 可见的 `/compute-activation` 已写入审核队列、证据预检、批准/退回/拒绝、过期 approved 废止、不可变计划准备与二次预检、精确摘要应用、应用回执和 active 紧急隔离源码。准备不激活；应用只改变平台内部 Provider/Pool 状态且不发布 Offer；隔离不关机、不退款、不撤销既有合同。尚未构建、接口联调、视觉验收或发布 |
| 算力 Offer 与报价工作区 | 🔴 高 | `src/features/compute-supply/`、`src/features/compute-offers/`、`server/src/compute_federation_{offer,price_snapshot}_*` | 🟡 `/compute-supply` 已写入本人 Offer 草稿生命周期，以及 active Offer 的 fallback_curve 报价历史和显式发布；仅 `admin/owner` 可见的 `/compute-offers` 已写入待审、发布、draining 退场及 expired/revoked 终结。操作绑定当前版本、摘要、稳定幂等键和明确确认；报价可进入候选发现，但不创建 Job、不预留容量、不冻结余额或自动成交，尚未构建、接口联调、视觉验收或发布 |
| 消费者算力市场工作区 | 🔴 高 | `src/features/compute-market/`、`server/src/compute_federation_broker_*` | 🟡 所有登录用户可见的 `/compute-market` 已写入项目级本人 Job 列表、submitted 创建、候选发现、不可变锁价、Reservation 列表、逐 meter 预算与容量预留，以及未执行任务 Release/Expire 源码。金融动作要求稳定幂等键、精确版本/摘要和明确确认；不创建 Attempt、不派发节点、不处理运行中取消或最终结算，尚未构建、接口联调、视觉验收或发布 |
| 消费者算力验收工作区 | 🔴 高 | `src/features/compute-market/ComputeConsumerReview*`、`server/src/compute_federation_attempt_*` | 🟡 所有登录用户可见的 `/compute-reviews` 已写入本人待审核 Provider 终态候选队列，以及 `accepted/rejected/disputed` 第一份消费者证据表单。队列按消费者过滤并排除已有审核；页面只展示候选摘要与工件引用，不下载或验证工件，不触发 Verification、付款、退款、容量消费或结算，尚未构建、接口联调、视觉验收或发布 |
| 算力执行工作区 | 🔴 高 | `src/features/compute-execution/`、`server/src/compute_federation_attempt_*` | 🟡 所有登录用户可见的 `/compute-execution` 已写入本人 Provider 选择、“执行 Lease/待激活”分段队列、首次 Attempt 激活、Lease 查询/续租、revision 1 无心跳 staging 中止、逐 meter 累计用量和首份 Provider 终态候选源码。用量模板由服务端从当前合同生成；激活、心跳、用量和候选均只是外部声明，中止才原子退款并归还容量。不发送节点命令、不验证外部证明、不形成可信用量或结算，尚未构建、接口联调、视觉验收或发布 |
| 平台算力观测工作区 | 🔴 高 | `src/features/compute-observations/`、`server/src/compute_federation_attempt_*` | 🟡 仅 `admin/owner` 显示的 `/compute-observations` 已写入待观测候选、最终 Provider meter 模板，以及来源、结果、完整累计 meter 和证据引用表单。保存只形成第一份 `unverified_platform_observation`，不自动验证来源或证据，不推进 Attempt/Job/容量，不移动资金或生成结算，尚未构建、接口联调、视觉验收或发布 |
| 平台算力验证工作区 | 🔴 高 | `src/features/compute-verification/`、`server/src/compute_federation_attempt_*` | 🟡 仅 `admin/owner` 显示的 `/compute-verification` 已写入待验证 v189-v191 证据链、三方 outcome、声明/观测/验证/可补偿 meter 预览，以及 `accepted/rejected/disputed` 人工决定表单。前端限制明显不满足保守策略的接受操作，服务端仍在事务内重新审计；决定不签发 Execution Receipt、不推进任务或容量、不移动资金或形成结算，尚未构建、接口联调、视觉验收或发布 |
| 执行回执签发工作区 | 🔴 高 | `src/features/compute-receipts/`、`server/src/compute_federation_attempt_*` | 🟡 仅 `admin/owner` 显示的 `/compute-receipts` 已写入 accepted Verification 待签发队列、终态与输出工件引用、verified/compensable meter 和显式确认。队列与签发事务都会重新审计上游证据；页面不下载或验证工件正文，签发不推进 Attempt/Job/容量、不移动资金或形成结算，尚未构建、接口联调、视觉验收或发布 |
| 可信终态收口工作区 | 🔴 高 | `src/features/compute-finalization/`、`server/src/compute_federation_attempt_finalization_*` | 🟡 仅 `admin/owner` 显示的 `/compute-finalization` 已写入待收口 Execution Receipt 队列、精确 Lease/Job/Reservation/Claim 版本与摘要、可补偿用量和状态/容量影响。管理员必须逐笔勾选并输入确认文本；服务端在立即事务中再次核对模板后才推进状态和容量，预授权与结算仍不变。页面尚未构建、接口联调、视觉验收或发布 |
| 待结算回执工作区 | 🔴 高 | `src/features/compute-settlement/ComputeSettlementIssuancePage*`、`server/src/compute_federation_attempt_settlement_*` | 🟡 仅 `admin/owner` 显示的 `/compute-settlement-issuance` 已写入 v194 待结算队列、服务端金额预览和精确资金绑定。管理员必须逐笔勾选并输入确认文本；v195 会结清消费者平台内预授权、退款并登记 Provider/平台 pending，但不释放、提现或调用外部付款。页面尚未构建、接口联调、视觉验收或发布 |
| 消费者结算申诉工作区 | 🔴 高 | `src/features/compute-settlement/ComputeSettlementChallengePage*`、`src/features/compute-attempt/settlementChallengeContracts.ts`、`server/src/compute_federation_attempt_settlement_challenge_*` | 🟡 所有登录用户可见的 `/compute-challenges` 已写入本人 v195 待申诉队列、既有扣结/退款/pending 金额、原因、事实摘要、证据引用和明确确认。候选只包含 72 小时内未挑战、未释放的本人回执；提交 v196 只登记不可变申诉并阻断后续释放，不立即退款、不移动余额、不调用银行、钱包或 Sui。页面尚未构建、接口联调、视觉验收或发布 |
| 结算申诉撤回、裁决与历史工作区 | 🔴 高 | `src/features/compute-settlement/*ChallengeResolution*`、`src/features/compute-settlement/SettlementChallengeHistoryList.tsx`、`server/src/compute_federation_attempt_settlement_challenge_resolution_*` | 🟡 `/compute-challenges` 已写入本人 open 申诉、不可变撤回和本人历史，只有 `admin/owner` 显示的 `/compute-challenge-resolution` 已写入全局 open 队列、accepted/rejected 裁决和全局历史。历史逐项审计 v195-v199 并显示内部余额状态，但不证明外部付款；页面尚未构建、接口联调、视觉验收或发布 |
| accepted 结算纠正工作区 | 🔴 高 | `src/features/compute-settlement/*SettlementCorrection*`、`src/features/compute-settlement/CorrectSettlementDialog.tsx`、`server/src/compute_federation_attempt_settlement_correction_*` | 🟡 仅 `admin/owner` 显示的 `/compute-corrections` 已写入 accepted 待纠正队列、v195-v197 证据链、整数 fen/micros 输入、守恒预览、证据引用和双重确认。v199 会向消费者平台内余额退款并冲减 Provider/平台 pending，随后 v198 仅释放净额；它不调用或证明银行、钱包或 Sui 退款，页面尚未构建、接口联调、视觉验收或发布 |
| Provider 结算申诉历史 | 🔴 高 | `src/features/compute-settlement/SettlementChallengeHistoryList.tsx`、`src/features/compute-settlement/MyComputeSettlementPage.tsx`、`server/src/compute_federation_attempt_settlement_challenge_resolution_*` | 🟡 `/my-compute-settlement` 已按当前 Provider 写入影响其收益的申诉、裁决、纠正和释放历史；后端先校验 Provider 所有权，再逐项重审计 v195-v199。available 只表示平台内部余额，不证明外部付款，页面尚未构建、接口联调、视觉验收或发布 |

### 阶段 P3 — 个人 AI 对话

目标：旧版 "一龙AI" 频道 → 个人 AI 会话管理。

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 个人 AI 会话列表（`/api/me/ai/conversations`）| 🔴 高 | `src/features/ai/AiChatPage.tsx` | ✅ 已完成第一版：`/pc/ai` 读取 `/api/me/ai/conversations?limit=50`，按项目分组展示历史会话 |
| AI 会话消息页 | 🔴 高 | `src/features/ai/AiChatPage.tsx` | ✅ 已完成第一版：支持消息读取、发送、节点状态提示、快捷工具和 Codex 保险箱快捷动作 |
| 会话历史分页加载 | 🟡 中 | `src/features/ai/AiChatPage.tsx` | 🟡 部分完成：会话和消息使用固定 `limit`，侧栏有展开/折叠；缺真正分页、加载更多和增量刷新 |

### 阶段 P4 — 好友 & 社交

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 好友列表 + 在线状态 | 🟡 中 | `src/features/friends/FriendsPage.tsx` | ✅ P4.1 完整 presence 实时同步 |
| 好友私聊消息 | 🟡 中 | `src/features/friends/FriendChat.tsx` | ⬜ 未开始 |
| 添加好友 / 扫码 | 🟢 低 | `src/features/friends/AddFriend.tsx` | ⬜ 未开始 |

### 阶段 P5 — 项目广场 & 发现

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 项目中心双 Tab（我的项目 / 项目广场）| 🔴 高 | `src/features/projects/ProjectsPage.tsx` | ✅ 已恢复；项目卡和左侧项目入口现在直接进入会话工作台内的项目主页，不再经过重复资料中转页；设置与成员管理保留为次级入口 |
| 项目工作台右侧上下文栏 | 🔴 高 | `src/features/conversation/ProjectContextSidebar.tsx` | ✅ 已重构为“项目 / 成员”双 Tab：项目页集中展示并可复制节点、工作目录和项目标识，Owner 可快速修改 Logo；成员页保留邀请、范围切换与成员级权限操作。个人状态、移动端、算力和旧版入口统一归入用户菜单，不再占用项目上下文。 |
| 项目广场列表（过滤/搜索/cursor 分页）| 🟡 中 | `src/features/plaza/ProjectPlazaView.tsx` | ✅ 已完成，`/pc/plaza` 与项目中心广场 Tab 复用同一视图；PC 新前端默认使用 `page_mode=cursor` 加载更多，旧 offset 仅保留兼容 |
| 加入/申请加入项目 | 🟡 中 | `src/features/plaza/` | ✅ 已完成，加入后刷新我的项目 |
| 项目卡片分享 | 🟢 低 | `src/features/plaza/ProjectCard.tsx` | ⬜ 未开始 |

### 阶段 P6 — 账号 & 设置

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 账号信息 + 修改密码 | 🟡 中 | `src/features/account/AccountPage.tsx` | 🟡 部分完成：账号信息、头像展示和昵称修改已完成；修改密码入口仍未补 |
| 绑定手机/邮箱 | 🟢 低 | `src/features/account/` | ⬜ 未开始 |
| 积分/余额查看 | 🟢 低 | `src/features/account/AccountPage.tsx`、`src/features/billing/` | 🟡 部分完成：账号页已展示余额、试用额度和最近账单；独立账单分页/充值页继续补 |

### 阶段 P7 — 移动端适配 & 收尾

| 任务 | 优先级 | 说明 | 状态 |
|---|---|---|---|
| 移动端响应式（< 780px）| 🟡 中 | 面板折叠、滑动抽屉 | ⬜ 未开始 |
| 键盘快捷键（Esc 关弹窗等）| 🟢 低 | 体验细节 | ⬜ 未开始 |
| PWA 离线壳 | 🟢 低 | service worker | 🟡 基础壳完成：缓存 `/pc` shell 和当前 hash 资源；浏览器要求 HTTPS/localhost 安全上下文 |

---

## 执行原则

项目文档工作台现已包含“知识首页 / 项目图谱 / 讨论推理 / 主题分区 / 治理总览 / 文档健康 / AI 整理建议”。主题树支持一个主主题和多个副主题；讨论推理图管理聊天来源、推导分支和正式知识晋升，不把旧讨论当作当前知识；治理总览把检索、生命周期、权威性和文档类型分开筛选编辑。“文档健康”直接展示目录 API 的服务端 `analysis`，包括可处理的问题状态、负责人和期限、趋势、评分解释、持续维护、联邦模块、Git 差异与安全恢复；前端本地分析只作为旧节点兼容回退，不能形成第二套健康度真源。

1. **每次只做一个任务**：单次 AI 对话只迁移上表中一个模块，完成后更新状态为 ✅
2. **优先补真实缺口**：P1 对话核心已基本迁移，后续优先处理任务终态细化、项目设置、频道管理、AI 会话分页和移动端适配
3. **有 API 先看路由**：开始前先查 `server/src/router.rs` 确认 API 端点
4. **实时刷新复用**：`/ws/app` 消息先进入 `src/features/realtime/`，统一标准化为公开事件和资源 key；页面模块只声明自己关心的资源，并用共享刷新 hook 触发加载
5. **Markdown 统一**：渲染逻辑放 `src/features/markdown/`，不在各页面各自实现

## PC 实时刷新架构

PC 前端动态刷新分三层，后续新增模块必须接入这条链路，避免每个页面重复写 WebSocket 监听、轮询和焦点刷新：

1. `src/features/notifications/useNotifications.ts` 只负责连接 `/ws/app`，收到后端消息后调用 `normalizeRealtimeEvent`，同时保留旧 DOM 事件名给未迁移模块兼容。
2. `src/features/realtime/` 负责把后端事件映射为资源 key，例如 `project.space:{projectId}`、`channel.messages:{projectId}:{channelId}`、`conversation.any:{projectId}:{conversationId}`、`task.timeline:{projectId}:{taskId}`。
3. 页面或业务 hook 使用 `useRealtimeResourceRefresh`，传入 `resourceKeys`、`refresh` 和必要的兼容判断函数。通用 hook 统一处理事件防抖、运行中短轮询、完成后短暂收尾轮询、窗口重新聚焦刷新。

当前已接入模块：

- 当前频道消息刷新：`src/features/conversation/useChannelAutoRefresh.ts`
- 项目成员会话消息、任务过程和最终回复刷新：`src/features/conversation/useConversationRealtimeRefresh.ts`

新增实时模块时不要直接在页面组件里 `window.addEventListener('elon:...')`；优先新增或复用资源 key，再用共享 hook 连接加载函数。

---

## 下一步行动（状态校准后）

当前不再把 P1.1 Markdown 渲染作为最高优先级；它已经完成。后续优先级按真实缺口排列：

1. P1.7：继续细化 `task_done`、任务终态、恢复态和公开过程 UI 的实测验收。
2. P2：补项目基础资料设置、项目删除/转让、频道新建/改名/删除和 Git remote/branch 设置。
3. P3：补个人 AI 会话真正分页、加载更多、增量刷新和大历史检索。
4. P7：补移动端响应式、键盘快捷键和 PWA 细节。

- **全部 11 个旧 `pc_*` 模块迁移完成，27 个旧资产文件已删除**
- `/pc` 现在直接指向新 Vite + React + TypeScript 前端（ServeDir from `$DATA_DIR/pc-next-dist/`）
- `/pc-next` 保留为向后兼容别名（同样指向新前端）
- `/pc-legacy` 指向发布脚本从 `d1f89950`（`e7b4a35b` 引入新框架前的父提交）导出的旧版静态对照快照
- 发布脚本会在旧版“打开移动端”旁注入“打开新版”，并把旧版 token 桥接到新版 `elon_auth`
- `server/src/web.rs`：所有 `PC_*` 常量和 `pc_app_*` 处理器已删除
- `server/src/router.rs`：所有旧 `/assets/pc_*` 路由已删除
- 前端 Vite base 已改为 `/pc/`，React Router basename 已改为 `/pc`

## 路由策略（最终状态）

| 路径 | 服务内容 |
|---|---|
| `/pc` | 新 React 前端（主路由） |
| `/pc-next` | 新 React 前端（向后兼容别名） |
| `/pc-legacy` | 旧原生 PC 静态快照（只读对照，不恢复旧源码） |
| `/web` | 移动网页版（不变） |
| `/` | 移动网页版（不变） |

## 路由策略（历史阶段状态）

| 阶段 | `/pc` | `/pc-next` | `/pc-legacy` |
|---|---|---|---|
| 准备期 | ✅ 已完成，历史状态为旧原生 PC | ✅ 已完成，历史状态为新前端验证入口 | 无 |
| 并行期 | ✅ 已结束 | ✅ 已结束 | 无 |
| 切换期 | ✅ 已完成，新前端成为主入口 | ✅ 保留同源别名 | ✅ 旧版快照可对照 |
| 收尾期 | ✅ 当前状态，新前端主入口 | ✅ 当前状态，兼容别名 | 🟡 只读对照快照，后续可按需要下线 |

## 旧模块收口状态

| 模块 | 原旧入口 | 当前状态 | 备注 |
|---|---|---|---|
| 应用壳、侧边栏、频道列表 | `pc_app.html`、`pc_app.js` | ✅ 已迁移到 `Shell`、`ConversationPage`、`ChannelNavList` | 频道固定、展开/折叠已在新前端维护 |
| 登录、注册、账号菜单 | `pc_app.js` | ✅ 已迁移到 `auth/` 和 `shell/` | 旧入口源码已删除 |
| 模型/运行路线选择 | `pc_app_models.js`、`pc_app_models.css` | 🟡 已迁移第一版到会话页运行路线控件 | 后续继续跟 Route A/B/C 产品文案对齐 |
| 项目中心、新建项目 | `pc_app_project_create.js`、`pc_app.js` | ✅ 已迁移到 `ProjectsPage`、`CreateProjectModal` | 项目中心默认广场；点击项目直接进入工作台项目主页，新建项目完成后同样直达主页 |
| 项目广场 | `pc_app.js` | ✅ 已迁移到 `ProjectPlazaView` | `/pc/plaza` 和项目中心广场复用同一视图 |
| AI 开发任务消息、审批、取消 | `pc_app_dev_tasks.js`、`pc_app_agent_runs.js`、`pc_app_task_snapshots.js` | 🟡 已迁移到 `features/dev/` 第一版 | 公开过程、恢复态、终态细节仍继续实测打磨 |
| 本机节点和客户端维护 | `pc_app_node.js`、`pc_app_node_admin.js`、`pc_app_client_maintenance.js` | ✅ 已迁移到 `features/node/`；已加入项目数据架构体检 | 旧/外部项目继承已跑通的目录与共享缓存；新项目优先推荐数据根；分析、容量和迁移均为建议，不得阻断任务 |
| 电脑医生 | `pc_app_doctor.js` | ✅ 已迁移到 `DoctorPage` | 后续按诊断能力扩展 |
| AI 声音/TTS | `pc_voice_project.js` | ✅ 已迁移到 `VoicePage` 第一版 | 与语音/TTS SDK 的边界继续在语音模块维护 |
| 通知 | `pc_app_notifications.js` | ✅ 已迁移到 `useNotifications` + `features/realtime/` | 新模块应继续复用统一实时刷新链路 |

## 多 AI 协作约定

- 开始 PC 前端任务前先 `git fetch origin main`，再读本文件和 `.github/instructions/pc-frontend-migration.instructions.md`。
- 一个任务只迁移一个明确模块，避免同时改新前端、旧多个模块和发布脚本。
- 如果必须修改旧 `pc_*.js/css/html`，先确认是否只是桥接或删除；新增复杂能力默认改到 `pc-frontend/`。
- 迁移模块后更新上方状态表，写清“旧入口是否删除、是否仍需 `/pc-legacy` 兜底”。
- 已有 AI 正在迁移某个模块时，其他 AI 不要同时编辑同一旧文件的大块逻辑；优先选择状态表中其他模块。

## 发布和验证原则

- 初期优先让新前端构建产物随 Rust binary 一起交付，保持现有后端发布模型简单稳定。
- 如果改为上传 `dist/` 到服务器，必须同步更新 Windows 和 Linux 发布脚本，并在文档中写明远端路径、缓存策略和回滚方式。
- 新前端任务至少验证 `npm run build`；涉及路由托管时还要验证 `/pc`、`/pc-next` 和静态资源路径。
- 用户可见的 `/pc` 新前端修复不能只以 `CodePushed` 收尾。页面、布局、交互、遮挡、登录/账号卡、聊天页或项目工作台改动 push 后，除非用户明确要求只同步代码，必须运行 `scripts\publish-server.ps1` / `scripts/publish-server.sh` 上传 `pc-next-dist`，再用统一收尾的 `PcFrontend` Kind 完成线上与本机状态校验。
- 遮挡、截图、按图修 UI 类问题的完成证据必须包含用户可见验收：浏览器截图、DOM 坐标检查或线上页面检查至少一种。构建通过只能证明代码可构建，不能证明用户看到的问题已消失。
- Windows 本地预览 `/pc` 时用 `scripts\start-pc-frontend-dev.ps1`。这个脚本固定解析 `npm.cmd`；不要用 `Start-Process -FilePath npm`，否则某些机器会先命中 `npm.ps1` 并按系统文件关联打开到记事本。
- 旧 PC 修复至少验证对应页面能加载；涉及服务端内嵌资源时运行最小 Rust 检查或相关测试。
