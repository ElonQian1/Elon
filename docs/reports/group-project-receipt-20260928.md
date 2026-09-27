---
version_status: current
reviewed_at: 2026-09-28
---

# 群项目结构化回执修复

本轮继续 [每群 ChatGPT 项目与长期会话](group-chatgpt-project-20260923.md)，不把代码、发布和真实业务验收混为一谈。

## 现场证据

- 无线 ADB 已核对登记手机硬件身份；安装版本为 `1.1.1817 / 1817`。
- 服务器 `0.3.1781` 源码 `405c5a2ea0bc2b3a64cc96c20fc747d4d2297033` 已包含 `90bda5f5d` 的项目恢复接口，无须重复部署旧服务器候选。
- 原生群聊多选两条合成消息，启用本群项目后依次出现 `project_identity_ready`、`project_server_acquire`、`project_unavailable`、`failed_before_authorize`。
- 只读服务器记录：一个绑定仍处于 `creating`、generation 1，尚无已保存项目或会话。该状态不证明官网没有创建资源，也不允许自动重发创建。

## 确定缺陷

`ChatGptWebPrivateProtocolEvidence.detail()` 把普通命令详情限制到 160 字符，群项目的完整 JSON 回执没有专用分支。成功身份回执较短所以能通过；带账号范围、项目 ID 和记忆范围的 `project_ready` 回执超过限制，被截断后变成非法 JSON。协调器又把解析错误归为 `project_unavailable`。

此前协调器单测直接构造事件，跳过真实协议解析，遗漏了该集成边界。新增回归从 `ChatGptWebProtocol.parse` 进入；旧代码实测失败：`Unterminated string at 160`，日志 `group-project-receipt-red-20260928-20260928-021630-425`。这是确定的代码缺陷，完整手机恢复结果另行记录。

## 修复边界

- 独立 `GroupChatGptProjectReceipt` 接收最多 1024 字符的结构化结果，校验成功/失败类型、账号范围、项目与会话 ID、项目记忆范围、错误码及可选身份原因。
- 不截断 JSON，不整体扩大普通命令详情上限；未知字段、错误类型、超长及坏 JSON 返回固定 `project_receipt_invalid`。
- 未知创建结果不得附带 `notSent=true`；坏回执不伪造未发送证明、不解锁自动重建。
- 协调器测试也改经真实协议解析。保留租约、账号隔离、项目稳定 ID、未知创建先查找、明确确认后重建和已完成回答独立回群。
- 不改网页传输、语音、代理、Cookie 或登录态。不触碰其他任务尚未提交的页面适配器修改。

## 验证与交付

- 官网项目 Node 定向回归：16 项通过。
- 原生协议 26、协调器 7、结构化回执 4、路由 3 项全部通过（共 40 项，失败/错误/跳过均为零）；日志 `group-project-receipt-green-20260928-20260928-022539-931`。包含超过旧上限的成功回执、坏格式/额外字段/错误类型、未知写不可假报未发送，以及其他命令仍保留旧长度限制。
- 正式包发布及手机完整恢复/回答/复用进行中，未以编译代替验收。
- Windows 项目绑定、多个主题选择与官方分享链接导入/分叉不在本次修复完成范围；其他已交付群图片能力以对应专项报告为准。
