# ChatGPT Writing Blocks 原生读取、编辑与导出

状态：原生结构化读取、本机编辑与导出已完成；普通独立会话的显式官网保存已在 `1.1.1692` 通过生产 UI 验收。扩展写回类型仍受下述边界限制，不是 Canvas 完成标记。

2026-09-13 发布与验收：[本轮记录](reports/chatgpt-writing-blocks-release-20260913.md)。

能力 ID：`android_chatgpt_text_block_local_editor_export_v1`，`completed / production_verified`，生产入口默认启用，代码块与真实 Writing Block 均已实测，不重复研究。

## 生产入口

在普通好友聊天的写作块或代码块中打开正文，使用原生编辑器的编辑、复制、恢复和导出按钮；符合归属条件的写作块另有显式“保存到官网”。`ChatGptSocialImageContentController.open` 的 `textBlock` 分支直接连接 `WebChatTextBlockEditor`，不经过旧测试页或官网编辑页面。

2026-09-13 按主线 `e59433b7e` 复核：上述实现和 1688/1692 的验收范围仍在，后续 1699 的会话状态修复没有移除它们。此次复核为代码及既有验收证据检查，不是一次新的导出或云保存测试。本地编辑/导出不等待只读官网票据准备；只有实际云保存及其结果核对期间限制修改。不能将项目等变体的写回缺口误记为整项功能尚未实现。

## 本轮范围

- 普通好友聊天“一龙 AI → ChatGPT”中，完整写作块和代码块提供原生正文入口。
- 历史私有响应与现有 SSE 投影共用 `chatgpt_web_text_blocks.js`；兼容既有 DOM 代码块。
- 写作块读取 `:::writing{...}` 与 `metadata.writing_blocks[id].content`，后者可以覆盖旧正文，包括空字符串。
- 另支持已核对的 `client_defined_widget / writing_block / data.content` 结构；不会把普通 Markdown 或未知 widget 叫作 Writing Block。
- 原生编辑器直接打开已读取内容，不打开官网，不等 composer，不切会话、不发消息。
- 提供编辑副本、复制、恢复原文、文件名选择和导出。退出未导出的编辑需要确认。
- 写作块导出真实 UTF-8 Markdown / 文本；代码复用现有语言扩展名表，未知语言导出 `.txt`。不伪造 PDF / DOCX。
- 复用现有文件字节存储、pending 发布和失败清理，导出不会覆盖已有文件，也不执行代码。
- 已解析块随原有快照缓存保存；保留正文空格、缩进、空行及换行，不使用显示标签代替正文。

快照切换只在同一条已完成消息内，对全文逐字一致且唯一匹配的 DOM 代码块补回已知写作块身份或语言。缺失、变更、重复、生成中或不同消息的块不沿用，不能仅凭周围正文相同认定块未变。
官网保存票据最多保留 16 个；过期的未提交项会清理，容量满时淘汰最旧的未提交项。结果不确定的写入不淘汰、不重放，过期后仍可在原身份和上下文下只读核对结果。

`chatgpt_private_protocol_probe(mode=text_block_inventory)` 仅对当前普通会话执行一次有界 GET，复用正文解析器报告块数量，不输出正文、ID 或请求头。v2 增加 DOM 身份、完整正文及行尾差异匹配数量；这些是诊断计数，不放宽写回校验。它不刷新页面、不发送消息，也不把读取失败报成官网没有能力；统计仅覆盖选中分支最近 80 条可见消息。

DOM 外层气泡只作为容器，优先采用其同角色、非临时且唯一的消息子节点 ID，不猜测歧义归属。手动打开会话复用主动账号读取策略，不再等待后台的近期官网成功记录；仍保留身份及限流冷却。同路径且没有查询/片段差异时只收起官网侧栏，不重新导航。

普通会话的已完成写作块还复用当前官网内存里的结构化消息：限定已核对版本、有效身份、唯一会话和相同消息 ID，不等待 composer，不在渲染中请求或导入模块，不覆盖生成中消息。首次私有读取发现写作块时按 document token 异步预备一次共享模块，不阻塞正文。这样保存后的 DOM 快照不会仅因呈现为 `pre` 就将写作块降成普通代码块。

## 边界

原生编辑默认保留本机副本；符合下节范围的写作块额外提供显式“保存到官网”。打开、编辑、复制、导出本身不会写回，离开未保存的副本会确认丢弃。

不增加邮箱发送、AI 改写、代码执行、Canvas 转换或库文件覆盖。
保留 Canvas 原生编辑/版本/导出链路，不把 Writing Blocks 换个标签接到 Canvas 写接口。

生成中的快照和未闭合写作块不允许编辑或导出。块超过 120,000 字符、未知协议版本或类型不匹配时，保留普通消息展示，不创建可编辑的截断文档。
私有 SSE 与历史中的单条原始消息解析上限为 240,000 字符，最多 16 个块；缓存继续服从现有总体大小限制。

## 官网证据

2026-09-12 保留并只解析官方公开 JavaScript，不执行下载的 bundle，不抓取身份令牌。

- `4813494d-gf2h57w5fiay19bd.js`，SHA-256 `6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e`：写作块边界、代码围栏保护、保存正文覆盖、空正文语义。
- `conversation-small-h1dtzoris1y9588z.js`，SHA-256 `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e`：typed writing widget 及其 `data.content`。
- `2f7309c2-gbg2qunp3jjvdsjq.js`，SHA-256 `b72d57a1c3be47a688c914b5e8d6cd0bb3a0928980b4b6b2565e3b5d64a30cc1`：库文件写入使用 `inline_content` 和 `expected_current_version`。这只能证明库文件保存契约，不能推导所有聊天写作块都拥有有效库文件身份。

