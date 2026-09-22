# 群 AI 助手

## 产品边界

用户批准的名称为“群 AI 助手”，内容分为“关注事项”和“更新动态”。它分享群友授权的 AI 任务成果，不监听群聊，也不默认共享个人 ChatGPT 历史。

- 群友主动选择自己的关注事项，预览后绑定到指定群；不按群名称绑定。
- 群成员查看最新已同步结果、来源、分享人和同步时间；私密续聊仍回到自己的 AI 会话。
- ChatGPT 在云端执行任务不等于一龙已经同步。分享者客户端在线且身份有效时才能读取个人任务结果；离线时群里保留已同步版本，不虚报实时更新。
- 不上传 Cookie、运行时令牌或整个个人会话。任务邀请链接用于复制任务，普通会话分享链接也不是持续更新结果的订阅接口。
- 未授权、离群、取消分享、任务已删除和暂时读不到必须分别处理。不得自动重建删除的任务。

官方产品语义参考：[Tasks](https://help.openai.com/en/articles/10291617)、[Shared Links](https://help.openai.com/en/articles/7925741-chatgpt-shared-links-faq)。不硬编码账号任务配额。

## 当前进度

2026-09-23，本模块仍为部分实现，不得标记群内分享功能完成。并行交付的 [群 ChatGPT 账号入口](reports/group-chatgpt-account-entry-20260923.md) 和上一批 [群回复恢复验收](reports/group-ai-send-recovery-20260922.md) 均保留，不受本模块替代。

| 能力 | 代码状态 | 验证状态 | 缺口 |
|---|---|---|---|
| 手机官网任务入口及请求观察 | implemented | device_verified | 已确认页面可访问且读取返回 200；不是新传输验收 |
| 分页任务列表、最新结果只读传输 | implemented | offline_verified | 新传输独立真机验收待完成 |
| 账号隔离、有界缓存、失败保留旧结果 | implemented | offline_verified | 随新传输真机验收 |
| 用户挑选、预览及明确授权 | missing | deferred | 原生选择器和生产命令尚未接入 |
| 群绑定、成员权限及取消分享 | missing | deferred | 后端存储和 API 尚未接入 |
| 结果去重同步和群动态阅读 | missing | deferred | 同步器、富文本快照、APK/Win 展示尚未接入 |

## 已核实的协议

证据来自手机当前页面加载的官方公开静态资源和本机 MCP 结构化请求观察，不是根据接口名称猜测。只实现下列 GET；不创建、运行、暂停或删除任务。

| 请求 | 契约 |
|---|---|
| `/backend-api/automations?filter=scheduled` | 分页对象 `items`、`cursor`；filter 也支持 `paused`、`finished` |
| `/backend-api/automation/{id}/latest_backing_run?include_snapshot=true` | 官方消费者兼容直接结果、`latest_update` 包装及 `null` |

`null` 是没有更新，不是网络错误。`last_backing_run` 是最近一次运行；`latest_update` 可能来自更早的一次有意义更新，不能用最后运行时间作为群消息去重键。

公开资源证据（SHA-256）：

- `conversation-small-f8ygwl0g2zcxhtqg.js`：`fcf2d589e5fad9b5d077561b50fac4fbd0992cda8141417baf3cf9ec272edf59`，请求定义。
- `5d2279e3-k1l8f23nneajbzig.js`：`98e8345b112429abe737da91cffc8dc2ad729102b90c3367dfbfd92e6d0c4210`，分页与分类消费者。
- `594e15ee-of70nfheles0fhzj.js`：`54872fdccbca42b5ca85359674ee406de804f241171ea374c3546948bb20276a`，最新结果、失败和待用户处理消费者。

原始私人响应和运行时请求头不进入 Git。公开脚本临时样本位于忽略目录，不作为协议稳定性承诺。

## 代码与验收入口

- `chatgpt_web_private_tasks_policy.js`：严格解析列表与结果，目录不携带任务 prompt；结果富文本仅保存在本机会话内存。
- `chatgpt_web_private_tasks.js`：同源身份层，7 秒超时、1 MiB 响应上限、60 秒缓存、最多 12 项缓存及 4 个并行读取；单飞、限流、身份切换后旧响应丢弃。
- `chatgpt_web_private_tasks_probe.js`：现有 MCP `chatgpt_private_protocol_probe` 的 `scheduled_tasks` 模式，只回传数量、缓存命中和结果状态，不回传任务标题、ID 或正文。
- `scripts/test-chatgpt-web-private-tasks.js`：协议、账号切换、超时/限流、单飞、缓存和探测隐私定向测试。
- `ChatGptWebScheduledTasksEvidenceTest.kt`：Android 接收端拒绝夹带私人字段的探测回执。

本次模块不等待输入框或 DOM 控件就绪，不修改已有聊天发送、语音和群 AI 回复路径。未接入自动轮询，也未自动向任何群发送内容。

## 后续实施顺序

1. 新传输真机只读验收，分别记录列表、缓存命中和最新结果；探测运行成功不等于结果读取成功。
2. 接入正式命令及原生“选择关注事项”页面，明确展示分享范围并预览单次结果；禁止泛化探测接口作为产品 API。
3. 服务端建立 owner、provider、task、group 的稳定绑定。每次读写重新校验成员身份，分享撤销立即停止后续同步。
4. 按绑定和 `update.id` 幂等导入授权结果；只复制必要的富文本资源，拒绝凭证 URL/私人会话附带字段。取消后不再发布，历史留存规则须在确认页明确。
5. APK 和 Win 共用结果协议，提供关注事项目录、更新动态和结果详情。后台刷新合并请求，使用上次同步时间说明新鲜度；短时故障保留旧内容。

对应领域规则仍见 [私有集成手册](web-ai-private-integration-playbook.md)。在端到端授权、权限和重复同步验收前，不发布“群 AI 助手已可用”的说明。
