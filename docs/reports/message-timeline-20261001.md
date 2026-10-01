---
version_status: current
reviewed_at: 2026-10-01
owner: conversation-platform
---

# 消息时间线首批交付证据

需求：[message-timeline-v1](../requirements/message-timeline-v1.md)。
本记录区分基础模块、已接入页面和待迁移页面，不表示全部聊天入口已迁移。

## 实现边界

| 能力 | 实现 | 验证 | 交付/验收 |
| --- | --- | --- | --- |
| 四类来源的分页、权限、增量和窗口重校验 | implemented | Rust 专项 11 项通过 | 后端 0.3.1810 已发布；真实账号历史阅读待验收 |
| Win/PWA 好友、群聊及其中的 AI 消息 | implemented | Node、两套浏览器替身、最终 PC 构建与定向 lint 通过 | 已发布并核对线上源码；真实长时体验待验收 |
| APK 好友、群聊及其中的 AI 消息 | implemented | 原生专项 10 项通过 | 1.1.1846 已发布；两台登记手机离线，装机延期 |
| 社交 AI 历史输入字符预算 | implemented | Rust Unicode 与最近消息保留测试通过 | 随后端 0.3.1810 发布 |
| 个人 AI、项目频道页面迁移 | partial | 后端投影等价和权限通过 | 页面仍使用既有有界读取入口 |
| 多书签、独立阅读进度、around/双向定位 | proposed | 未实现 | 见独立方案；不得宣称上线 |

## 模块与行为

- 存储：`server/src/store/message_timeline/`。默认 50、最大 100 条；
  使用 `(created_at,id)` 索引定位，首次快照与增量水位同事务。
- body-free 变更日志覆盖新增、编辑、撤回、删除、引用依赖和部分任务/AI 元信息。
  保存最多约一百万条事件；积压超过一千条时改用有界恢复，避免逐条重放整个离线周期。
- `GET /api/me/message-timeline` 只读；已读通过独立 POST，读取历史不自动标记全部已读。
  window POST 只重校验当前最多 300 个消息 ID、更新内容/删除状态和游标，仍逐次验证权限。
- 共享 Web 状态机：`server/src/assets/message_timeline.js`；PC adapter 位于
  `pc-frontend/src/features/message-timeline/`；原生 adapter 为 `MessageTimelineWindow.kt`。
- PC 窗口 300 条、PWA/APK 150 条，另有字节预算；单条超大消息保留原文，预算不是硬上限。
  未完成发送独立保存，不能为了清理窗口丢失待确认消息。
- Web 使用有界 DOM 与 content-visibility；不把它表述为完整虚拟列表或实测帧率提升。
  APK 保留 RecyclerView。历史上翻保存锚点；新消息不替换当前历史窗口。
- 隐藏页面暂停轮询，恢复可见/网络/推送时增量刷新；相同内容不重复写缓存或重绘。
- 社交 AI 历史文本预算为 32,000 Unicode 字符，保留最近完整消息，超大单条有显式截断标记。
  这是历史区块的字符预算，不是精确 token、总请求预算或自动摘要/检索系统。
- 旧消息 API 保留兼容。后端发布必须先于使用新接口的客户端。

## 可重复验证

- `node --test scripts/test-message-timeline.cjs scripts/test-mobile-social-recovery.cjs`：16 通过。
- `scripts/test-message-timeline-browser.cjs`：Edge 无头浏览器通过十万条源、五次上翻、
  DOM 上限、可见锚点、历史期新消息/已读隔离及日志过期后当前窗口恢复。
- `scripts/test-pc-social-chat-recovery.cjs`：完整 React 浏览器替身通过缓存/草稿/推送、
  超时、隐藏页面、账户/会话切换、403 清理、编辑及发送确认竞争等回归。
- `npm run build`：窗口恢复增补后的最终构建通过；定向 ESLint 无告警。
- APK `MessageTimelineWindowTest`、`SocialChatReadChannelTest`、`SocialChatRecoveryTest`：
  含日志过期历史恢复共 10 项通过。
