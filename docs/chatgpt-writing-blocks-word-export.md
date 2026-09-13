# Writing Blocks Word 副本导出

本批次扩展生产原生编辑器的本机导出，不新增或冒充官网私有导出接口。
既有结构化读取、Markdown / 文本 / 代码源文件导出、显式官网保存保持原链路。

能力 ID：`android_chatgpt_writing_block_docx_export_v1`。
状态：已实现并发布 `1.1.1712`，Android 编译及离线验证通过；桌面 Word 文件互操作通过，手机验收待设备可用。`production_status=published_1712_device_acceptance_pending`，不标记手机验收 completed。

## 用户入口

普通好友聊天的完整写作块 → 打开原生编辑器 → 导出副本 → Word (.docx)。
输出当前编辑内容，而不是旧官网正文。复用下载目录发布、失败清理、打开和分享副本的入口。
不等待 composer、Cookie、网络或官网保存票据，不写回官网，不自动打开外部应用。
代码块仍默认导出对应语言源文件，不将代码保存功能伪装为 Word 或云保存。

## 保真与边界

- 使用项目已依赖的 CommonMark 解析器，不另写 Markdown 正则解析器或添加大型文档库。
- 真实 OOXML ZIP 包含正文、样式和编号关系；不是将 Markdown 文本改扩展名。
- 标题、粗体、斜体、删除线、引用、原生多级列表、表格、行内代码和代码块转换为对应文档结构。
- 代码保留空格、制表符、空行和 Unicode；Markdown 的普通软换行按照段落语义显示为空格。需要原始字节保真时使用已有 `.md` / `.txt` / 源文件导出。
- 链接保留文字和目标地址；图片保留替代文本和目标地址，不下载、嵌入图片或创建外部关系。原始 HTML 仅作为文字，不执行脚本或嵌入对象。
- 空文档可以导出。正文继续限 120,000 UTF-16 字符，结构限 30,000 节点 / 64 层、512 个列表、Word 支持的 9 级列表、32 列表格。超限拒绝导出，不创建截断副本。
- XML 无法表达的控制字符、非法 Unicode 和未知节点失败时保留编辑器内容；不会静默替换或修改原文。
- 不实现 PDF、本地代码执行、邮箱发送或额外官网写回范围。Word 阅读器字体和分页可能不同，不承诺复制官网像素样式。

## 模块

- `WebChatTextBlockDocx.kt`：字符校验、OOXML 包装及文档样式。
- `WebChatTextBlockMarkdown.kt`：CommonMark 到 Word 正文、表格与列表的转换。
- `WebChatTextBlockExport.kt`：复用已有格式选择及字节存储，仅追加 DOCX 编码分支。
- `WebChatTextBlockDocxTest.kt`：生产编码器的包结构、字符、样式、编号、边界和合成互操作样本。

本机 Word 能打开合成文件仅证明文件互操作，不替代手机原生菜单、下载目录和文件分享验收。

## 手机验收入口

`scripts/smoke-chatgpt-web-text-block-ui.ps1` 已增加显式 `-WritingFormat docx`，复用原生产编辑器的语义按钮操作，不新增测试页面或重编 APK。必须指定既有受控写作块和 `-RequiredKinds writing_block`；不创建会话、不发送消息、不写官网。既有样本模式不再需要误导性的 `-CreateFixture` 开关，新建样本仍必须显式提供该开关。

验收先核对原样本身份与正文摘要，要求原生聊天及语音均空闲；仅把本机编辑副本换成仓库合成 Markdown。必须实际选中 Word 格式，不允许缺少选项时改选 TXT。原生回执区分源正文摘要和 ZIP 文件摘要，不能用纯文本摘要冒充 Word 文件一致性。

从手机下载目录只读取本次随机命名的合成导出，核对传输前后完整 SHA-256，再以禁用 DTD/外部解析的有界 ZIP/XML 读取验证实际文件：10 段文字逐字匹配、中文/Unicode、制表符/空行/代码缩进、标题/粗斜体、表格、列表编号及内部关系。随后检查分享选择器并取消，恢复原文、重开确认、恢复原会话与屏幕常亮设置。结果仅包含摘要和计数，不输出正文；校验器只针对该受控样本，不宣称覆盖整个 OOXML 标准或手机阅读器视觉。

工具验证：`writing-docx-acceptance-final-20260914-074258-223` 的 59 项离线检查通过，`writing-docx-native-ui-compile-20260914-074002-231` 的 Java 编译通过，D8 生成并检查 `classes.dex` 通过。首轮新增的损坏编号样本揭示校验缺口，补齐后通过。这些检查验证验收工具，不是重新运行生产导出编码器，也不是手机验收；当前仍因设备离线延期。

```powershell
& ./scripts/smoke-chatgpt-web-text-block-ui.ps1 -DeviceSerial $serial -ExpectedHardwareSerial $hardware `
    -ExistingFixturePath $ownedWritingPath -RequiredKinds writing_block -WritingFormat docx
```

## 2026-09-13 验证

- 源码提交 `9e48db58f`，没有更改 provider adapter 或云保存协议。
- `writing-word-native-verified-20260913-225212-484`：317.6 秒，Release Kotlin / Java 编译及 34 项定向单元测试通过，0 失败 / 错误 / 跳过。其中 DOCX 12、编辑历史 8、文本块模型/原格式导出 9、生产入口接线 5。
- 首轮 `writing-word-native-20260913-224309-158` 编译成功、33 项测试通过，1 项压力用例错误地假设 8,000 组格式节点会超过 30,000 节点上限；修正为实际超限样本后通过。没有修改上限使测试虚假通过。
- `writing-word-interoperability-20260913-225035-976`：17 秒，Microsoft Word 只读打开生产编码器生成的合成文件，识别 1 个表格、3 个列表项、14 个段落、1 页；未启动文档修复，未保存回输入文件。渲染页已检查中文、Unicode、编号、表格和代码缩进，未见截断或重叠。
- 标准 `render_docx.py` 因本机未安装 LibreOffice 失败；实际渲染采用本机 Word 输出检查用 PDF，再用 Poppler 转图。这不代表 APK 已实现 PDF 导出。
- 本轮设备检查为无线 ADB 未连接，未执行手机导出；不重复此前 1706 已验收的 Markdown / 代码源文件流程，也不将其作为新 DOCX 菜单的验收结果。

## 发布

`writing-word-release-20260913-230213-451` 在 528.9 秒内完成正式 APK 构建和发布；版本 `1.1.1712 / 1712`，源码 `8c4f7013748814f0b69b4f9013fb760ff892c5fc`，APK SHA-256 `0fa4ac7c6a37adb89f35ff8823401e77153c18b079fb3add8270b7fd8fd65cea`。
随后独立读取远端 `app/version.json`，版本、源码和摘要一致。发布脚本自动尝试白名单手机安装时无线 ADB 连接超时，最终为 `APK_ADB_DEPLOY_STATUS=verification_deferred`；没有安装成功或新功能真机验收的结论。
