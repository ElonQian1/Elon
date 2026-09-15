---
version_status: current
reviewed_at: 2026-09-15
implementation_status: verified
---

# Windows 社交聊天首轮整改验收

需求与后续路线见 [社交聊天整改方案](../requirements/social-chat-desktop-remediation.md)。本轮改动为 Windows 工作台与 PC 网页共用的 React 前端。

## 已实现

- 消息右键、更多按钮与 Shift+F10 统一进入社交菜单；方向键选择、Esc 退出和焦点返回；移除真人聊天中的 AI 评价操作。
- 引用回复保留原有草稿，显示作者和引用版本，支持取消；发送为三端兼容的 Markdown 快照。
- 群聊本人文字编辑、已编辑标记、完整历史与并发冲突提示；他人、待发、撤回消息和专用卡片不能进入普通编辑器。
- 本人一分钟内撤回，服务器拒绝时保留原文并显示原因；本机隐藏与撤回分离，隐藏可恢复。
- 多选最多 20 条，转发前选择目标并预览确认；部分成功只续发剩余项，网络结果不明不自动重发，账号退出中止后续批次。
- 本机收藏快照、取消收藏、账号与服务地址隔离、退出清理；存储失败不伪装成功。
- 附件选择、剪贴板图片、拖入文件、音频文件上传，失败重试和移除；每条最多六个、单个不超过 12 MiB；附件跟随原会话，不能串入新会话。
- 群成员目录与 @、已加载消息查找、服务器群历史检索、暗色置顶总结阅读。
- 保留缓存、断网恢复、前后台同步、草稿与编辑审计；阅读旧消息时出现新消息提示，用户主动回到最新；输入法回车选字不触发发送。
- 保留并行交付的文章列表、卡片和专用文章功能。修正好友手机号检索参数。

## 验证证据

| 验证 | 结果与范围 |
|---|---|
| `scripts/test-pc-social-chat-parity.cjs` | 真实 React 页面，菜单权限/焦点/边界、编辑历史、引用草稿、收藏隐藏、多选转发部分失败、撤回拒绝与成功、查找、暗色总结、关闭后迟到的成员响应、附件选择/粘贴/拖入/切换、输入法、退出取消批次 |
| `scripts/test-pc-social-chat-recovery.cjs` | 缓存刷新、列表部分失败、超时、推送/前台/联网恢复、修订、未读、旧响应隔离、发送失败与回执竞速、权限拒绝、账号切换、缓存损坏/配额 |
| `scripts/test-social-chat-operations.cjs` | 撤回窗口、编码路径、Unicode 引用、认证与附件载荷、拒绝、写超时不自动重试 |
| `scripts/test-group-message-revisions.cjs` / `scripts/test-group-message-revisions-ui.cjs` | PC/PWA 修订合并、完整历史、并发冲突、失败草稿、文本安全与权限 |
| `pc-frontend/scripts/test-social-media-downloads.cjs` | 媒体显示、头像、撤回隐藏附件、云地址及下载链接 |
| `pc-frontend/scripts/test-project-apk-member-download.mjs` | 会员下载回归 |
| `scripts/test-article-ui.cjs` | PC/PWA 文章草稿、预览、发布重试、卡片与阅读器兼容 |
| TypeScript / ESLint / Vite | 类型检查、变更文件零告警、生产构建通过 |

测试账号、消息、文件和 API 均为隔离 fixture，不向真实群友发送测试消息。浏览器截图已检查宽窄窗口、弹窗居中和暗色文本；这不代表已在用户的 Windows 安装包或物理手机上验收。

## 尚需独立交付

私聊编辑审计、结构化引用与跳转原文、全历史游标分页、云收藏/隐藏同步、具备服务端幂等键的离线发件箱、Windows 原生录音及通话。本轮不把这些能力显示为已完成；音频文件发送不等同于麦克风录音。

## 发布与恢复

使用 `scripts/publish-pc-frontend.ps1 -ReuseLiveServer`，依赖现有聊天 API 与已发布文章 API。最终部署身份和本机工作区状态由正式发布回执及 `finish-ai-task.ps1 -Kind PcFrontend` 核对，不以本地构建代替上线成功。最终 registry 提交后必须再次发布相同前端工件对应的新 HEAD，再执行收尾。
