# Windows 会话缓存与同步恢复验证

日期：2026-09-15。需求：`docs/requirements/pc-social-chat-recovery.md`。范围：React `/pc`，Windows 壳默认加载同一入口。

## 根因与修复

- 好友与群列表只在 mount 获取，Promise.allSettled 等待全部完成且吞掉错误：改为各自独立的有限时长同步，局部失败保留已有列表，区分加载、失败和真实空数据。
- 会话与消息只有组件内存，选择会话立即清空：新增按服务地址及账号隔离的近期缓存，恢复上次会话和各会话草稿。持久化上限 20 个会话、每个最多 120 条消息、序列化不超过 150 万字符；七天有效期。存储失败显示提示，退出清理缓存。
- 私聊不刷新，群聊串行轮询可能永久等待：统一采用 12 秒读取超时与取消，前台消息五秒兜底、列表十五秒、隐藏页面三十秒；推送事件合并后立即读取。
- 全局通知连接忽略 friend_message/group_message/group_message_edited，休眠后的 OPEN 连接不会重连：转发聊天失效事件，focus/pageshow/online/visibility 恢复重新连接并同步。聊天补齐不播放任务完成声音。
- 旧请求/发送回执可能跨会话：取消与账号/会话绑定隔离；发送回执与已获取同 ID 消息去重，保留已获取的版本。失败恢复草稿并保留后续输入。
- 后台读取使用 preserve_unread，前台读取同步已读。权限拒绝清除对应缓存内容。

## 已验证

`scripts/test-pc-social-chat-recovery.cjs` 运行实际 FriendsPage 和 useNotifications，在 Edge Chromium 中拦截测试接口，不修改生产聊天。覆盖：失败非空状态、好友请求挂起而群聊可用、私聊事件更新、独立草稿、失败网络下缓存重开、请求超时、OPEN 连接恢复、群聊编辑和重复事件、后台未读、网络恢复、旧响应隔离、发送失败、推送/回执竞速、权限拒绝、账号切换/退出、损坏缓存及 quota 失败。最终通过，未发生 pageerror。

实际 1280×800 浏览器截图显示：网络失败下会话、消息与草稿仍可见，暗色错误提示和重试按钮可读。截图为隔离测试场景，非用户真机验收。

- PC `npm run build`（严格 TypeScript + Vite）通过。
- 修改范围 ESLint（friends、notifications hook、auth、API client）通过。
- `test-group-message-revisions-ui.cjs` PC/PWA 编辑、冲突、历史与只读交互通过。
- `test-group-message-revisions.cjs` 修订/撤回/Unicode/待发合并通过。
- `test-social-media-downloads.cjs` 头像、图片、语音、文件、撤回和 URL 通过。
- `test-project-apk-member-download.mjs` 通过。

## 边界与发布入口

缓存是近期加速副本，不是全部聊天历史归档；媒体二进制仍依赖原资源及浏览器缓存。首次从未缓存的账号仍需联网。用户实际 Windows 托盘、系统休眠和长时间离线环境的长期验收尚待更新后反馈。

按当前发布脚本，真实公开发布身份位于 `/pc/assets/release.json`（需求中的 `/pc/release.json` 为概略入口，旧服务器可能对后者返回 SPA）。本次只改变前端，复用现有好友/群聊查询、未读参数和推送事件，无服务端合同变化。
