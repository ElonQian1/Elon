---
version_status: current
reviewed_at: 2026-09-26
implementation_status: implemented
---

# Win 群聊 AI 业务 MCP

## 范围

`win_control` profile 增加 `win_group_ai_action` 和 `win_group_ai_action_status`，
不依靠坐标、截图、任意 JavaScript 或开发工具。
前端桥调用与“AI 回复到群聊”按钮相同的 `startGroupAi` / `GroupAiTask`，继续使用
已有附件清单、私有上传、官方身份宿主、单次派发与服务端幂等回群流程。
不改变普通聊天草稿，不修改分享权限，不用旁路脚本模拟 AI 答案。

## 操作

1. `groups` 返回当前登录用户的群 ID、群名和短期 `owner_binding`；每页 20 项。
2. `messages` 传绑定和群 ID，返回最近 120 条内的消息分页，每页 30 项，含消息 ID、
   修订号、最多 240 字预览和图片标记。保留未读状态，不导出附件地址或字节。
3. `start` 传 UUID `command_id`、绑定、群 ID、选择的消息 ID/修订号、问题和
   `confirmed=true`。调用前必须已获用户对该群、选择内容和发送行为的明确授权。
   固定使用 ChatGPT；会话继续分享保持关闭，图片失败不降级成只发文字。
4. 每个命令先查询 `win_group_ai_action_status(command_id)`，等待命令完成。
   `queued` 或命令 `completed` 均不代表 AI 已回答。
5. 对返回的 `task_id` 提交 `status`，看到 `phase=completed` 后仍须确认
   `delivery_verified=true` 和 `source_verified=true`。验证实际回读群消息内容和来源记录，
   不是只看本地完成标记。
6. `resume` 仅核对已派发任务，不重新发送；未派发失败不能经它自动重试。
   `cancel` 停止处理，不撤回已经发出的消息。

传输丢失时复用相同 `command_id` 和参数。不同参数复用 ID 被拒绝。
账号切换、退出重登或页面重载使旧绑定失效，必须重新发现；过期、已领取但失联的命令
不会由新前端重放。页面重载后找不到内存任务时返回 `task_not_found`，不能猜测成功。

## 边界

- 仅项目绑定 MCP 可排队；回执只允许所属项目查询。
- 前端领取和回执复用本机管理员令牌与可信 Origin 校验，没有新增公网入口。
- 队列最多 256 项、领取截止 120 秒；完成后删除问题正文，仅保留重试指纹。
- 业务结果与普通诊断时间线分离；日志及诊断导出不记录问题、聊天、Cookie 或令牌。
- 结果白名单清洗；没有任意 URL、文件路径、HTTP 请求或脚本执行能力。
- `stage` 区分连接、上传、发送准备、派发、接收与回群，不把未就绪误报为功能不存在。

## 验证

`pc-frontend/scripts/test-group-ai-control.cjs` 覆盖账号切换、重复选择、修订漂移、撤回、
权限、分享关闭、禁止重新发送及群消息/来源回读。沿用 `test-group-ai-task.cjs` 的
附件先确认后派发、发送不明不重试和幂等回群回归。
Rust `group_ai::tests` 覆盖命令验证、项目隔离、单次领取、幂等冲突、过期不重放及字段净化。

现场图片分析与回群验收待发布后执行；编译、离线回归和安装成功不能代替真实回复。
