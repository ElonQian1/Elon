# Writing Blocks 集中发布与验收

## 当前交付

- 最新正式包 `1.1.1692` / code `1692`，源码 `876513bc9` 已推送并无损安装小米。APK SHA-256：`a92dad5667711b2af7124a6e107ea596ee0756f4cd94338043055cbb931c0798`；线上元数据已独立核对。
- Release 构建/发布通过，日志 `writing-native-final-release-20260913-050436-615`。保存状态修正的 10 项 Android 测试通过；最新结构投影/写回 JavaScript 48 项通过，无失败或跳过。
- `writing-save-native-1692-20260913-051317-276`，42.2 秒：`passed=true`，原生编辑、Markdown 导出和物理字节核对、显式官网保存、返回并重开正文一致、会话无重载全部通过。
- `sent=0`、`cloud_write_attempts=1`、`restored=true`、`awake_restored=true`。复用自有测试样本和生产原生按钮，未清 Cookie/应用数据，未修改代理，导出副本保留。
- `android_chatgpt_text_block_local_editor_export_v1` 已完成；`android_chatgpt_writing_block_save_v1` 的普通独立会话范围已完成并默认启用。项目/临时/库文件/typed-widget 等扩展写回仍不开放，不宣告所有变体完成。

## 已验收代码块基线

- 修正源码 `ee9d97527bb5b914ccbe08cd12c221864c035865` 已推送 `origin/main`，正式版本 `1.1.1688` / code `1688` 已无损安装至小米。
- 该版本 APK SHA-256：`40f733106597a672429685b7331d99488ec220546362c710705d2f8a4be61286`；线上版本元数据当时已独立核对。
- 修正后的 Release Kotlin/Java 编译及 13 项定向测试通过（3 个套件，无失败、错误或跳过）；覆盖块模型、格式、缓存、写回回执和本地操作不被只读核对阻塞的 UI 契约。
- 真机日志 `text-block-native-export-1688-20260913-024333-717`：`passed=true`，明确验收范围 `required_kinds=[code]`。两个真实回复中的代码块均通过编辑副本、导出 `.txt`、SHA-256 核对、恢复原文、重开源内容不变。
- 该次复用已有受控样本，`sent=0`、`cloud_writes=0`、`restored=true`、`awake_restored=true`，最终返回首页。未采集或导出私人会话内容。
- **代码块本机编辑/导出已实测，不因后续 Writing Block 验收重复生成代码块样本。** `android_chatgpt_text_block_local_editor_export_v1` 的写作块变体验收仍待补齐，不把部分覆盖登记为整体 completed。

## 已发布基线

### 1689 写作块连续性

- 源码 `871c04003` 已推送并发布 `1.1.1689` / code `1689`，白名单小米无损更新。APK SHA-256：`b4403a98f1b9cd18ea3e8d6e559b1d7b7e44d2b10b2ea7d3ef74038f32e8ba36`。
- 完整且唯一的同消息、同正文块保留已知写作身份/语言；过期保存票据有界清理，但结果不确定的提交保留只读核对，绝不重放。
- 定向 JavaScript 51 项、Release Kotlin/Java 与 22 项 Android 测试通过。发布日志 `text-block-continuity-release-20260913-032351-484`。
- 真机只读私有清点为 3 条消息、1 个代码块、1 个写作块且可准备写回；没有新增提示或云端写入。这纠正了仅看 DOM 得出的“没有真实写作块”判断。
- 原生 DOM 快照仍将它降为代码块：进一步检查发现外层 `conversation-turn-*` 抢占了子节点的稳定消息 ID，且主动读取受后台预取新鲜度限制。后续批次修正这两处，并取消当前相同路由重复导航；不更换已实测成功的 GET 端点。

- `aaf3973e42a78624d1c2863d1b210d78f0c78cc3`：Release Kotlin/Java 与 1,272 项单元测试通过，243 个套件，无失败或跳过。
- APK `1.1.1687` / code `1687` 已发布并以 `adb install -r` 安装至白名单小米；保留 Cookie、账号及应用数据。
- APK SHA-256：`15c3fe99cb5c933a06405b3ce266d496cd26613df667537531f5f1c36b49edc6`。
- 已核对 APK 内解析器、写回策略、归属核对、私有写回四个脚本与该版本源码字节一致。
- 独立文字 POST 实验开关未开启，未改既有文字发送默认路径。

## 本轮发现

### 1691 保存协议通过与提示回归

