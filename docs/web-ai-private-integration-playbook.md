# 网站私有接口接入与避坑手册

用途：在用户明确授权的范围内，把网站能力接入 APK 生产原生 UI，减少重复研究、状态错误和无效验收。
本手册总结已有证据与后续工作方法，不宣布任何新功能完成，不另建 transport、能力注册表或测试框架。
当前完成范围以[剩余任务的 Current Status Map](web-ai-private-native-remaining-batch.md#current-status-map)为入口；历史报告只证明当时、当版本、当场景的结果。

## 1. 这次为什么拖慢

一个已知的只读接口通常不难；把返回值正确接进现有聊天、文件、语音和会话状态，是另一项工作。
但不能用“协议复杂”解释全部反复。已查到的返工点包括：

- 只测接口或 JavaScript 输出，没有经过真实 Android 接收器，HTTP 成功仍显示空会话。
- 将输入框、身份、目录、网络合并成一个 ready，修好私有读取后仍被无关输入框状态堵住。
- 同一能力经历源码、协议、安装和消费者 UI 多个阶段，只写“完成”会让下一轮误判。
- 历史待验收记录与新完成记录并存，不先读当前状态，就会重做已经通过的范围。
- 测试入口依赖展示文案；官网给标题加了状态字样，脚本就报“功能缺失”，实际只是定位失败。
- 旧接入 Skill 曾只允许 DOM 路线，与当前授权的私有接口路线冲突，已同步修正入口。

这些有代码或记录依据；本次没有逐日工时统计，不能虚构“哪项浪费了几天”。
改进重点是闭合用户操作的整条链路，而不是增加测试次数、报告长度或并行实现数量。

## 2. 先说清楚实际用了哪条通道

| 通道 | 实际含义 | 不能冒充的结果 |
|---|---|---|
| 页面同源私有请求 | 复用 WebView 身份，在页面上下文直接发 HTTP 请求，不点击 DOM | 不是已经移除 WebView |
| 官网运行时命令 | 调用已核实的官网交易回调，由其维护请求及页面状态 | 不是独立 Android HTTP POST |
| 被动协议观察 | 接收官网已经发起的请求或数据流 | 不能据此宣布私有发送完成 |
| 原生媒体 + 页面身份 | Android 持有 WebRTC 音频/数据通道，页面提供官网身份与会话凭证 | 不是把 Cookie 当公开 Realtime API Key |
| 独立原生 HTTP | Android 自己完成身份绑定、发送、流解析及状态对账 | 不能凭复制一次请求就宣布可用 |
| DOM 操作 / 系统替代 | 官网控件操作，或系统听写/TTS 等独立能力 | 不计作官网私有功能完成 |

每项记录真实通道；“不点 DOM”和“完全不需要官网运行时”是不同目标。
默认使用已经验证的官网能力；系统替代由用户明确选择，不静默冒充官网声音或听写。
保留现有官网完整入口，但不能用自动跳转官网来掩盖原生功能未实现。
涉及认证、账户保护和来源限制时保留校验，不绕过登录、反机器人或授权检查。

## 3. 复用既有架构，不再造一套

现有职责顺序：生产 UI 命令 -> 按操作准入 -> provider transport -> 响应归一化 -> 原生状态/UI。
WebView 身份与官网运行时按操作提供依赖；不是每次命令都重新载入整个网页。
复用这些入口，只有出现明确缺口才扩展所属模块：

- [操作准入](chatgpt-operation-readiness.md)：`ChatGptWebOperationReadiness` 与现有 MCP action 共用。
- [请求生命周期](chatgpt-private-request-lifetime.md)：`chatgpt_web_private_json_request.js` 管理完整请求期限。
- [历史接收契约](chatgpt-private-history-native-contract.md)：JS 与 Kotlin 使用同一份合成 fixture。
- [发送交易](chatgpt-same-origin-text-transaction.md)与[官网运行时发送](chatgpt-official-runtime-text-submit.md)：保留现有发送账本与回执。
- [原生语音字幕](chatgpt-realtime-voice-native-transcript-stream.md)：复用已经验证的数据通道和气泡连续性。

### 状态归属

同一个用户不等于多个客户端状态会自动一致。一个操作必须绑定其所属范围：

- provider、账户/工作区 scope，不能把另一账户的缓存当当前数据。
- 会话 ID，发送还包括父消息、请求 ID、模型/工具和附件快照。
- 页面代次/document epoch，旧页面控制句柄和旧回调不能作用于新页面。
- 语音会话及 transcript item/response ID，音频、字幕和文字会话绑定同一 owner。

同一交易只设一个写入者。缓存和 UI 可以先显示，但不得自行重复提交。
区分 `not_dispatched`、`acknowledged`、`reconciled`、`unknown_after_dispatch`。
发出后结果不明时先只读对账；不能自动改走另一通道重发、重复上传或重复开通话。
这些名称描述已有职责，不要求另建同名状态机。

### 准入不是一个总开关

| 操作 | 必需条件 | 不应等待 |
|---|---|---|
| 展开工具/模型菜单、浏览本地缓存 | 预设或已缓存的描述数据 | 每次动态读 DOM |
| 私有目录/历史/文件读取 | 该请求的身份与目标 scope | 输入框 ready |
| 重命名/移动/删除/分享 | 目标归属、权限及对应确认 | 无关输入框状态 |
| 发送/附件提交 | 当前交易上下文及已就绪附件 | 无关目录刷新 |
| 语音开始/停止 | 对应通话、权限、信令和媒体状态 | 把 UI 打开/关闭当连接结果 |

缓存可立即展示，后台更新；缓存不能使过期凭证、文件 URL 或操作句柄继续有效。
区分“未加载/未发现”“已确认不可用”“身份要求”“请求失败”，不要都叫“官网没有功能”。
入口状态先表达准备中，真正执行失败后再给具体错误；不清空已显示的有效会话来表达刷新。

## 4. 协议研究最短闭环

1. **先查现有能力和源码。** 已完成范围直接复用；待验收不等于缺代码。
2. **确定真实产品。** origin、入口、账户或访客模式、当前页面代次；不要把 Google AI Mode 与 Gemini 混为一项。
3. **取得一条受控操作证据。** 用已授权页面/浏览器工具或 APK 现有 MCP 观察实际请求，以及调用它的官网源码。
4. **跟完整链路。** 不只抄 URL，还要确认调用前状态、请求字段来源、响应结构、流结束标志、错误、最终业务关联和原生显示。
5. **固定证据而非易变实现名。** 记录资源路径/摘要、协议版本和导出映射；压缩源码局部函数名不等于 import/export 名。
6. **建立合成契约用例。** 经真实接收器验证，覆盖本次风险边界，再接生产按钮。

浏览器/官方页用于协议研究和对照；最后验收必须来自首页“一龙 AI”的生产 UI。
直接 MCP 调 handler 可以定位问题，但不能代替用户实际按钮、文件选择和返回路径的验收。
浏览器暂时不可用时可分析已保存且有摘要的源码；注明时间/版本，未确认的新行为不能当作当前事实。
不得猜测接口、复用过期校验材料，或把另一产品的公开 SDK 示例直接当网站协议。
观测窗口和响应大小有上限；入库只保留结构、合成数据与证据摘要，不提交 Cookie、请求头、签名 URL 或私人正文。

## 5. 已遇到的坑

下表中的旧报告说明根因，不覆盖当前完成状态；不要因旧报告仍写 pending 就重新研究。

| 坑与已观察结果 | 后续避免方式 | 本项目证据 |
|---|---|---|
| 官网更新资源后，旧导出映射使原生发送回退；观测清单截断又容易被当作模块不存在 | 从实测 anchor 的静态 imports 找角色；复用 `analyze-chatgpt-runtime-contracts.cjs` 做无执行 AST 比对，歧义按真实依赖确认。不能只改文件名或自动接受相似函数；最后核对运行时回执和原生回复 | [1655 兼容修复](reports/chatgpt-runtime-bindings-20260911-b.md) |
| 历史 GET 成功，但 JS 发字符串 `content`，Kotlin 按数组读取，原生消息为空 | producer 与真实 consumer 共用 wire fixture；同时验证部分快照不能清掉输入/语音状态 | [历史契约](chatgpt-private-history-native-contract.md) |
| 文件元数据已返回 200，但官网的 `is_project:null` 被当作非法字段，原生下载仍失败 | fixture 保留真实的缺省、null、false 差异；按官网消费语义校验，不自行收严。分别记录 HTTP、解析、关联和落盘结果，不把协议成功当业务成功 | [1651 引用下载](reports/chatgpt-runtime-bindings-20260911.md#normal-1651-acceptance) |
| 项目会话中的文件引用被强制先查元数据而返回 404 | 文件归属不等于会话项目；追踪当前官网调用者，缺少文件归属/库 ID 时可直接申请会话范围下载授权。保留权限检查，不在失败后去掉 scope 重试。系统下载排队既不算成功也不算失败，要核对本次新增文件字节 | [1657 项目引用下载](reports/chatgpt-project-citation-scope-20260911.md#normal-1657-acceptance) |
| 图库有原始资源 URL，却重新用 asset pointer 申请下载，返回 404 | 区分原始资源、缩略图和文件指针；按官网 URL-download 证据复用既有字节下载模块。确认完整落盘和解码，不拿预览 JPEG 或 queued 回执冒充原图下载成功 | [1654 原图下载](reports/chatgpt-gallery-original-download-20260911.md#normal-1654-acceptance) |
| 未读到 DOM 被记为成功的空图片库，六小时内不再刷新 | unknown 不落成 authoritative empty；只有完整有效响应能确认空结果 | [历史契约中的图库修复](chatgpt-private-history-native-contract.md) |
| 请求在响应头到达就取消超时，body 卡住后一直 busy；超时项目请求迟到覆盖新目录 | deadline 覆盖 body/解析；取消与 settle 有界，晚回调核对 owner/epoch | [生命周期](chatgpt-private-request-lifetime.md) |
| 显式项目成员读取复用了两分钟后台预取新鲜度门槛 | 显式账户读取用自己的准入；后台节流不应拒绝用户的有效操作 | [项目附件后续](reports/chatgpt-project-media-1614.md) |
| 用户重试沿用后台长冷却，刷新又关闭面板取消观察 | 区分手动恢复和自动重试；瞬时网络故障允许短等待后手动重试，保留认证/限流保护。刷新原位更新并复用请求，不只验证底层 HTTP 去重 | [附件列表恢复](reports/chatgpt-file-read-recovery-20260911.md) |
| 只收到上传 reservation 200，没有上传字节；绕过原生选择器的 control 却成功 | 分开证明选择、建文件、传字节、处理完成、消息关联和内容可读 | [上传研究](chatgpt-private-attachment-upload.md) |
| 项目文件目录被当作个人文件库，下载 404；修正 scope 后，客户端单参数限制又误拒绝带两个参数的官网授权地址 | 保留目录、实际文件和项目来源；使用官网返回的地址，校验来源和 ID，保留授权参数，不拿预览 URL 模板限制下载 URL。逐层诊断到原生落盘，不能只看授权 200。1648 已实测保存和 PNG 解码通过，不重复研究 | [项目文件下载](reports/chatgpt-library-folder-download-1644.md) |
| 响应 MIME 为 SSE，实际上传处理 body 是 NDJSON；进度 100 也不等于最终处理完成 | 以实际 framing 和终止事件为准，按 file ID 校验最终回执 | [上传研究](chatgpt-private-attachment-upload.md) |
| 发送已发生，但页面上下文变化导致回执 unknown；盲目 fallback 有重复发送风险 | 账本绑定原请求，确认 dispatch 与清理编辑器分开，未知写结果只读对账 | [运行时发送](chatgpt-official-runtime-text-submit.md) |
| 音频接通不代表有字幕；公开 Realtime 示例的 `oai-events` 不等于网页通道标签 | 使用网页实际 channel、text/binary 帧和 delta/final 协议；字幕按 item/response 去重 | [原生字幕](chatgpt-realtime-voice-native-transcript-stream.md) |
| SSE 被当成逐条完整 JSON，丢掉事件类型与交错通道 | 先按实证解码 marker、channel、继承字段和相对补丁；失败保留最后有效内容，不误报完成。正文与来源补充流复用同一补丁模块 | [增量流协议](chatgpt-private-delta-stream.md) |
| 补充来源的第一批快照被当最终列表；页面重注入后新解析器与旧缓存各持一份状态 | 命令完成前继续原位更新；状态所有者保持单例。来源删除/过滤先于列表限额，账号或文档变化使旧授权失效。代码测试不等于真实来源和下载验收 | [上下文来源集成](chatgpt-private-context-source-files.md) |
| 当前 native 图库入口变为“图片已更新”，旧脚本报找不到功能 | 优先稳定 semantic ID；派生文案有变化时修测试定位，不削弱业务断言 | [1620 图库验收](reports/chatgpt-gallery-thumbnails.md#usb-acceptance-on-1620) |
| 打开缓存预览成功，却没有新网络记录 | 只证明预览可用，不声称验证了独立缩略图传输或流量改善 | [1620 图库验收](reports/chatgpt-gallery-thumbnails.md#usb-acceptance-on-1620) |
| 有界资源清单截断，漏掉全部运行时角色；模块重注入又丢掉诊断注册表 | 保留角色证据且不放大清单；同页面单例诊断与消费者保持同一生命周期，不把“未观察”当“不支持” | [1625 运行时修复](reports/chatgpt-runtime-bindings-20260910.md) |
| 原生测试仍用旧控件标识；新标识中的空格又被 Android shell 拆分 | 从当前生产 port 取得语义标识；跨 ADB 参数编码，在设备端仍校验操作范围，点击前确认弹窗状态 | [1625 模型验收](reports/chatgpt-runtime-bindings-20260910.md#normal-1625-device-result) |

文件下载必须证明实际落盘字节；列表有 URL 不算下载完成。附件已上传不算会话已引用。
UI 已关闭不算媒体已停止，音频完整不算字幕完整；分别检查终止状态与原生最终内容。
新会话 ID 在首次发送后才产生时，要迁移原请求归属，不能把合法身份变化判成另一笔交易。

## 6. 更快的交付与停止返工规则

1. **开工只选真实缺口。** 从当前状态找到 `code_gap` 或 `verification_gap`，写明这批要关闭哪一项。
2. **按完整业务批次实现。** 同一批完成 transport、归一化、状态、生产入口与定向测试；分模块提交，不每个小改动打 APK。
3. **一次统一构建/安装。** 记录 Git SHA、APK SHA-256、实际安装版本和 adapter/transport 版本。测试的包必须包含目标代码。
4. **一次实际生产路径验收。** 检查操作的最终效果、当前 owner、错误恢复和返回原页面；只补新失败处，不扩大无关样本。
5. **按范围关闭。** 已测成功且满足当前功能要求的路径默认启用，记录稳定 `capability_id` 和 completed 范围。
6. **无回归不重做。** 仅上游契约变化、新范围或当前失败证据才重新打开；不得因换一个测试脚本重新实现能力。

“一次验收”不是忽略失败：一个通过样本仅关闭其场景；写操作未知、消息串会话、字节丢失等必须修复。
批量铺代码也不等于积累多套未接入实现；没有生产入口的重复 handler 不作为进度。
每次失败先归类为环境、测试脚本、接口契约、状态协调、原生呈现或确需用户动作。
记录本轮假设、一个能证伪它的检查和结果；同一检查连续失败且没有新证据时，改检查层级，不再原样重跑。
手机不在继续源码和定向测试，标记设备验收延后，不以改代码替代缺失证据。
用户不必参与代理能完成的选择/导航；麦克风、听声音或系统本人确认才安排人工步骤，并明确是哪项测试。

当前任务顺序：完成 ChatGPT 剩余功能并验收 -> Google 同类接入并验收 -> 发热/耗电优化。
不把温度或流量 A/B 当当前功能门槛；也不因功能可用就声称已降温。保留基本资源上限和安全清理。
旧报告保留历史；最新完成范围与下一缺口只在当前状态入口维护，不在每份旧报告复制状态表。

### 最小交接卡

将这些字段补入既有能力文档/注册流程，不新建第二份能力账本：

```text
capability_id: existing_stable_id
scope: provider + account mode + user workflow + supported variants
transport: page_private_http / official_runtime / observer / native_media / native_http
reuse: existing files, commands and state owner
protocol_evidence: source asset digest + controlled action + response/final shape
contract: input, receiver schema, identity scope, operation-specific admission
write_result: dispatch/ack/reconciliation and unknown-result handling
code_status: implemented / partial / missing
verification_status: offline_verified / device_verified / deferred / failed
evidence: focused tests + commit + installed APK digest + production control/result
production_default: true or false, with reason
remaining: exact unproven scope or regression, not a repeated history
```

`completed` 必须有明确 scope 和真实可用证据，不代表全账户、全格式、全协议都通过。
Android 成功构建不是 UI 验收；HTTP 200 不是业务验收；测试内容被回复也不能替代附件内容确实被读取。

## 7. Google 接入前检查表

先复用已有[响应观察研究](google-web-private-response-research.md)、[回复观察器](google-web-private-reply-observer.md)和[目录能力](google-web-private-conversation-directory.md)。
已经核实的 `/async/folif` 与 `AimThreadsService/ListThreads` 是回复信号/目录证据，不是私有发送实现。

- 确认用户使用 Google 搜索 AI Mode 还是 Gemini，实际 origin、账户/访客状态及可用能力。
- 不假定 Google 有 ChatGPT 一样的项目/模型/临时聊天结构；没观察到不能捏造，也不能直接判定不支持。
- 阅读现有 `google_web_private_reply_observer.js`、`google_web_private_response_tap.js`、`google_web_private_thread_directory.js`，标明复用边界。
- 用一条受控发送找到真实 request owner、身份材料来源、会话 ID、响应 framing、增量和最终错误/完成事件。
- 对照官网调用方，确认是完整私有请求、运行时命令还是被动观察，分别登记，不偷换完成定义。
- 先做“原生发送 -> 对应原生气泡流式显示 -> 结束 -> 同会话续聊”这一条完整路径。
- 再补新会话、账户/访客、工具、附件等真实存在的范围；模型和服务端状态由所属 provider 解释。
- 共用已有原生模型/交易账本/有界请求能力，不复制 ChatGPT 的端点、压缩函数名、React 假设或 token 获取方式。
- 验收失败先检查 native consumer、owner 和请求生命周期，不先增加 DOM 轮询、总 ready 或第二套 sender。
- 主动读取与后台预取复用请求实现，但不要共用普通失败冷却；一次空正文不能禁用附件列表或项目归属查询。登录拒绝、限流仍须跨读取类型保护。身份预热的短冷却不能被上层再次放大为长时间失败，见[主动读取回归](reports/chatgpt-explicit-account-read-20260911.md)。

## 8. 调试入口也要稳定

- 优先生产 UI 的稳定语义标识与 APK MCP 状态/回执。MCP 无法解释的问题再用一次有针对性的 UI 检查。
- USB 与无线 ADB 用序列号核对同一台手机；mDNS 旧记录、不同子网、连接失败不等于业务接口坏了。
- USB 已可用就继续验收，不重复寻找失效无线端口；安装后读取实际版本，保留应用数据与登录态。
- 长命令复用日志脚本，已有运行先查结果。PowerShell 中用 `& ./scripts/name.ps1`，不把 `.ps1` 当文件打开。
- 必须另起进程时隐藏控制台并设置超时；不反复开控制台或并行重启同一 Gradle 验证。

这份手册的成功标准是下一网站少重复一次已知错误，而不是多写一份“研究完成”报告。
