# Win Google AI 游客新对话快速恢复 V1

## 目标

当 Google AI Mode 官方页未登录、语义适配器尚未连接，或原生界面只剩旧会话缓存时，Windows 生产首页的“新对话”必须仍然可用。操作应立即进入可见的恢复流程，不得以无说明的禁用按钮或长时间等待命令回执结束。

## 范围

- “新对话”作为恢复型动作，不依赖当前会话已经可以发送。
- 适配器未连接、上下文未绑定、缓存仍为只读或输入框未就绪时，回到厂商固定主页并重新建立新会话上下文。
- 保持发送的严格上下文门禁；恢复期间不得把缓存会话误判为可写。
- 显示明确、可操作的进行中与降级提示。
- ChatGPT 和 Google AI 共用恢复策略；本轮重点验收 Google 游客模式。

## 非目标

- 不读取 Cookie、Token、请求正文或厂商私有接口。
- 不伪造官方命令成功回执。
- 不修改 PWA，也不复制新的 Google/ChatGPT 网页适配器。
- 不改变官方网页的登录、地区、语言或真人验证限制。

## 验收标准

1. Windows 客户端可用且厂商声明支持新会话时，“新对话”不会仅因 `canSend=false` 而被静默禁用。
2. Google 游客页处于 `adapter_connected=false`、`context_ready=false` 或缓存只读状态时，点击后立即触发固定主页恢复，不等待不存在的适配器回执。
3. 恢复过程中保留 `canSend=false`，直到新的官方页面快照重新绑定上下文。
4. 正常已连接会话继续使用原有厂商适配器 `new_conversation` 命令和匹配回执。
5. 用户能看到“正在建立新会话”或失败原因，不再出现无反馈点击。
6. 用户状态、恢复策略、PC 用户浏览器合同和 TypeScript 构建测试通过。

## 预计实现范围

- `pc-frontend/src/features/user-browser/localAiUserState.ts`
- `pc-frontend/src/features/user-browser/localAiNewConversation.ts`
- `pc-frontend/src/features/user-browser/useLocalAiWebChatController.ts`
- `pc-frontend/src/features/ai/AiChatPage.tsx`
- `pc-frontend/src/features/user-browser/AiWebChatSidebar.tsx`
- `pc-frontend/scripts/test-local-ai-user-state.cjs`
- `pc-frontend/scripts/test-local-ai-new-conversation.cjs`
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs`
