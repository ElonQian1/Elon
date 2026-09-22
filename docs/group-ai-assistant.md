# 群 AI 助手

## 产品边界

用户批准的名称为“群 AI 助手”，内容分为“关注事项”和“更新动态”。它分享群友授权的 AI 任务成果，不监听群聊，也不默认共享个人 ChatGPT 历史。

- 群友主动选择自己的关注事项，预览后绑定到指定群；不按群名称绑定。
- 群成员查看最新已同步结果、来源、分享人和同步时间；私密续聊仍回到自己的 AI 会话。
- ChatGPT 在云端执行任务不等于一龙已经同步。分享者客户端在线且身份有效时才能读取个人任务结果；离线时群里保留已同步版本，不虚报实时更新。
- 不上传 Cookie、运行时令牌或整个个人会话。任务邀请链接用于复制任务，普通会话分享链接也不是持续更新结果的订阅接口。
- 未授权、离群、取消分享、任务已删除和暂时读不到必须分别处理。不得自动重建删除的任务。

官方产品语义参考：[Tasks](https://help.openai.com/en/articles/10291617)、[Shared Links](https://help.openai.com/en/articles/7925741-chatgpt-shared-links-faq)。不硬编码账号任务配额。

## 当前进度

2026-09-23，`group_ai_assistant_shared_tasks_v1` 已补齐并发布 APK 选择、确认分享、后台同步，以及服务端权限和 APK/Win/PWA 阅读流程。正式 APK 1807 的目录、非空结果预览、完整阅读及返回已真机通过；真实授权分享验收仍待用户指定事项，不能把测试夹具当成私人任务分享成功。并行交付的 [群 ChatGPT 账号入口](reports/group-chatgpt-account-entry-20260923.md) 和上一批 [群回复恢复验收](reports/group-ai-send-recovery-20260922.md) 均保留。

| 能力 | 代码状态 | 验证状态 | 缺口 |
|---|---|---|---|
| 手机官网任务入口及请求观察 | implemented | device_verified | 已确认页面可访问且读取返回 200；不是新传输验收 |
| 分页任务列表只读传输 | implemented | device_verified | 完整首屏 2 项；实际多页仅离线覆盖 |
| 最新结果只读传输 | implemented | device_verified | 真机确认 `no_update` 及 3374 字非空正文；分享到群仍待明确选择 |
| 账号隔离、有界缓存、失败保留旧结果 | implemented | offline_verified | 真机已确认缓存命中，账号切换/失败分支由离线测试覆盖 |
| APK 用户挑选、预览及明确授权 | implemented | device_verified | 目录、预览和授权确认界面通过；实际分享仍等用户指定事项 |
| 群绑定、成员权限及取消分享 | implemented | offline_verified | 8 项生产 SQL 回归，含退群/重新入群不恢复旧授权 |
| 结果去重同步和群动态阅读 | implemented | offline_verified | APK/Win/PWA 入口已接；真实非空更新待分享者选择样本 |
| Win/PWA 自己导入 ChatGPT 任务 | not_in_this_delivery | deferred | 当前由 APK 登录身份层采集；Win/PWA 可阅读和撤销自己的分享 |
| 交互图表及私有附件原件 | unsupported | deferred | 当前分享官方结果正文 Markdown；不导出 raw content、私有 URL 或令牌 |

## 已核实的协议

证据来自手机当前页面加载的官方公开静态资源和本机 MCP 结构化请求观察，不是根据接口名称猜测。只实现下列 GET；不创建、运行、暂停或删除任务。

| 请求 | 契约 |
|---|---|
| `/backend-api/automations?filter=scheduled` | 分页对象 `items`、`cursor`；filter 也支持 `paused`、`finished` |
| `/backend-api/automation/{id}/latest_backing_run?include_snapshot=true` | 官方消费者兼容直接结果、`latest_update` 包装及 `null` |

`null` 是没有更新，不是网络错误。`last_backing_run` 是最近一次运行；`latest_update` 可能来自更早的一次有意义更新，不能用最后运行时间作为群消息去重键。

公开资源证据（SHA-256）：

- `conversation-small-f8ygwl0g2zcxhtqg.js`：`fcf2d589e5fad9b5d077561b50fac4fbd0992cda8141417baf3cf9ec272edf59`，请求定义。
- `5d2279e3-k1l8f23nneajbzig.js`：`98e8345b112429abe737da91cffc8dc2ad729102b90c3367dfbfd92e6d0c4210`，分页与分类消费者。
- `594e15ee-of70nfheles0fhzj.js`：`54872fdccbca42b5ca85359674ee406de804f241171ea374c3546948bb20276a`，最新结果、失败和待用户处理消费者。

原始私人响应和运行时请求头不进入 Git。公开脚本临时样本位于忽略目录，不作为协议稳定性承诺。

## 代码与验收入口

- `chatgpt_web_private_tasks_policy.js`：严格解析列表与结果，目录不携带任务 prompt；结果富文本仅保存在本机会话内存。
- `chatgpt_web_private_tasks.js`：同源身份层，7 秒超时、1 MiB 响应上限、60 秒缓存、最多 12 项缓存及 4 个并行读取；单飞、限流、身份切换后旧响应丢弃。
- `chatgpt_web_private_tasks_probe.js`：现有 MCP `chatgpt_private_protocol_probe` 的 `scheduled_tasks` 模式，只回传数量、缓存命中和结果状态，不回传任务标题、ID 或正文。
- `scripts/test-chatgpt-web-private-tasks.js`：协议、账号切换、超时/限流、单飞、缓存和探测隐私定向测试。
- `ChatGptWebScheduledTasksEvidenceTest.kt`：Android 接收端拒绝夹带私人字段的探测回执。

本模块不等待输入框或 DOM 控件就绪，不修改已有聊天发送、语音和群 AI 回复路径。只有用户确认的绑定才会同步；未选择事项时不会读取或上传个人结果。

### 生产接入

- `chatgpt_web_scheduled_tasks.js`、`ChatGptScheduledTasks.kt`：独立生产命令，私有内容不进入 MCP 命令回执或日志。账号范围使用用户 ID 与工作区 ID 的 SHA-256，不以易过期的 access token 作为长期绑定键；缓存仍受运行时身份及文档约束。
- `GroupAssistantFeature/Ui/Sync/Api`：原生群设置入口、分页挑选、结果预览、明确同意、撤销与原生阅读。进入页面先读共享结果，再异步同步自己的绑定，不阻塞浏览；前台每 5 分钟最多同步 10 项，按最久未检查顺序轮转，进入后台取消。
- `store/articles/group_assistant`、migration 305：稳定群 ID、所有者、账号范围及任务 ID 绑定；每次读写校验群成员，其他群友不获取私有任务 ID/账号范围。撤销或分享者离群会移除结果并拒绝迟到上传，重新入群不恢复旧授权。
- 更新以 provider result ID 幂等导入，保留最近 100 条正文；清理旧正文后仅保留去重凭据，旧结果重放不会复活。网络失败不清空旧结果，已同步时间与检查时间分离。
- `pc-frontend/.../group-assistant` 与 `server/src/assets/group_assistant.*`：同一授权 API 的目录、分页动态、Markdown 详情与返回。未复制 ChatGPT 凭证，也不把分享链接当实时订阅。
- 分享范围仅为标题及官方最新结果的正文，保留 Markdown 表格、代码和公开引用链接。交互图表及附件原件尚不导入；不能描述为原始多模态内容完整迁移。确认页明确此范围。

新增定向验证：生产命令 7 项、既有任务传输 14 项，Android `ChatGptScheduledTasksTest` 3 项，生产 SQL 8 项；PC 完整 build 通过。PWA Playwright 在 320/390/1280 像素宽度验证分页、Markdown、撤销、账号变化丢弃响应和返回保留草稿，未访问外部站点。真机非空结果分享不以这些夹具代替。

### 本轮验证与安装包一致性

- 相关 JavaScript 回归 127 项通过，其中本模块 14 项；Android 定向单元测试及编译通过。
- 线上 `1.1.1804` 曾声明源提交 `d59abdde6`，但下载包没有新任务脚本，手机仍为 adapter 432 并拒绝新探测模式。因此不能把该版本记作本功能已上线或已验收。
- 并行发布任务确认该旧包来自快进代码后的 `SkipBuild` 复用；包内容仍为此前提交。当前批次改为单一完整构建，不再复用该包。
- Windows 发布入口增加独立 `apk-publish-artifact-validation.ps1`：上传前核对 APK 内所有 `chatgpt_web*.js` 与当前源码的 SHA-256；同时保留原版本校验。缺少脚本、旧内容和重复条目均失败关闭，定向 7 项通过。
- 本机构建产物的 163 个脚本核对通过；该线上旧包被新门禁正确拒绝，不能只看版本清单的提交号。
- 已用官方发布流程生成的本机 Release 候选覆盖安装验收：adapter 433、身份与 bridge 就绪，`catalogCount=2`、`catalogComplete=true`、`cacheHit=true`、`latestState=no_update`。只读取，没有修改任务或发群消息。
- 该本机候选与线上旧包同为 1804，不能冒充同一制品。后续必须发布更高版本并校验摘要；非空最新结果及完整群分享仍未验收。

正式交付已更新为 `1.1.1805`（code 1805），源码 `8b42f9ce0555a323af3d75843e10f09a3bcd7854`，APK SHA-256 `2481112774fdfb14e058f127aceef812865a518791ab81dc6266572ccf9b149b`。完整 Release 构建和 163 个脚本校验通过，线上清单回读一致；小米覆盖安装及版本回读通过，荣耀离线。正式包再次确认 adapter 433、认证及 bridge 就绪，MCP `scheduled_tasks` 回执 `ok=true`，2 项完整目录、缓存命中、`no_update` 均与候选一致。没有创建、修改、运行或分享个人任务。

## 剩余验收

### 2026-09-23 生产接入发布

- Server `0.3.1775` 已完整构建发布，源码 `e264ea10372d79f9f299ab5f2c313300c57fac74`；线上 `/api/server/version` 回读一致。migration 305 与群项目绑定 migration 306 按顺序一并部署。
- PC 前端完整 build、bundle budget 门禁和静态资源发布通过；`/pc/` 返回 200。PWA 群助手 JS/CSS 线上 SHA-256 与本地源码一致，未登录访问个人群助手目录返回 401。
- APK `1.1.1806`（code 1806）已发布，源码 `79ce4cac17ef45fffb6d8c141993396737835668`，线上清单 SHA-256 `e3accabcb7626642ccd801187620febac9d678a6a4d782f2b5e1471ffc9c9e5c`。此提交与服务端发布提交之间没有 Android 源码差异；小米无线 ADB 已独立回读安装版本 1806。
- 合并时保留 adapter 435 的两个独立入口：群项目绑定与关注事项读取。合并后任务 JS 19 项、PWA 三视口回归再次通过；PC 账号切换后丢弃旧响应的补丁通过完整 build。
- 上述是代码、制品与安装证据，不代替真实私人结果共享验收。尚未由开发者代选私人任务分享到群。

### 目录到预览的账号作用域回归

- 1806 正式包可读到 2 项目录，但预览失败；同源只读诊断确认固定错误码 `tasks_account_changed`，不是官网任务缺失或 VPN 失效。
- 冷目录在账号请求头尚未水合时，把 `personal` 占位串写入作用域；预览时请求头已出现真实账号 ID，两次 SHA-256 不一致，导致误判换号。真机仅输出布尔比较与错误码，未导出身份值或私人结果。
- 生产桥 v2 改从每次 `/api/auth/session` 返回的 `user.id + account.id` 生成稳定作用域，并校验已捕获账号头与 Cookie 账号一致。无身份时拒绝请求，不再使用 `personal` 占位或仅凭旧 Bearer 缓存身份。
- 新增冷目录到热预览、Cookie 换号/退出且捕获凭据未变的回归，总计 21 项任务 JS 测试通过。UI 将账号变化与登录失效分别提示，不输出异常原文。
- 同签名本机研究候选（版本仍为 1806，不等于线上正式制品）热加载生产桥 v2 后，目录和预览均 `tasks_ready`，确认页 `consent_visible=true`；取消后回到 2 项目录且群绑定仍为 0。没有点击分享。正式版必须重新完整构建并恢复关闭研究开关，不能发布此诊断 APK。
- 随后只读预览第二项，收到 `tasks_ready / hasUpdate=true / characters=3374`，确认页和原生完整阅读页均可打开。阅读返回后目录保持 2 项、群绑定仍为 0；退出回到原群聊。未导出标题、任务 ID 或正文，没有运行或分享任务。

### 1807 正式包验收

- 完整构建发布 APK `1.1.1807`（code 1807），源码 `90bda5f5dd77ed48280685b00db33b9d9d57a469`，SHA-256 `4d97b3c707c32b8c33bf6bd77f47f51ab48bc9a4087f59e660e40b936f645b4a`。线上版本清单与小米已安装版本独立回读一致；研究开关关闭，不复用研究候选。
- 未热加载脚本，直接通过原生群设置进入助手：目录 2 项、第二项预览成功、原生完整阅读页可见，返回后目录仍为 2 项、绑定为 0；退出回到原群。全程无失败提示，没有运行或分享私人任务。
- 合并后 Android 3 项 `ChatGptScheduledTasksTest` 再次通过。正式包同时包含并行群项目身份及未发送恢复修复；该功能的发送验收由其独立报告记录，不从群助手只读验收推导。

### 待验证边界

1. 复用已验证的任务列表、缓存、无更新和非空正文读取，不重复研究，不自动运行私人任务造样本。
2. 由用户明确选择一个事项，再验证授权绑定、真实结果同步、群内阅读及撤销，不擅自公开私人任务。正式包只读路径已验收，不再重复构建验证。
3. 未来 Win 本地 WebView2 采集与交互图表/私有附件需要各自独立契约和验收，不能因为本轮文本结果分享通路上线而标记完成。

对应领域规则仍见 [私有集成手册](web-ai-private-integration-playbook.md)。在端到端授权、权限和重复同步验收前，不发布“群 AI 助手已可用”的说明。
