---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-20
---

# Win Google AI 主回答与发送界面连续性 V1

## 目标

Win 生产首页的一龙原生聊天输入框向 Google AI 模式发送消息后，必须留在一龙聊天页，
并把官网当前轮的主回答正文同步为 assistant 消息；官网来源卡片继续作为引用展示，不能替代正文。

## 必须实现

- 只有用户主动点击“显示官方页”或官方标签时，才允许切换到官方页面。
- 官方 WebView 的后台导航、恢复、加载或 `windowVisible` 状态变化不得自动打开官方标签。
- Google AI 回答候选选择必须优先主回答容器与连续正文，来源卡片集合不能依靠引用数量赢过正文。
- 正文中的标题、段落、编号列表、强调、表格和公开引用继续复用现有富文本与来源组件。
- 保留访客 Profile、Cookie、上下文绑定、乐观发送、后台刷新、手动官方页和系统浏览器入口。

## 非目标

- 不复制 Google 官方 UI，也不读取网络请求、Cookie、账号凭据或私有接口。
- 不修改 PWA 页面或 PWA 发布产物。
- 不发布 Android APK；共享 Google 可见语义模块的修复只随本次 Win 客户端交付。
- 不重做聊天布局、来源组件或内部浏览器标签。

## 验收标准

1. 原生发送成功和官方 WebView 状态刷新都不会自动派发官方标签事件。
2. 用户主动显示、切换、收起官方页的现有入口继续可用。
3. 带编号正文和多个来源卡片的 Google AI 页面选择正文为主内容，来源只追加为引用。
4. 原生 UI 同时显示正文富文本和来源链接，不再只显示三条来源摘要。
5. Google 访客会话、同机 Profile 与上下文连续性保持不变。
6. Win 用户浏览器合同、前端构建、Rust 定向测试、发布后真实复杂问答验收通过。

## 实现范围

- `pc-frontend/src/features/user-browser/AiWebChatSidebar.tsx`
- `pc-frontend/scripts/test-ai-browser-tabs.cjs`
- `android/app/src/main/assets/google_web_message_extractor.js`
- `android/app/src/main/assets/google_web_answer_candidate_policy.js`
- `scripts/test-google-web-answer-candidate-policy.cjs`
- `scripts/test-google-web-message-extractor-contract.cjs`
- `docs/user-browser-module-integration.md`