- 源码 `0970b58f6`，正式 APK `1.1.1691` / code `1691`，SHA-256 `9856064ca49d9caee9ca4d58e95039c4a842e2f002b8e4af8c42e32b6fc58c85`，已无损安装小米。发布日志 `writing-single-node-release-20260913-043939-562`。
- 生产导航与原生 Writing Block 保留通过，`page_generation=1`；原生编辑、Markdown 导出及物理文件哈希通过。
- `writing-save-native-1691-20260913-045028-104` 的 UI 断言未通过，但随后定点回执核对为 `mcp_2=writing_ready`、`mcp_3=writing_saved`。这证明一次 POST、服务端回读与官网单块状态同步已通过，不能将 UI 文案断言失败误诊为接口失败。
- 根因为原生编辑器的“副本已导出”优先级盖过相同正文的“已保存到官网”；后续补仅在真实保存回执、无 pending、正文未再修改时优先显示保存状态。未重放上述请求，导出稿保留，页面和屏幕常亮设置均已恢复。
- 随后只读观察发现 DOM 仍可能降级写作块身份，补当前已加载官网消息的结构化投影，不在渲染时请求网络、不覆盖生成中消息；私有正文出现写作块时只异步预备一次已核对的共享模块。保存状态修正的 Release 定向检查通过（`writing-save-status-android-20260913-045429-969`），结构投影及同源读写 JavaScript 回归通过（`writing-canonical-ui-js-20260913-050239-413`）。

### 1690 原生写作块与保存证据

- 源码 `149e29cfd` 已推送并发布 `1.1.1690` / code `1690`，小米无损安装。发布日志 `text-block-identity-release-20260913-035609-781`。
- 真实稳定消息 ID 已正确进入原生；直接私有读取可以呈现 Writing Block，原生编辑、导出 `.md` 及物理文件 SHA-256 核对通过，不是普通代码块冒充。
- 自动生产导航验收 `writing-block-native-export-1690-20260913-040758-770` 未通过：当前 URL 正确时仍被 2 秒恢复任务重载，原生入口随后降为 DOM 代码块。已补同 URL 禁止恢复重载，以及实际完整路由快照不等待输入框；content-only 预取仍不能冒充路由到达。
- 对带测试标记的自有样本执行过一次显式云保存，返回 `writing_saved_sync_pending`，说明 POST 后服务器回读已匹配，但整段历史同步未确认。后续只读核对返回 `writing_write_unconfirmed`；没有自动重发或把失败记成成功。本机编辑稿已先导出保留。
- 后续源码改为已核对的官网单节点树更新动作，服务器回读后保留局部编辑冲突保护；确认后异步刷新原生缓存。JavaScript 68 项通过（含官方源码 AST 契约、无跳过）；导航修正的 Release Kotlin/Java 与定向单元测试通过，日志 `text-block-navigation-boundary-android-20260913-042104-791`。

- 生产聊天页的真实输入框可以生成测试样本，原生编辑器可以打开并编辑副本；不是测试 UI 或官网兜底页。
- 初版外部验收脚本错误使用 `part_index + target=content`；已改为 MCP 支持的 `target=message`。
- 官网样本第一个代码块没有语言标记，仅提供 `.txt`，不能固定要求 `.py`。验收现在使用实际支持的格式，随后核对文件 SHA-256。
- 后台只读官网票据核对不应禁用本地编辑、导出和返回；仅实际写回及结果核对期间保护正在提交的内容。
- 编辑器图标改用当前弹窗文字颜色及禁用态透明度，避免浅色弹窗中白色图标不可辨认。
- 外部脚本仅修改带固定测试标记且摘要匹配的本机副本，不触发官网保存。临时测试工具自动移除，测试会话与导出文件保留，不删除账号数据。

## 验收范围

- `scripts/smoke-chatgpt-web-text-block-ui.ps1` 与 `scripts/android/TextBlockUiAcceptance.java` 操作生产原生控件；复用已生成且内容完全匹配的测试会话，支持按 `RequiredKinds` 明确分项范围。
- Writing Block 的完整生产主链已在 1692 单独验收，不把旧的普通代码块通过替代 Writing Block 或官网写回通过。
- 早期失败测试曾停留在导出选项；已显式取消并恢复测试块原文、关闭编辑器。该失败运行的原会话恢复结果为 false，不改写为成功。
- 普通独立会话的官网写回已 `completed / production_verified`；项目、临时会话、库文件联动和不具备明确源身份的变体继续禁止写回。