- `validate-rust.ps1 ... test --bin elon-server message_timeline`：含窗口恢复、当前正文、
  删除、游标换代及权限用例共 11 项通过；合并上游后的验证指纹
  `608e871c21bb401dccc9137df6d1cf5748e0bfd8ea1cce4c41776ca0530652af`。
  上游成员管理与本模块均保留，本模块迁移编号为 310。

上述为定向逻辑、数据库和本机浏览器证据，不是生产负载、帧率、用户设备或一周长时实测。
本机 Rust 验证使用平台管理的本地共享缓存入口，通过容量门禁；未绕过空间保护。
窗口恢复增补后的首轮 Rust 验证因容量门禁中止，随后本地编译缓存和共享盘最终产物组合验证通过。
首次缓存 GC 因其他活跃构建保留分区；后续确认无活跃写者并审阅计划后，
平台 GC 回收闲置共享验证缓存约 2.71 GiB，回执为 `gc-20261001-114151.json`。
未删除源码、工作区或未知目录；跨目标分区的后续 GC 仅预演，未执行。

## 发布与故障恢复

- 功能提交 `64f373ad43c4c6d783ee2012ed86243bdbe5b7da` 已入主线。
  后端及 PC/PWA 发布源码为 `994e32ff36c6059e1e6c819bce5c958fc107904f`。
  同一主线的成员管理发布流程完成该工件，包含本功能；本任务复核后复用，未重复部署。
- `check-task-complete.ps1 -Kind Server` 和 `-Kind PcFrontend` 均通过：
  后端 `0.3.1810`、健康 `OK`、PC 路径 HTTP 200、前端与后端发布身份匹配。
  线上 `message_timeline.js` SHA-256 与源码一致；携带合法参数的匿名时间线读取返回 401。
  这些只读检查不等于已验收真实账号下的全部消息操作和历史恢复。
- 正式 APK `1.1.1846`（build 1846），发布源码
  `4fecbc2b58c1ea41a6ddcf33184cd69f6695acd5`，相对上述源码仅新增非 Android 改动。
  SHA-256 为 `43fe72c7aab5265179515d069682fea63b339108ed3ef0118b3a62c5215bbed0`。
  官方恢复入口返回 `APK_RELEASE_STATUS=already_covered`，AndroidFeature 完成检查通过。
  复用同工件发布后的主项目设备回执（2026-10-01 20:16:25，UTC+8）：
  小米 23116PN5BC、荣耀 AAK-AN00 均 `offline`，无在线安装失败；未宣称手机已更新。
- 首次后端发布因 UNC 对象文件路径被 MSVC linker 解析为选项而失败。
  使用平台容量预留、锁和本地中间产物后越过此失败点；未降低磁盘容量门槛。
- 随后的本任务构建在 `serial2`、`bytemuck` 编译时以 `0xc0000043` 退出，
  与用户提供的 `rustc.exe` 启动错误弹窗一致。该状态为 Windows 文件共享访问冲突；
  当时的具体文件及占用者未捕获。事后工具链文件可读，其他构建成功完成并发布。
  结论是发布已恢复，不能宣称根因已定位或永久修复，也未将其归咎于杀软或源码。
- 本任务 APK Release 编译成功，后续校验进程一度无法识别 `Get-FileHash`。
  新进程对现有 APK 的 177 份 Web AI 资源校验通过；PowerShell 7 下通过官方
  `publish-apk.ps1 -SkipBuild` 恢复时，已有上述主线正式包，脚本按输入等价复用。
  本任务最初生成的 APK 未上传，不将其不同摘要冒充正式包。

本批发布证据不代表统一时间线需求全部完成：个人 AI/项目 UI 迁移仍为 partial，
多书签仍为 proposed；功能登记释放本轮认领后保留这些后续入口。

## 后续入口

[多书签方案](../requirements/message-reading-bookmarks-v1.md) 保留固定标记与独立移动进度，
长期锚点使用消息身份；短期同步游标与数据库 OFFSET 都不能充当持久化书签。
个人 AI/项目 UI 迁移仍须保护流式本地消息、节点执行输出、任务恢复卡和来源字段。
外部厂商 Web AI 继续沿用已有供应商会话语义与有界窗口，不能假定通用 API 可替换其私有合同。
