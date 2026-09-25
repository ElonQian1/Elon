# Win 群聊 AI 在 HTTP 页面启动失败

## 根因与修复

- 群聊 AI 操作编号和继续讨论交接编号直接调用 `crypto.randomUUID()`。
- Win 加载的 HTTP 工作台可能不是 secure context；`getRandomValues` 可用但
  `randomUUID` 不存在。异常发生在 `startGroupAi` 创建任务之前，尚未准备、
  派发或发送问题，不是 ChatGPT 网络失败。
- 两处改用项目已有 `uuid` 依赖的 `v4`；缺少 `randomUUID` 时使用
  `getRandomValues`，保持 Rust 宿主要求的标准 UUID，以及服务器幂等编号。
- 不使用时间戳、本地队列 ID、`Math.random` 或全局 crypto polyfill。

## 验证

- `npm run test:group-ai`：17 项通过，涵盖完成、取消、账号切换、幂等派发、
  不确定发送不重放、交接隔离及编号格式。
- `test-group-ai-browser.mjs`：真实 Chromium HTTP 非 secure context，
  `randomUUID` 缺失、`getRandomValues` 可用；图片右键入口、多选、标准 UUID、
  所有后续调用沿用同一个编号、完成回答只投递一次、返回原群均通过。
- `test-group-ai-context-ui.mjs`：禁用 `randomUUID` 后，1280/390 宽度下，
  来源阅读、分享设置、继续讨论和保护原有草稿通过。
- TypeScript/生产构建及修改模块 ESLint 通过。
- 浏览器测试使用替身服务与回答，未发送真实群消息，不代表真实识图验收。

## 尚未完成：群聊原图分析

这是另一个现有缺口，不是 UUID 错误的成因。当前 PC 群聊所选上下文只发送
文字和附件说明，`GroupAiSelectionDialog` 已明确提示不包含附件原文件。
`group_ai_web_session` 尚未提供将所选群图片传入隔离会话的上传动作。
本次保留真实提示，不把附件 URL 拼进问题冒充已经上传。

后续应按选区版本读取用户有权访问的图片，复用同源私有附件传输，确认全部
附件就绪后再派发一次；失败不得悄悄降级成仅文字的“图片分析”。同时覆盖
撤回/编辑、账号切换、下载失败、文件限额和上传完成前取消。需要单独的桌面
宿主实现、构建发布与真实图片分析验收。
