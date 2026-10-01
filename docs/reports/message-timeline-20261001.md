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
| 四类来源的分页、权限、增量和窗口重校验 | implemented | Rust 专项 11 项通过 | 待发布；真实生产历史库待验收 |
| Win/PWA 好友、群聊及其中的 AI 消息 | implemented | Node、两套浏览器替身、最终 PC 构建与定向 lint 通过 | 待发布；真实长时体验待验收 |
| APK 好友、群聊及其中的 AI 消息 | implemented | 原生专项 10 项通过 | 待发布/装机；真实滚动性能待验收 |
| 社交 AI 历史输入字符预算 | implemented | Rust Unicode 与最近消息保留测试通过 | 待发布 |
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
  删除、游标换代及权限用例共 11 项通过；最终验证指纹
  `29336b8dd1db3ec65ec51a9610c9f1e70b60c1b66b357adc92c6b82470d40c57`。

上述为定向逻辑、数据库和本机浏览器证据，不是生产负载、帧率、用户设备或一周长时实测。
本机 Rust 验证使用平台管理的本地共享缓存入口，通过容量门禁；未绕过空间保护。
窗口恢复增补后的首轮 Rust 验证因容量门禁中止，随后本地编译缓存和共享盘最终产物组合验证通过。
缓存 GC 重新检测到其他活跃构建后保留该分区，本次未删除其他任务缓存或文件。

## 后续入口

[多书签方案](../requirements/message-reading-bookmarks-v1.md) 保留固定标记与独立移动进度，
长期锚点使用消息身份；短期同步游标与数据库 OFFSET 都不能充当持久化书签。
个人 AI/项目 UI 迁移仍须保护流式本地消息、节点执行输出、任务恢复卡和来源字段。
外部厂商 Web AI 继续沿用已有供应商会话语义与有界窗口，不能假定通用 API 可替换其私有合同。
