---
version_status: current
decision_status: proposed
implementation_status: pending
reviewed_at: 2026-10-02
owner: conversation-platform
---

# 书签与阅读位置技术设计

本文件定义[多书签需求](../requirements/message-reading-bookmarks-v1.md)的拟议接入合同。
接口名称、字段和默认参数是待实现设计，不能作为当前可调用 API。

## 当前能力与缺口

以本轮基线 `c3e5311116c150c06e5a4410313e9027b9665b8b` 为核对依据：

| 当前源码入口 | 已有能力 | 本功能需要补齐 |
| --- | --- | --- |
| `server/src/store/message_timeline/` | 四种来源权限和投影、latest/before/sync/window、排序索引 | around、after、双向边界、持久锚点解析 |
| `server/src/router/social_routes/message_timeline.rs` | v1 读取、窗口重验、已读入口 | 兼容 v2 路由、能力声明、私人书签/进度路由 |
| `server/src/assets/message_timeline.js` | Web 公共有界窗口 | 双向裁剪、目标窗口置换、请求代次隔离 |
| `pc-frontend/src/features/message-timeline/` | PC 公共窗口及锚点 | 侧栏、书签导航接线 |
| `server/src/assets/social_chat_recovery.js` | PWA 群聊/好友生命周期 | 书签界面、进度、向新历史分页 |
| `android/app/src/main/kotlin/com/elon/app/MessageTimelineWindow.kt` | 原生有界窗口 | 双向协议和定位 |
| `android/app/src/main/kotlin/com/elon/app/MessageTimelineNavigation.kt` | 重入绑定、刷新、历史按钮 | 显式打开意图、活动书签与进度保存 |

v1 `TimelineRequest` 只有 `before/sync/limit`，window 重验不等于 around。
APK 历史底部手势当前走 latest；书签模式必须改走 after，显式最新按钮独立。
已有[APK 重入修复](../reports/android-social-timeline-reentry-20261002.md)的普通打开最新、
空同步也绑定导航、串行请求与失败可重试都要保留。

## 责任划分

- TimelineSource：来源解析、鉴权、消息查询、排序、删除与撤回投影。
- ReadingPosition：书签/进度存储、冲突、离线队列、导航意图。
- 平台适配：可见锚点采样、列表定位、媒体与生命周期，不自行裁决同步冲突。
- AI 上下文编译：保持现有边界，不隐式读取全部书签附近消息。

建议服务端新增 `store/reading_positions/` 和薄路由，复用现有迁移入口；
Web 新增 `reading_positions.js`，PWA 与 PC 类型包装共同使用；Android 新增 `reading/` 包。
入口只做绑定，使用跨语言合同 fixture。不新增网络服务、消息表副本或强制重写渲染框架。

## 身份和数据记录

`ConversationRef = {kind, project_id, conversation_id, branch_id?}`。
自有 kind 为 friend/group/ai/channel；服务端规范化实际项目和会话，不持久化可能漂移的
“默认 AI 会话”别名。好友 scope 结合 owner 解析，好友 ID 本身不是全局会话身份。
AI 分支存在时明确绑定，禁止跳到另一个当前显示分支。

`Position = {message_id, created_at, block_id?, block_revision?, fraction?}`。
按 `(created_at, message_id)` 排序；服务端确认排序键，编辑不改变顺序，客户端不能
自报时间越过权限。块定位仅在源支持时启用，否则只承诺消息级。块修订改变时不复用
旧块偏移；像素位置连同设备/布局签名仅保存在本机，不跨设备使用。

| 记录 | 最小字段与约束 |
| --- | --- |
| Bookmark | id、owner、规范 scope、title、note、固定 Position、metadata_revision、created_at、deleted_at |
| BookmarkProgress | bookmark_id、current/furthest Position、progress_revision、server_updated_at |
| ConversationProgress | owner、scope、current/furthest Position、progress_revision、server_updated_at |
| ProgressCandidate | 对象、device_id、device_seq、session_id、base_revision、Position、server_received_at |
| LocalOutbox | account、operation_id、对象、base_revision、device_seq、操作体、状态 |

