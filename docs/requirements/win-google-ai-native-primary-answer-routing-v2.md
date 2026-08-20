---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-21
---

# Win Google AI 原生正文路由与前台连续性 V2

## 目标

修复生产 Win 首页从一龙原生输入框向 Google AI 模式发送消息时的组合回归：发送必须始终留在
一龙聊天页，官网完整主回答必须同步到原生消息区，右侧来源结果集合只能作为引用，不能替代正文。

## 必须实现

- 原生 `send_prompt` 在执行官网命令前，显式把官方 WebView 收回后台并声明聊天页为当前前台意图。
- 用户返回聊天页后，较早开始、较晚完成的官方页显示操作不得再次把 WebView 提到前台。
- Google 回答候选必须识别包含多个链接标题与摘要的来源结果集合，即使这些结果由多个长列表项组成。
- Google 正文与用户问题处在同一视觉列时，正文中的引用链接和带引用的编号项目不得被误判为右侧来源结果集合。
- 响应式窄窗口把来源列表折叠到正文同一列时，以“复制文字/Copy text”等回答操作区作为结构边界：操作区前是正文候选，操作区后的搜索结果不得成为正文。
- 候选最终选择阶段必须再次排除已经标记的来源集合，避免分数或 DOM 顺序让它覆盖主回答。
- 保留官方页主动入口、来源链接、访客 Profile、上下文绑定、富文本、安全引用和后台回复刷新。

## 非目标

- 不修改 PWA、Android UI 或移动端发布物。
- 不复制 Google 官方 UI，不读取网络请求、Cookie、账号凭据或私有接口。
- 不重做聊天页面、来源组件、内部标签页或厂商适配器架构。
- 不改变用户主动点击“显示官方页”时的现有行为。

## 验收标准

1. 原生发送前会隐藏官方 WebView；发送完成、导航和回答刷新均不会自动切换官方标签。
2. 延迟完成的旧官方页显示请求在聊天前台意图生效后会被再次隐藏，不能覆盖原生界面。
3. 三条来源标题与摘要组成的长列表不会被选为 AI 正文，即使它有多个 narrative block 和更高分数。
4. 同页存在主回答和来源卡时，原生消息显示主回答富文本，来源只作为引用链接追加。
5. 带多个行内引用的编号正文仍被识别为主回答，不能退化为右侧来源标题与摘要列表。
6. 来源列表因窄窗口折叠到正文同一列时，回答操作区之前的三点正文仍胜出，操作区之后的来源列表失败关闭。
7. 官方页主动打开/关闭、Google 游客会话、上下文绑定和新会话能力保持可用。
8. 定向 JS 合同、PC lint/build、Rust 定向测试及发布后真实 Win 发送闭环通过。

## 实现范围

- `android/app/src/main/assets/google_web_answer_candidate_policy.js`
- `android/app/src/main/assets/google_web_message_extractor.js`
- `scripts/test-google-web-answer-candidate-policy.cjs`
- `scripts/test-google-web-message-extractor-contract.cjs`
- `pc-frontend/src/features/user-browser/AiBrowserExperience.tsx`
- `pc-frontend/src/features/user-browser/useLocalAiWebChatController.ts`
- `pc-frontend/scripts/test-ai-browser-tabs.cjs`
- `docs/user-browser-module-integration.md`
