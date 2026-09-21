# 群 AI 分析发送失败诊断（2026-09-22）

本记录为只读诊断证据，不表示功能已修复。代码基线 `cd4408528`，小米运行 APK `1.1.1800`。通过已登记无线 ADB 校验硬件身份后，使用 APK MCP `trace_recent`，并用服务器只读 SQLite 查询交叉核对。未重试分析、发送消息、修改数据库或清理登录态。

## 最近两次证据

北京时间；服务器创建时间略晚于客户端开始时间。

| 服务器请求创建 | 终态更新时间 | 服务器状态 | 群回答 |
|---|---|---|---|
| 09-22 01:23:31.862 | 01:24:18.843 | indeterminate | 未生成 |
| 09-22 01:25:33.598 | 01:25:44.160 | indeterminate | 未生成 |

两次客户端均为相同过程：

1. 临时文档已登录，`login_required=false`、`composer_ready=true`、`private_send_ready=false`，没有消息或草稿。
2. 私有发送只读准入连续五次返回 `runtime_unavailable` / `base_context`。
3. 转入 `official_send_fallback`，随后申请服务器单次发送授权。
4. 发送命令返回 `ok=false`、`reason=draft_not_accepted`、`authority=official_page`、`indeterminate=false`。发送命令到拒绝回执约 110 / 155 毫秒，不是等待回答三分钟超时。
5. 失败探针为 `send_phase=idle`、`dispatched=false`、`accepted=false`、`stream_events=0`。该探针描述私有发送状态，不能独自证明 DOM 发送是否发生，须结合第 4 项及代码分支。
6. 执行器记录 `failed_after_authorize`，服务器最终记为 `indeterminate`，用户得到“请求可能已发送，但没有收到完整回答”的提示。

同群此前 09-21 15:23 的精选分析完成并生成群回复；不能把此前成功当成这两次成功，也不能据此认定群投递或新增来源卡片是本次故障点。

## 已确认的代码问题

- `android/app/src/main/assets/chatgpt_web_text_transaction_orchestrator.js` 的 `sendPrompt` 在 `setComposerValue` 返回 false 时发出“官方输入框未接受文本”。该分支在 `waitForStableSendButton` / `button.click()` 之前返回。因此这两次观测停在发送准备阶段，而非已有完整回答但群投递失败。
- `android/app/src/main/kotlin/com/elon/app/chatgptweb/GroupWebAiExecutor.kt` 在调用页面发送前设置 `dispatched=true`；`fail()` 仅用 `dispatching || dispatched` 推导 `uncertain`，没有利用拒绝回执的确定性。单次授权和实际供应商请求派发被混成同一状态。
- `GroupWebAiFailureInspection.kt` 只记录失败信息，然后统一调用原 `fail()`，无法纠正上述分类。
- `GroupWebAiFeature.kt` 将该分类回写服务器 `uncertain`，并且 `GroupWebAiFailure.canRetry` 禁止用户重试。这是误报持续暴露为不可恢复失败的原因。

## 仍不能确认

- `base_context` 聚合了多项前置条件：官网版本 profile、模块函数、会话绑定等。现有记录不能定位到底缺哪个条件，不能断言是官网升级、登录失效或网络故障。
- `draft_not_accepted` 证明写入未被适配器确认，但没有记录具体编辑器类型和写入失败细项。失败时独立文档被销毁，事后检查个人聊天页不能替代该文档证据。
- 两次没有出现登录要求或页面加载错误；首次前置页面加载较慢，但不足以把发送拒绝归因于网络。

## 后续修复顺序

1. 在原独立群聊文档内加入有界、只读的准入细项和编辑器写入诊断，修复已证实的运行时绑定或编辑器适配缺口，不通过跳过校验假装就绪。
2. 区分授权、确定未发送、已派发结果未知、已收到完整回答、群投递成功。只对明确的发送前拒绝开放安全重试；服务器单次授权的释放或重新授权必须与客户端一并设计，不能简单翻转布尔值或自动改走计费 AI。
3. 真正结果未知时保留去重保护，优先查询已有结果。不能对所有失败自动重复 POST。
4. 用真实群分析验证从准备到群回复的整条链路；本次只完成诊断，未执行新的群分析或宣称修复成功。