metadata_revision 和 progress_revision 分开，改名不引发阅读冲突。
owner 从认证派生；创建、列表、预览、定位和进度写入每次检查本人及来源权限。
scope/固定起点不支持 PATCH；另存新书签可以共用原点。operation_id 幂等，同键异体拒绝。
数据索引覆盖 owner/scope/更新时间/ID、owner/bookmark_id、owner/对象/device/seq。
消息源复用 scope/时间/ID 索引；追加迁移，不回填聊天正文、不为每条消息建进度。

## 双向时间线 v2

拟新增 `/api/me/message-timeline/v2`，与 v1 共用 store/query，保留旧路由。
能力响应声明 `timeline_around`、`timeline_after`、`reading_bookmarks` 及限制；新端核对
能力和 schema，老服务器不支持时保留原聊天、明确书签暂不可用。

请求模式 `latest | around | before | after | sync` 互斥。around 使用消息 ID 或本人
书签/进度目标；删除后的回退排序键从服务端保存对象取得，不信任客户端任意位置。
建议响应 schema `elon.message_timeline.v2`：

```text
messages[]                         升序，目标若可见则恰好一次
before_cursor / after_cursor       返回窗口首尾边界
has_older / has_newer               边界外是否存在可访问历史
sync_cursor / has_more_changes      变更水位与待续取状态
target {requested_id,resolved_id,status,position}
reset_required / reset_reason      恢复原因，不代表跳到最新
```

around 默认目标前 24 条、目标、后 25 条，共最多 50；一侧不足可从另一侧补足。
前后查询严格排除边界，用 limit+1 判断是否还有；after 取紧邻的最早一段更新历史，
不能误取最新 50 条。响应共最多 100 条，不能前后各返回 100。鉴权、目标、两侧查询和
变更水位来自同一读快照。游标绑定账号、scope、方向、epoch 和边界，不与 sync 混用。

裁剪窗口后用实际保留的首尾消息续页。v2 每条消息提供可作前/后边界的服务端不透明
定位令牌，或等价可验证边界结构；实现需选定一种并通过共同 fixture。
离开最新区时新消息只更新提示和窗口内的编辑/撤回，不移动窗口；has_newer=false
不自动开启跟随最新。增量过期时有界重验或 around 当前持久锚点，不从最新开始。
长期书签不依赖短期游标，游标失效不等于书签失效。

## 书签和进度接口

以下路径均为拟议名称，落地时统一确认：

| 操作 | 接口 | 行为 |
| --- | --- | --- |
| 列表 / 创建 | `GET / POST /api/me/reading-bookmarks` | 本人、scope、游标分页；创建返回固定起点及版本 |
| 改名备注 / 删除 | `PATCH / DELETE /api/me/reading-bookmarks/{id}` | metadata 条件更新，删除幂等 |
| 读写进度 | `GET / PUT /api/me/reading-bookmarks/{id}/progress` | base_revision、operation_id、device_seq，返回确认版本 |
| 普通进度 | `GET / PUT /api/me/reading-progress` | 规范 scope，同一冲突合同 |
| 处理冲突 | `POST /api/me/reading-bookmarks/{id}/progress/resolve` | 选候选并比较版本；普通进度提供等价操作 |
| 跨端变化 | `GET /api/me/reading-changes` | 有界增量及 tombstone，过期后分页重验元数据 |

错误区分未登录、不可用、无会话权、版本冲突、参数无效、能力缺失、超额及临时失败。
越权书签与不存在 ID 对外同样不可用；日志不含正文/备注。401/撤权停止重试并清理
可见内容；409 处理冲突，429/503 退避并尊重 Retry-After。定位网络失败保留原画面。

## 持久化、冲突和删除

原子落盘位置与 outbox 后才显示已保存本机；确认只移除对应 operation_id，不清掉
请求在途期间的新位置。同对象同设备合并尚未发出的中间位置，保留最新 current
和有效最大的 furthest；已经发出的 payload 不变，新变化生成新 operation_id。

服务端事务校验 owner/权限/存活、幂等、base_revision，再推进版本。
同设备 device_seq 防旧回写，跨设备不按手机时间或消息远近决胜。
base_revision 冲突时可靠保存候选并返回冲突；双方列表可见，画面不跳转。
用户选择仍要比较最新 revision，另一候选保留可恢复。每对象每设备合并最新未解决
候选，建议最多 8 台；满额明确拒绝并保留本地队列，不静默丢位置。
已解决候选建议最多 10 个、保留 30 天；未解决项不按已解决清理规则丢弃。

