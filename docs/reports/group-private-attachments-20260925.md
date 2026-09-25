# 群聊 AI 私有附件链路

日期：2026-09-25。范围：APK/Win 群消息长按或右键、多选后的 ChatGPT 分析，以及 Win 个人 ChatGPT 附件入口。

## 根因与实现

- `crypto.randomUUID` 兼容修复已在本批基线中，本批不重复实现。
- 原群聊分析只发送文字和附件说明，未传原文件。仅消除 UUID 异常不能让 AI 看见图片。
- 服务端从经成员权限、消息归属及 revision 校验的选区生成版本化附件清单；客户端不能指定任意下载地址。老客户端没有 `attachment_transport_version=1` 时明确拒绝附件分析。
- 每条消息仍按时间顺序保留原文，包括 Markdown、代码、表格与引用文本。文件单独上传，以 `group_01_...` 等唯一名称关联原消息，不把私有下载链接当作图片内容。
- APK 只下载平台附件到任务拥有的缓存目录，经字节数和可用 SHA-256 校验后复用现有原生文件租约及同源私有上传器。
- Win 新增有界 64 KiB 字节桥，复用 APK 的私有文件预留、上传、完成、项目作用域与 composer 关联模块。个人附件按钮也接入该链路，不再以 DOM 文件选择器作为正常路径。
- 两端只有收到 `private_attachment_associated` 才准许发送文字；上传失败、选区修改、账号/会话切换和取消不会降级成纯文字，也不会自动重放未知结果。
- 新客户端要求服务端明确返回附件清单（允许空数组），避免滚动发布期间旧后端忽略新字段后继续纯文字分析。
- 发送与完成入群继续使用现有一次授权、一次发送、完整回答确认和幂等发布流程。

## 边界

- 私有 HTTP 用于附件传输；最终消息提交复用现有发送器及官网运行时状态协调。不是移除 WebView，也不是把 Cookie 导出到服务器。
- 支持 PNG/JPEG/WebP 和常见 PDF、Office、TXT、CSV、JSON、Markdown 文档；每次至多 9 个、每个至多 8 MiB。沿用现有私有图片安全预算（不超过 400 万像素）。不支持的文件显式拒绝，不声称完整支持音视频或任意富媒体。
- 当前批次仅选区分析带原文件；旧的最近 30 条文字上下文自动回复未扩展为自动上传整个群历史附件。不会未经选择批量发送群文件。
- HTML 样式和交互组件不是 ChatGPT 消息输入协议；保留的是文本结构与独立文件，不承诺网页布局逐像素复现。
- 平台附件路由现有访问模型不变。客户端只解析平台相对路径，不转发 ChatGPT 凭证，不访问清单提供的外站 origin，不跟随下载重定向。

## 验证矩阵

| 能力 | 代码 | 本轮证据 | 真实验收 |
|---|---|---|---|
| 选区、权限、revision、文件清单、老客户端拒绝 | implemented | 复用生产 Store 的独立 SQLite harness 24 项通过 | deferred |
| APK 文件下载及上传前置门禁 | implemented | Release Kotlin 编译通过 | deferred |
| Win 字节桥与共用私有上传器 | implemented | 字节完整性、错误摘要、重复分块、导航及私有上传批次定向测试通过；Tauri check 通过 | deferred |
| PC 上传编排与群回复门禁 | implemented | TypeScript/生产构建、定向 ESLint、任务及字节桥联测通过 | deferred |
| 生产后端 | implemented | `validate-rust check --bin elon-server` 通过 | 部署单独记录 |

全量后端 `--tests` 被无关旧测试编译错误阻挡，因此使用已有群聊 harness 验证本批生产模块，未修改其他领域测试。上述假服务/离线证据不能替代真实账号文件上传、视觉问答和原群消息落地验收。

## 验收方法

使用不含隐私的图表图片和带 Markdown/代码的文字消息，多选后提问图中数值并要求引用文字，检查仅一个 AI 气泡回到原群；对不支持类型、超过限制、撤回消息及上传时取消，确认没有纯文字误发或重复回复。无需用私人照片作为测试数据。

## 发布与现场证据

