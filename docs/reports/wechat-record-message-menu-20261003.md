---
version_status: current
reviewed_at: 2026-10-03
---

# 聊天记录消息菜单

## 根因

- Win/PC 的 `special` 同时控制专用展示和整个操作分支，记录卡片仅保留 AI 分析、多选，引用、收藏、撤回等被连带排除。
- Android 阅读书签用新 PopupMenu 截获长按，旧菜单变为“其他消息操作”的二级入口，并非记录无法绑定长按。
- PWA 每次消息刷新无条件关闭菜单，即使原消息未变化；记录的菜单摘要与引用预览还直接显示内部 JSON。

## 能力组合

- 通用操作与专用展示分离，不用 `special` 一刀切屏蔽消息菜单。
- Win/PC：保留引用、多选、AI 分析、阅读书签、@ 发送者、详情、本机隐藏，以及符合既有时间和所有权规则的撤回；支持收藏记录卡片，收藏列表复用受群权限约束的阅读器。
- Android：原消息菜单直接追加阅读书签、从此续读，不再出现中间菜单；保留原引用、AI 分析、多选、时间、删除及条件撤回。
- 三端追加打开记录、复制标题；复制标题不复制 JSON，也不暗示已复制完整聊天记录。
- PC 卡片补触摸/手写笔长按，移动超过 8 px 取消；长按后的 click 不打开阅读器，短按与键盘/鼠标操作保留。
- PWA 菜单随原消息变更、撤回、会话或账号变化而失效，普通刷新不关闭菜单。
- Android 操作面板在键盘可见、小屏及选项较多时限制高度并允许滚动，不把底部操作放到屏幕外。

## 未扩大的边界

记录正文、附件仍归原群权限管理。普通文字转发无法复制记录和授权，因此不把内部卡片 JSON 交给旧转发链路；Win/PWA 单项转发显示不可用原因。跨群复制记录需独立的受权导入/发布接口，不在本次冒充完成。

不改变 ZIP 导入、图片质量、缓存策略、账号登录或代理核心。引用仍提交 `message_id + revision`，服务器构造快照和校验权限，不在客户端拼接一段伪引用文本。

## 验证

- `scripts/test-record-message-menu.cjs`：实际 React 菜单、聊天记录卡片及编辑器，1280/390 宽度；触摸长按、滚动取消、短按、简洁引用和发送结构；实际 PWA 模块的刷新存活与撤回失效。
- `scripts/test-social-quotes-ui.cjs`：既有文字引用的取消、失败保留、重试、发送、返回源消息，1280/390/320，明暗两种主题。
- `scripts/test-social-chat-operations.cjs`：所有权、撤回窗口、权限失败及写请求不自动重试。
- Android：`ChatRecordMessageMenuTest` 验证完整适配器的卡片子文字长按与复用、原生共同菜单和引用动作；复用 `ChatRecordCardInteractionTest` 与 `ChatImageMessageActionsTest` 防止短按/图片菜单回归。
- 真机入口：`ChatRecordUiAcceptance.message_menu`，真实长按标题、检查同一菜单、选择引用再取消，保留已有输入，不发送群消息，不输出记录正文。

发布、装机及真机运行状态以该批次发布回执和设备验收输出为准，不能由上述测试入口的存在推定已经完成。

本批离线验证已通过：React/PWA 专项、既有引用六种视口/主题组合、社交请求合同、TypeScript/Vite 构建、改动文件 ESLint、Android 10 项定向测试（0 失败/跳过）、V2 治理检查、源码与文档体积检查。PWA 修复前的同一测试明确在“未变化刷新应保留菜单”断言失败。日志前缀：`record-menu-browser4`、`record-menu-pwa-final`、`record-menu-quote-regression`、`record-menu-pc-build`、`record-menu-lint`、`record-menu-android3`。
