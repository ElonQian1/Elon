---
version_status: current
reviewed_at: 2026-08-16
implementation_status: tested
---

# Win 网页 AI 生产会话结构诊断 V1

## 用户结果

Codex 与一龙控制台能够区分“测试原生子窗口”和“生产首页后台官方会话”。即使测试子窗口从未创建，仍可从生产 WebView2 会话读取经过双重清洗的适配器、上下文、目录、缓存和流式状态，以便只在 `/pc/ai` 定位 ChatGPT/Google AI 无上下文、无目录或无法发送的问题。

## 范围

- 数据来源是 `LocalAiBrowserRuntime` 中生产首页实际使用的后台官方会话，不创建额外网页窗口。
- `official_session` 只包含布尔值、有界计数、固定枚举、稳定动作名和稳定错误码。
- 节点收到 Tauri 回执后必须再次按白名单重建对象，不能信任前端或桌面壳直接提交的字段。
- `phase=not_created` 继续只描述一龙原生测试子窗口；生产会话存在性由 `official_session` 独立表达。
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

## 交付矩阵

| 能力 | 实现 | 验证 | 交付 | 验收 |
|---|---|---|---|---|
| 生产后台会话存在性与就绪度 | implemented | offline_passed | deployed | installed_read_only_passed |
| 消息上下文与流式结构状态 | implemented | offline_passed | deployed | installed_read_only_passed |
| 会话/项目/置顶目录结构状态 | implemented | offline_passed | deployed | installed_read_only_passed |
| owner/provider 缓存结构状态 | implemented | offline_passed | deployed | installed_read_only_passed |
| 节点回执二次隐私清洗 | implemented | offline_passed | deployed | installed_read_only_passed |
| 真实 ChatGPT/Google 多轮消息 | implemented | user_action_required | deployed | deferred |

## 正式安装版证据

- Windows 精确发布身份：`0.3.69+aaac049520d1368c343579178affdf552b20f565`。
- 节点、PC 前端与 Tauri 自更新重连后均在线；生产 `/pc/ai` 的 ChatGPT `official_session` 为 `ready`，适配器、语义快照、输入框和上下文均就绪，缓存为 `live`。
- 同一回执中测试子窗口仍为 `phase=not_created`，证明该阶段不再冒充生产会话状态；Google AI 当前未创建生产会话，不把缺失会话伪造成已验收。
- 本次脱敏时间线共 202 条，`frontend`、`rust`、`network`、`tauri`、`control` 五类来源均有证据，错误或警告为 0。
- 未输入或发送任何第三方消息；真实多轮上下文仍保持 `user_action_required`。

## 回退

诊断缺失、字段未知或结构不合法时返回空 `official_session` 或失败回执，不影响生产首页继续使用官方页面回退；不得为获得诊断而清除 WebView2 Profile、Cookie 或本机会话缓存。