- 业务源提交 `bc9a3f937bf78f3fd2642f6a41003da6f5c02711` 已推送；APK `1.1.1815 (1815)` 已发布并覆盖安装到登记的小米，版本回读通过；荣耀离线。
- 后端 `0.3.1776` 健康与源 SHA 回读通过。PC 页面由独立前端发布入口补发，`PcFrontend` 完成检查通过。Windows 同源版本安装包已构建，远端 outbox 为 `synced`，`NodeAgent` 完成检查通过；这不等于正在运行的每台 Windows 已更新。
- APK SHA-256：`e09b2acec7421203d1a5fac79ef3eb7b2be95dc0d6168aba9d941b6200315e5b`。
- 手机 MCP 的已有个人聊天附件入口使用 `fixed_media_batch_v1`（无隐私 PNG/PDF/TXT）实际尝试一次：仅收到旧网页附件请求回执，未取得 `private_attachment_associated`，最终为 `failed`，会话消息数仍为 0。测试附件及对应草稿已移除，未向群聊发送消息。
- 同一现场登录与输入框就绪，但 `private_send_ready=false`；`fresh_text_admission` 返回 `runtime_unavailable / base_context`。手机 VPN 存在，ChatGPT HTTPS 探测 HTTP 200、659ms。网络探测成功不证明页面全部运行时模块就绪，也不能据此断言官网没有上传能力。
- **现场状态为 failed / needs investigation，不是 completed。** 该尝试验证的是共用上传器的个人入口，不冒充本次群附件下载、上传及原群回复全链路验收。当前仍需定位运行时适配未就绪的具体原因，并补一次真实群图片分析与 Windows 上传验证；不得把离线字节桥测试或发布成功当成此项通过。

## Win 现场复核：输入框依赖缺失

- 9 月 25 日晚经 Win MCP 核对，实际运行的是 `b3945a7486e0e269b46ede1b51d5f1a477e952aa`；通过精确 `update_and_restart` 后回读为 `bc9a3f937bf78f3fd2642f6a41003da6f5c02711`，节点、Tauri 和前端均在线。发布包存在不代表旧进程已激活它。
- 实际新版本的个人 AI 页显示 `TypeError / chatgpt_web_adapter_composer.js`，附件及发送按钮禁用。MCP 进一步确认 `last_error_code=adapter_bootstrap_failed`、`adapter_connected=false`、`composer_ready=false`、`context_ready=false`，页面加载已结束。此处不能报成官网没有功能或单纯网络连接超时。
- 使用真实 Win `ADAPTER_ASSETS` 按顺序执行到 composer，稳定复现第 24 行 `__elonChatGptDictationActions.create` 访问未定义对象。Win 子集漏装了共享输入框已经依赖的听写 actions，以及听写 runtime 和菜单关闭策略。Android manifest 包含它们；本结论不解释前述 APK `base_context` 失败。
- 在离线对照中仅补入三个现有模块，composer 即完成初始化。修复只补 Win 清单并将 adapter 从 209 升至 210，不改共享听写实现，也不绕过上传、身份或恰好一次发送门禁。
- 新增 `scripts/test-chatgpt-win-composer-bootstrap.cjs`：真实清单顺序、无测试注入的初始化、重复注入及旧缺失回归四项通过。此前“Win 清单中的文件属于 Android 清单”的单向检查不能发现漏项；人工注入依赖的 composer 单测也不能证明 Win 装配可运行。
- 真实群聊已经完成图片右键、AI 回复、单图片选区及新版文件披露检查，未勾选继续讨论授权；未提交人物识别问题。完整图片上传、回答和群落地仍待后续实测，不能由上述修复推导成功。

### 后续共用装配与发送状态

Win adapter 212 / `74baa51534ead13b05a47e0ae8391fe5cf6c1e05` 已在本机激活，
真实完整清单初始化通过；普通/群组共享文本模块，身份依赖加载顺序已修复。
网格附件的普通会话实测仍在发送前返回 `runtime_not_observed`，没有接受回执；
问题和快照完整恢复草稿。此证据不证明群图片上传成功，群图片草稿未发送。
完整当前边界见 [Win 网格现场记录](win-grid-chat-20260925.md)。