删除与进度写串行化；删除/不存在 ID 的进度 PUT 永不 upsert 书签。
离线创建后删除且未上传可本地取消；书签 ID 永不复用。服务端删除正文相关元数据后
保留仅 owner/ID 的删除身份记录至账号删除，拒绝重放创建同一 ID；增量 tombstone
可按 changes 有效期清理，过期客户端须先分页重验。创建去重另绑定 operation_id，
响应丢失时查询原 ID，不以新 ID 盲目重试。旧创建请求不能复活删除对象。
撤销删除以新 ID 重建同样元数据与位置，幂等确认。

队列有界并合并进度，超额提示同步/清理，不静默丢弃。普通消息缓存淘汰不删除
未同步位置；退出清空内存，磁盘按账号隔离，同账号再次认证后才能回放。
明确清本机数据时告知尚有未同步位置；断网时无法即时得知远端撤权，联网后重验。

## 导航状态与竞态

```text
阅读目的：ordinary | bookmark(bookmark_id)
导航阶段：idle | locating(target,generation) | active | error
窗口模式：live | history
临时查看：return_position?（单个有界返回点）
同步状态：local_pending | syncing | synced | conflict | failed
```

A → B：采样并落盘 A → B 新 generation → around → 成功置换窗口定位 → 激活 B。
失败保留 A，A 迟到响应不更新 B 的列表/按钮/进度。回最新先保存原位置，成功后解除
活动书签，失败保留原模式。搜索/引用先保留返回点并暂停采样，返回后恢复；只有
“从这里续读”才改变当前进度，不维护无限嵌套窗口。

请求绑定 account_generation、scope、navigation_generation、direction，切号/会话/书签
时取消且在应用响应前比较代次；不能仅靠 abort。每活动窗口串行请求、合并相同触边，
显式新目标可以取代旧意图。普通打开 latest、书签 around 使用明确 OpenIntent，
不能让通用 onResume 无条件 latest。程序定位、恢复、图片布局和窗口裁剪期间屏蔽采样。

## 已读、AI 与资源

current/furthest 不作为未读清零或连续已读上界。书签历史浏览不走旧最大值回执，
可见区间已读需另行定义。发送/引用沿用既有协议，书签不发群通知、不扩大 AI 输入。
自有 AI 在服务端持久 ID 可用后允许创建；外部 Web AI 声明能力后才接入，
不能抓取全历史或把厂商临时缓存 ID 当成跨端稳定身份。

消息数量、文本字节、正文展开、媒体缓存分别限额。当前窗口至少保留一条的策略可能
让单条超长消息突破文本预算，须补正文分块/折叠和按需渲染，不能宣称总内存已封顶。
离线只承诺已缓存目标；没缓存时提示联网，不显示其他位置伪装命中。
自动预览由当前有权内容派生，删除/撤回/撤权清除；手写备注与自动预览分开。

## 发布与工程验收

先扩服务端并声明能力，再启客户端入口；v1 兼容。回滚关闭新入口、保留新表与数据，
不退迁移丢书签。按来源/客户端能力控制入口，不把未接入页面写为全来源完成。
需求 B01–B12 之外，合同测试覆盖两侧不足、边界目标、并发删除、游标错用、
进度与删除竞争、重放、冲突候选满额、changes 过期、登出晚响应及失败保留模式。
三端同样 fixture 比较活动书签、目标 ID、首尾边界和待同步状态。

用 1千/1万/10万条固定数据在同设备同网络比较：around 查询次数与深度无关，
索引 seek + limit，不按目标深度 OFFSET 或追页。正常一次跳转只取一次窗口；
媒体、元数据和重试另外计数。记录 P50/P95 查询/首个目标渲染、行数、字节及掉帧。
固定环境十万条相对一千条 around 查询 P95 目标不超过 2 倍，查询计划无全表扫描；
这是待测目标，不推导所有设备固定毫秒数。顺序滚动工作集不得随已读总量增长。

首批记录小米 APK 真机、PWA 浏览器、Win 实际页面的多书签往返，及
书签 → 最新 → 退出 → 重入 → 继续阅读完整链路。已有分页单测不能替代入口验收。
本次仅形成设计，无新增运行或性能结果；实施证据另写报告并绑定提交与版本。
