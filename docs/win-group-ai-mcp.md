---
version_status: current
reviewed_at: 2026-09-27
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

无需先打开群聊页面。首次有群命令时，主窗口只唤起隐藏的
`/pc/group-ai-worker` 执行页，不领取命令、不导航、不抢焦点。
执行页与主窗口使用相同 WebView2 Profile、云端 Origin 和已有 `elon_auth`，
不把 Cookie、用户令牌或本机管理员凭据交给 MCP。
原 `open_group_workbench` 仍可用于用户明确要求显示群聊时的前台导航，但不是业务前置条件。

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
- 隐藏执行页只获厂商能力列表和独立群分析会话权限；禁止个人会话控制、导航、清数据、
  文件读取和剪贴板权限。原生层核对固定 label 与完整受信入口 URL，并禁止弹窗/异站导航。
- 登录变化通过同源存储同步；发现群和派发前核对服务端身份与本机绑定。
  本机任务页没有云端令牌不等于用户退出云端群账号。
- 一个后台执行页单飞领取命令，不重建仍可能在发送中的宿主；退出重登使旧绑定失效。

## 验证

`pc-frontend/scripts/test-group-ai-control.cjs` 覆盖账号切换、重复选择、修订漂移、撤回、
权限、分享关闭、禁止重新发送及群消息/来源回读。沿用 `test-group-ai-task.cjs` 的
附件先确认后派发、发送不明不重试和幂等回群回归。
Rust `group_ai::tests` 覆盖命令验证、项目隔离、单次领取、幂等冲突、过期不重放及字段净化。
`test-group-ai-worker.cjs` 覆盖本机页面不消费命令、隐藏执行页单次执行、回执重试、
登录同步和最小权限；原生 `group_ai_worker::tests` 验证固定路由、Origin 和 loopback 限制。

2026-09-27 已通过原图现场验收：已有 Win 登录账号、指定群与原消息、私有图片上传、
ChatGPT 完整回答、单次回群及消息/来源回读，均通过后台 MCP 完成，没有界面点击。
结果为 `phase=completed`、`delivery_verified=true`、`source_verified=true`。
版本与具体证据见 [现场报告](reports/win-group-image-mcp-20260927.md)。
编译、离线回归、安装成功或发送受理，均不能单独代替上述业务结果。

## 运行时维护

- Rspack 构建证据按文档令牌缓存。Resource Timing 可被页面清空，不能据此断言
  已加载的运行时消失，更不能因此切换到旧的发送实现。
- 缓存只保留已审核资源名称；实际导出、模块缓存、AppScope、账号和会话仍须校验。
  更换文档会清空证据，混合运行时版本会被拒绝，不复用旧账号的运行状态。
- 隔离群任务发送后读取已接受请求的绑定，不依赖首页输入框是否重建。
- MCP 可返回任务级 `runtime_diagnostic` 固定状态码和计数；字段允许为空，不承载
  正文、令牌或请求头，也不是送达证明。回群仍以实际群消息和来源回读为准。