以上不是账号下的实际写回验收。后续官网写回必须先核对具体块的消息 / 库文件归属与版本，读取后校验，单次提交后确认；不能猜接口、覆盖别的块或把失败自动重发成重复写入。

## 官网写回源代码批次

2026-09-13 `android_chatgpt_writing_block_save_v1` 的普通独立会话范围已 `completed / production_verified`，随现有官网写入开关默认启用。范围限定具有明确 provider ID、variant、源消息 ID 的完整 `:::writing` 块；项目/临时会话、库文件联动、typed widget、无明确 ID/variant 的块不允许写回，不能把本范围通过扩大为所有变体完成。界面中的源 ID 只是后台核对提示，范围或归属核对不通过时仍可编辑/导出副本。

- 官方 `a965fc59-fzrm5l4zirdbhwph.js`，SHA-256 `752c85e9623229704c208167584c5b7a6e8f18410e6258713d2de7d483a62e19` 的 `nc`：POST `/conversation/message/writing-blocks`，携带 `conversation_id`、`message_id`、字符串 `index`、`id`、`writing_block`、`updated_at`；块内保存 content / index / variant / metadata / title / id。
- 页面同源请求仅由既有身份层提供请求头，数据不离开设备。原生缓存中的源 ID 只是定位提示，不能代替当前页面、账号、分支和服务器正文校验。
- 打开本机编辑器立即展示正文，后台准备保存选择票据；保存前再次读取原消息并核对完整块、元数据及当前官网内存内容。输入框未就绪不阻挡这条读写链路。
- 用户确认后只发送一次；只有回读与提交内容一致，并通过官网单节点写作块更新动作对账后才显示“已保存到官网”。使用已核对的 `shared.sY` 状态事务与 `shared.KJ.updateTree`，只更新源消息内对应块的 metadata，保留其他消息、块和新产生的本地编辑。不再调用整段 `textHydrateHistory`，不 reload WebView，也不直接替换 React store；确认成功后复用私有正文预取刷新原生缓存。
- 单节点动作证据：`2120deb9-jv4295pp9oyi96ww.js`，SHA-256 `87d27849205f36c88e3364ca5557290ce59f78766bb623a058760754b2adcdc5`，`b → Jr → Gr`。当前版本映射限定 `web_20260912`，未知版本不猜符号。
- 超时/连接失败/服务端不确定结果保留 pending，后续只读核对；HTTP 已知拒绝单独处理。官网写入成功但页面同步失败单独标记，不能把重新保存当恢复。
- `updated_at` 是官网客户端提供的更新时间，**不是服务端 compare-and-swap 版本条件**。前后核对能检测已观察的冲突，但不能承诺跨设备同时写入绝无竞态。库文件乐观锁协议不得混入此接口。
- 新增模块：`chatgpt_web_writing_block_policy.js`、`chatgpt_web_writing_block_context.js`、`chatgpt_web_private_writing_blocks.js`；原生命令/回执 `ChatGptWebWritingBlock.kt`，编辑器协调 `WebChatTextBlockCloudSession.kt`。回执不包含正文或凭证。

`1.1.1692` 已完成一次受控写作块的生产原生修改、Markdown 导出与字节核对、显式官网保存、返回与重开正文一致、不重载及状态恢复验收；发送新消息为 0，云写入尝试为 1。跨设备并发保存仍只有上述限制声明，不扩大验收范围。

## 代码与验证

- 解析：`android/app/src/main/assets/chatgpt_web_text_blocks.js`。
- 原生模型：`WebChatTextBlock.kt`；原生编辑与导出：`WebChatTextBlockEditor.kt`、`WebChatTextBlockExport.kt`。
- 定向测试：`scripts/test-chatgpt-web-text-blocks.cjs`、`scripts/test-chatgpt-writing-block-public-evidence.cjs`、`WebChatTextBlockTest.kt`。
- 官网写回测试：`scripts/test-chatgpt-web-writing-block-save.cjs`、`ChatGptWebWritingBlockTest.kt`；2026-09-13 JavaScript 合并回归 87 项通过、无跳过，覆盖请求契约、消息归属、重复写入、未知结果、回读及官网本地编辑冲突。
- 2026-09-13 源码批次的 Android Debug Kotlin/Java 编译及 21 项定向单元测试通过；修改后重新验证，不复用旧源码编译结果。当时未打包，后续集中发布及真机结果见本轮记录。
- 共享回归：既有历史投影、私有 stream / delta / fetch、快照缓存和原生富内容策略测试。
- 2026-09-12：JavaScript 回归 73 项通过（包含保留的官方源码契约检查，无跳过）；Android Debug Kotlin/Java 编译与 14 项定向单元测试通过。未打包发布 APK、未执行真机视觉/文件导出验收。
- 首次 Android 检查被 180 秒日志静默保护中止，没有源码错误；确认是在 Kotlin 编译阶段后延长至 600 秒，重跑通过。不把被中止的检查记为通过。

## 完成与剩余

代码块本机链路在 `1.1.1688` 完成；Writing Block 本机编辑/导出、普通会话官网保存及原生回显在 `1.1.1692` 完成。早期 `1690/1691` 失败与后续修正均保留在发布报告，不能将早期失败改写为通过，也不再重复已通过范围。
剩余为项目/临时会话、库文件联动、typed widget 等扩展写回协议；它们仍没有完整实现/验收。代码块保持本机编辑和文件导出，不执行代码或伪造官网代码保存；PDF/DOCX 导出未实现。
