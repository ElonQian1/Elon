---
version_status: current
reviewed_at: 2026-09-27
implementation_status: implemented
---

# Win 群图片 MCP 现场验收

## 结果

2026-09-27 00:40（Asia/Shanghai），使用用户已登录的 Win 账号，在用户指定的测试群，
完成原来指定图片的后台问答与回群。未替换成较容易的图片，未使用截图、坐标或人工拼接回答。
验收问题要求描述物品、场景与布置，不识别人物身份。

实际调用 `win_group_ai_action` / `win_group_ai_action_status`：

1. `groups` 精确匹配指定群，并取得当前账号的绑定。
2. `messages` 找到原始图片消息，核对修订号；选择 1 张 JPEG，128238 字节。
3. `start` 调用生产群回复任务，先上传真实图片，再通过已审核的官网运行时发送。
4. `status` 返回完整回答并完成回群。
5. 再次 `status` 与 `messages` 回读，核实正文和原消息来源。

最终业务结果：

```json
{
  "phase": "completed",
  "stage": "completed",
  "busy": false,
  "dispatched": true,
  "source_count": 1,
  "attachment_count": 1,
  "message_count": 3,
  "has_answer": true,
  "answer_chars": 159,
  "delivery_verified": true,
  "source_verified": true,
  "error_code": null
}
```

群内实际消息以“图片验收回复”开头，为三句完整场景描述；不是占位消息或仅发送成功回执。
群消息创建时间为 `2026-09-26T16:40:57.709159764+00:00`。
正文、账号、群标识、原图及凭证不写入本报告。

## 故障与修复

- 现场捕获到 `runtime_fallback:runtime_not_observed`：上传、发送准备已通过，
  发送时却因性能资源记录不再包含运行时而走旧路径，导致官网输入框未接受文本。
- `405c5a2ea` 将审核过的资源证据限定在当前文档内保存；清空性能记录不再改变
  运行时判定。模块是否真正执行、导出契约与身份仍逐次验证，未放开未知构建。
- `d4dcf51da` 让隔离任务读取已接受请求的会话绑定，避免重新从替换后的首页输入框
  寻找回答；账号或文档变化仍中止读取。
- 本轮还保留附件归属、编辑器重建、模型准备及令牌刷新相关的前置修复。
  早期“接受发送但未取到回答”的尝试均未按成功登记，也未自动重放。
- 任务级诊断字段已经接入；本次成功回执中的该可选字段为空，未将其作为验收依据。

## 发布与验证

- 功能提交：`405c5a2ea0bc2b3a64cc96c20fc747d4d2297033`，已推送 `origin/main`。
- 服务端及 PC 前端：`0.3.1781`，对应上述提交，发布健康检查通过。
- Win 本地节点及桌面实际运行身份：
  `0.3.69+405c5a2ea0bc2b3a64cc96c20fc747d4d2297033`。
- 无人值守安装验收：`6429302a-f2ea-4368-971a-6658fe61fff2`，
  `target_release`、`final_release`、`desktop_release` 一致，状态 `passed`。
- Rspack 运行时、附件、发送和消息定向回归：90/90 通过。
- 群任务及 MCP 前端回归通过；Rust `node_agent_win_group_ai` 定向测试通过。
- `check-task-complete.ps1 -Kind NodeAgent` 和 `-Kind Server` 在功能提交上通过。
- 本批未发布新的 APK，也不将 Win 的现场结果当作 Android 真机验收。

构建与验收日志位于共享仓库 `.git/ai-command-logs`，前缀为
`group-runtime-evidence-js`、`group-task-observation-rust`、
`group-runtime-publish-server`、`group-runtime-publish-win` 和 `group-mcp-complete-*`。
Win 安装回执位于本机 `Elon/win-conversation-acceptance-v1` 数据目录。

## 适用边界

这次证明的是指定原图的真实闭环，不代表所有官网构建、模型、附件类型都已验收。
未知运行时构建仍需审核；未收到完整回答的请求继续保持“不确定、禁止自动重发”。
Cookie、登录态、原草稿与原群消息均未清除。
操作契约继续以 [Win 群聊 AI 业务 MCP](../win-group-ai-mcp.md) 为准。
