# ChatGPT Writing Blocks 原生读取、编辑与导出

状态：source-integrated；原生 UI / 文件保存真机验收待完成。不是 Canvas 完成标记。

能力 ID：`android_chatgpt_text_block_local_editor_export_v1`。

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

## 边界

原生编辑目前是**本机副本编辑与导出**，不是“保存回官网”。离开未导出的副本会明确确认丢弃；官网原文保持不变。

本轮只读取与输出文本文件；不增加邮箱发送、AI 改写、代码执行、Canvas 转换或库文件覆盖。
保留 Canvas 原生编辑/版本/导出链路，不把 Writing Blocks 换个标签接到 Canvas 写接口。

生成中的快照和未闭合写作块不允许编辑或导出。块超过 120,000 字符、未知协议版本或类型不匹配时，保留普通消息展示，不创建可编辑的截断文档。
私有 SSE 与历史中的单条原始消息解析上限为 240,000 字符，最多 16 个块；缓存继续服从现有总体大小限制。

## 官网证据

2026-09-12 保留并只解析官方公开 JavaScript，不执行下载的 bundle，不抓取身份令牌。

- `4813494d-gf2h57w5fiay19bd.js`，SHA-256 `6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e`：写作块边界、代码围栏保护、保存正文覆盖、空正文语义。
- `conversation-small-h1dtzoris1y9588z.js`，SHA-256 `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e`：typed writing widget 及其 `data.content`。
- `2f7309c2-gbg2qunp3jjvdsjq.js`，SHA-256 `b72d57a1c3be47a688c914b5e8d6cd0bb3a0928980b4b6b2565e3b5d64a30cc1`：库文件写入使用 `inline_content` 和 `expected_current_version`。这只能证明库文件保存契约，不能推导所有聊天写作块都拥有有效库文件身份。

以上不是账号下的实际写回验收。后续官网写回必须先核对具体块的消息 / 库文件归属与版本，读取后校验，单次提交后确认；不能猜接口、覆盖别的块或把失败自动重发成重复写入。

## 代码与验证

- 解析：`android/app/src/main/assets/chatgpt_web_text_blocks.js`。
- 原生模型：`WebChatTextBlock.kt`；原生编辑与导出：`WebChatTextBlockEditor.kt`、`WebChatTextBlockExport.kt`。
- 定向测试：`scripts/test-chatgpt-web-text-blocks.cjs`、`scripts/test-chatgpt-writing-block-public-evidence.cjs`、`WebChatTextBlockTest.kt`。
- 共享回归：既有历史投影、私有 stream / delta / fetch、快照缓存和原生富内容策略测试。
- 2026-09-12：JavaScript 回归 73 项通过（包含保留的官方源码契约检查，无跳过）；Android Debug Kotlin/Java 编译与 14 项定向单元测试通过。未打包发布 APK、未执行真机视觉/文件导出验收。
- 首次 Android 检查被 180 秒日志静默保护中止，没有源码错误；确认是在 Kotlin 编译阶段后延长至 600 秒，重跑通过。不把被中止的检查记为通过。

## 下一次验收

一次生产页验收即可：打开已有写作块与代码块，确认正文无截断；编辑副本并导出；核对文件字节和扩展名；返回原会话，确认原文和输入草稿未改变；重开会话检查缓存入口。
无需说话、重新登录、清 Cookie 或重复研究已经完成的语音功能。
官网写回仍是独立未完成项，应继续核对身份和版本契约，不能登记为 completed。
