---
version_status: current
reviewed_at: 2026-08-16
implementation_status: tested
---

# Win 网页 AI 生产会话结构诊断 V1

## 用户结果

Codex 与一龙控制台直接诊断生产首页使用的 ChatGPT/Google AI 官方 WebView2 会话。独立测试聊天窗已退役；窗口阶段和经过双重清洗的适配器、上下文、目录、缓存、流式状态来自同一生产会话，用于只在 `/pc/ai` 定位无上下文、无目录或无法发送的问题。

## 范围

- 数据来源是 `LocalAiBrowserRuntime` 中生产首页实际使用的后台官方会话，不创建额外网页窗口。
- `official_session` 只包含布尔值、有界计数、固定枚举、稳定动作名和稳定错误码。
- 节点收到 Tauri 回执后必须再次按白名单重建对象，不能信任前端或桌面壳直接提交的字段。
- `phase` 直接描述生产官方窗口生命周期；`official_session` 表达同一会话的详细结构状态。
- ChatGPT 与 Google AI 使用相同结构合同，仍按 provider 固定白名单查询。

## 非目标与安全边界

- 不返回消息正文、标题、草稿、引用、会话 ID、项目名称、URL、host、owner/profile 指纹或账号标识。
- 不返回 Cookie、token、密码、Authorization、网页异常详情或命令 value。
- 本批不发送第三方消息、不登录账号、不完成人机验证，也不把编译通过冒充真实网页多轮验收。
- 不恢复暂停中的跨项目自动派发或 PC 自动续跑。

## 验收标准

1. 生产会话诊断返回窗口/加载、适配器、语义快照、输入框、上下文、目录、缓存、流式与有界计数状态。
2. 会话目录结构至少区分 `navigation_snapshot_ready`、`navigation_live`、`directory_complete`、官网观察数量、合并后可用数量、项目与置顶数量。
3. 本机会话缓存只返回条目数量和是否存在当前会话，不返回任何会话身份或标题。
4. 节点二次清洗拒绝未知 provider，并对枚举、动作、错误码和计数执行白名单/上限约束。
5. 恶意回执夹带页面正文、URL、owner、Cookie、token、异常详情或未知字段时，MCP/HTTP 最终状态不包含这些内容。
6. 定向 Rust 测试、Win 控制合同测试、生产构建、正式安装版精确身份回读和 `/pc/ai` 只读结构验收通过。
7. 源码中不存在 `/pc/user-browser/native`、`local-ai-native-*`、独立聊天组件或其专属命令、权限和状态机。

## 交付矩阵

| 能力 | 实现 | 验证 | 交付 | 验收 |
|---|---|---|---|---|
| 生产后台会话存在性与就绪度 | implemented | offline_passed | deployed | installed_read_only_passed |
| 消息上下文与流式结构状态 | implemented | offline_passed | deployed | installed_read_only_passed |
| 会话/项目/置顶目录结构状态 | implemented | offline_passed | deployed | installed_read_only_passed |
| owner/provider 缓存结构状态 | implemented | offline_passed | deployed | installed_read_only_passed |
| 节点回执二次隐私清洗 | implemented | offline_passed | deployed | installed_read_only_passed |
| 独立测试聊天窗退役并收敛到生产入口 | implemented | offline_passed | deployed | installed_read_only_passed |
| 真实 ChatGPT/Google 多轮消息 | implemented | user_action_required | deployed | deferred |

## 正式安装版证据

- 前一生产诊断基线的 Windows 精确发布身份：`0.3.69+aaac049520d1368c343579178affdf552b20f565`；本次退役改动的精确身份以任务发布回执为准。
- 节点、PC 前端与 Tauri 自更新重连后均在线；生产 `/pc/ai` 的 ChatGPT `official_session` 为 `ready`，适配器、语义快照、输入框和上下文均就绪，缓存为 `live`。
- 上述旧基线仍保留测试窗阶段字段；当前发布已将阶段收敛为生产官方窗口生命周期，不再创建第二套聊天窗。
- 本次脱敏时间线共 202 条，`frontend`、`rust`、`network`、`tauri`、`control` 五类来源均有证据，错误或警告为 0。
- 未输入或发送任何第三方消息；真实多轮上下文仍保持 `user_action_required`。

## 回退

诊断缺失、字段未知或结构不合法时返回空 `official_session` 或失败回执，不影响生产首页继续使用官方页面回退；不得为获得诊断而清除 WebView2 Profile、Cookie 或本机会话缓存。
