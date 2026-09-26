---
version_status: current
reviewed_at: 2026-09-26
---

# 手机网格对话入口

需求：[APK 手动网格对话与只读 MCP](requirements/android-binance-grid-chat-v1.md)。

## 手机聊天

在一龙的 ChatGPT 聊天里打开输入框“+”，点击“附带网格”。它读取手机当前登录的
币安账户，显示币种、方向、杠杆、格数和精确策略 ID。选择后读取新详情并预览，
点击“加入输入框”才把快照放入可编辑草稿。输入问题后发送，沿用现有 ChatGPT
私有发送器。再次打开“附带网格”可重新读取或移除快照。

普通聊天、编辑和发送都不会刷新币安。快照最多五分钟；来源、会话或快照数据被改动后，
必须重新读取。发送失败保留的完整草稿不会再附加第二份快照。手机与 Win 的币安登录
相互独立；空列表表示当前手机账户的实际响应，不能自动换用 Win 数据。

## AI 调用

沿用手机本机认证 MCP，不开放任意网页脚本或交易操作。使用项目登记手机和身份验证
建立 ADB forward，再向该设备的 `/mcp` 调用 `binance_grid_read`。认证凭据只留在
本机传输层；不得写入聊天或收据。

开始列表：`{"kind":"list","request_id":"example_list_001","start":true,"limit":25}`。
轮询同一请求省略 `start`；`ready` 后按 `next_offset` 分页，同一快照的时间和条目稳定。
开始详情：`{"kind":"detail","request_id":"example_detail_001","strategy_id":"123456","start":true}`。
详情先刷新本人列表核对精确 ID，再读取固定详情接口。错误目标返回 `strategy_not_found`。

原生 `binance_grid_attachment` 的 `status/open/remove` 操作复用聊天输入框同一个入口，
供无人值守验收检查空列表、打开预览流程和移除草稿。返回值只有状态与计数，不自动发送。

已有 `scripts/web-conversations/stdio.mjs` 提供 `binance_grid_list` 和 `binance_grid_detail`
两个供应商无关工具；省略 native `kind`，其他参数相同。必须配置 `ELON_APK_MCP_URL`
为已验证手机转发的 `http://127.0.0.1:<port>`。此入口固定 APK，不回退 Win。

Codex/Claude 的持久注册流程仍使用[现有注册器](personal-web-conversation-reader.md)，
它把全部模块保存到内容寻址目录，避免任务目录清理后失效。更新注册器资产后需要重新
注册并重载客户端工具；服务器协议验收不等于某一 AI 客户端已经发现并使用新工具。

## 状态与错误

- `pending`：只轮询同一 request_id，不重复 start 新 ID。
- `ready`：返回固定字段、来源和采集时间；金额为字符串、未知为 null。
- `snapshot_not_found` / `snapshot_expired`：旧请求不能恢复，显式创建新请求。
- `context_changed`：账号或文档变化，重新检查当前会话后再读取。
- `host_busy`：已有管理/读取操作占用宿主，等其完成后使用新请求。
- `list_context_unavailable` / `login_required`：需要用户在手机官网完成登录或打开网格列表。
- `apk_transport_unavailable`：手机休眠或无线转发不可达；通过项目登记信息重连，
  核对硬件身份，恢复本机转发，必要时唤醒手机。先轮询原请求，不自动重复发起读取。
  新的 Android 信任/配对提示仍需用户在手机上确认。

代码、发布、安装、真实空列表、非空详情及聊天发送分别验收；完整证据在
[APK 验收记录](reports/android-binance-grid-chat-20260926.md)。
