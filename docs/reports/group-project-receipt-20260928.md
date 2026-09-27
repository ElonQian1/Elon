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
- `f9d50568e` 已发布并安装 `1.1.1818 / 1818`，APK SHA-256 `400c4da3bc3b6f20509710b6c9f0781c5a9c65624013021222f3062cdcb1d5cc`。解锁后真实多选验收已恢复之前创建的项目并保存绑定：`project_ready -> project_server_bind`；只读服务器确认 `ready`、generation 1、有项目、无会话。没有重复创建。
- 完整回答/复用仍未通过：项目页就绪后收到 `runtime_rejected`，发送探针为 `idle`、未 dispatch、未 accepted、零流事件。不能把项目恢复成功当作整条业务成功，也不自动重放这次已授权请求。
- Windows 项目绑定、多个主题选择与官方分享链接导入/分叉不在本次修复完成范围；其他已交付群图片能力以对应专项报告为准。

## 新运行时项目发送

后续审计发现 Rspack context 仅允许普通根路径和 `/c/`，且拒绝非空项目 atom；它会直接拒绝已恢复的项目页面。Rspack admission 返回 `{profile,stage,code}`，原生只接受版本化 `{schema,stage,code}`，因此还会误报 `invalid_protocol_evidence` 并多等五次。

- 已核对公开源码 `934244.b068731984.js` 的 `CUv.a` 接收 `projectId` 并传递给官方 completion；`184143.0e420b28d4.js` 的 `JqV.K` 为当前会话项目 atom。资产 SHA-256 分别为 `e23a29fd7ee8ae21081eef0147d7d561b57fa619d165c52155f402766b4b1594`、`7ebd1e14d160fc99199c520761ba2e14580c865a21558ef982f6fa832d5e1f5e`。
- 适配精确项目路由，校验已提交组件树、账号、会话与项目身份。新本地会话的项目 atom 尚未持久化时，必须同时匹配组件祖先项目 ID 和项目路径；不能仅凭 URL 发请求。
- 官方发送携带校验后的 `projectId`；保持原有官方事务、单飞、未知写不重放、附件和草稿保护。上下文切换时禁止继续发到其他项目。
- admission 统一使用现有原生可校验的版本化结构，无账号、私密内容或令牌字段。
- 此补丁离线回归与正式包/真机验证分开记录，尚未宣告全链路通过。
