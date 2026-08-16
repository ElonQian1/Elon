---
version_status: current
reviewed_at: 2026-08-16
implementation_status: tested
---

# Win 网页 AI 实时上下文与目录连续性 V1

## 用户结果

Windows 生产首页的同一个聊天 UI 在 ChatGPT 与 Google AI 之间切换时，立即显示各自本机缓存；后台官方页继续同步当前会话、回答、会话列表和项目，不把一次不完整的网页采集误当成完整目录。发送和目录同步必须等待各自动作真实可能需要的时间，不能在官网仍处理中提前显示失败。

## 范围

- ChatGPT 与 Google AI 仍使用按一龙 owner 和厂商隔离的本机 WebView2 Profile。
- 生产首页继续使用 `AiChatPage`、原消息气泡、原输入框和同一侧栏；旧测试子窗口已经退役。
- ChatGPT 会话目录支持快速可见快照、后台完整采集、置顶、项目、普通聊天和本机缓存。
- ChatGPT 可见消息窗口保留稳定重叠上下文；Google AI 继续保留同一搜索会话的多轮合并。
- Cookie、token、密码、完整 URL 查询和聊天正文不进入日志、命令回执或功能登记。

## 验收标准

1. `send_prompt` 与 `list_conversations` 使用动作级回执期限；官网在声明的最长处理时间内完成时，一龙不能在 2.4 秒提前报超时。
2. ChatGPT 目录先返回当前可见项和 `complete=false`，完整滚动采集随后更新；部分或失败采集不能删除已有会话、项目和置顶状态。
3. 完整目录可以移除已从官网全局历史中消失的普通会话，但仍保留项目内历史，规则与 APK 当前索引合并策略一致。
4. ChatGPT 消息快照按 `messageWindowStart`、`observedMessageCount` 和稳定消息 ID 合并，切换和虚拟化窗口不能无故清空仍属当前会话的上下文。
5. 切换 ChatGPT 与 Google AI 时先显示对应 owner/provider 缓存，后台再刷新；缓存状态不能解锁写操作。
6. Rust 行为测试、共享适配器语法/版本奇偶、PC TypeScript/ESLint/生产构建和生产首页静态合同通过。

## 交付矩阵

| 能力 | 实现 | 验证 | 交付 | 验收 |
|---|---|---|---|---|
| 主页面消息收发回执期限 | implemented | offline_passed | not_started | pending |
| ChatGPT 多轮可见上下文 | implemented | offline_passed | not_started | pending |
| 会话、置顶与项目目录连续性 | implemented | offline_passed | not_started | pending |
| owner/provider 缓存回显 | implemented | offline_passed | deployed | pending |
| ChatGPT / Google AI 厂商切换 | implemented | offline_passed | deployed | pending |
| 真实第三方多轮消息 | implemented | user_action_required | deployed | deferred |

## 验收边界

离线测试和生产构建证明内部合同闭合，不证明当前账号、地区、Cloudflare 或厂商 DOM 一定可用。向 ChatGPT 或 Google AI 真实发送消息属于可逆但代表用户对第三方发言的操作，必须在用户明确授权后，仅从 Windows 生产首页执行并只保存脱敏结构回执。
