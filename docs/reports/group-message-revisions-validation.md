# 群聊消息修订验证记录

需求：[群聊文字编辑与修改记录](../requirements/group-message-revisions.md)。本记录只说明测试与验收证据，不定义额外产品规则。

## 已执行验证

- 实际 Rust 修订存储模块及迁移的独立测试：9 项通过，覆盖原始文字、附件及发送时间保留、全部历史分页、权限和跨群访问、撤回、幂等、并发冲突、事务失败回滚、追加式历史保护和限流。
- `scripts/validate-rust.ps1 -- check --manifest-path server/Cargo.toml --locked`：通过完整服务端编译检查。
- `pc-frontend` 的 `npm run build`：类型检查与生产构建通过。
- APK `:app:compileDebugKotlin`：通过；`:app:testDebugUnitTest --tests com.elon.app.GroupMessageRevisionTest`：2 项通过。
- 整合并行群成员选择器后，APK 修订测试与 `GroupMentionTest` 再次通过；保留头像长按和原成员选择行为。
- `node scripts/test-group-message-revisions.cjs`：通过 PC/PWA Unicode 差异、延迟刷新保护、撤回保护、待发送消息保留和 URL 编码检查。
- `node scripts/test-group-message-revisions-ui.cjs`：PC/PWA 两组浏览器交互通过，覆盖失败重试、普通聊天草稿保留、完整历史分页、安全显示原文、Escape 关闭、版本冲突显式核对和只读群成员。
- 既有 `test-social-media-downloads.cjs` 与 `test-project-apk-member-download.mjs` 回归通过，覆盖图片、语音、文件、头像来源及项目安装包下载。

浏览器测试使用明确标注的隔离测试数据与接口拦截，不修改线上聊天，不代表真实账户或手机视觉验收。运行浏览器测试需安装 Playwright，或通过进程变量 `PLAYWRIGHT_MODULE_PATH` 指向已有包；默认使用已安装的 Edge。

## 环境与证据边界

Rust 测试第一次为 9/9 通过；迁移编号对齐并行任务后，重跑遇到依赖构建程序 `STATUS_ACCESS_VIOLATION`。缓存诊断确认安装与来源指纹一致，低磁盘空间告警仍在。经管理验证入口关闭 SCCache、使用单编译任务及测试调试信息级别 1 重建后，9 项再次通过；没有清理共享缓存或修改本机全局配置。

独立 Rust 测试夹具直接编译生产存储和迁移代码；它提供连接及时钟，不替代全服务端 HTTP 鉴权或线上业务验收。完整服务端入口另经编译检查。

UI 工作台任务为现有页面扩展，没有新的目标图片。能力检查没有缺失能力；设备清单只有物理手机，没有独立模拟器，因此未准备 Renderer、未安装或操作用户手机。

- `FIT_RUN_STATUS=VERIFICATION_DEFERRED`
- `FINAL_VISUAL_LOSS=not_measured`
- `VISUAL_ACCEPTANCE_THRESHOLD=not_evaluated`
- `CROSS_PLATFORM_VISUAL_PARITY=source_aligned_runtime_deferred`
- `BUSINESS_DELIVERY_READY=false`（工作台尚缺 Renderer 画面证明，源码/构建可按项目规则先发布）
- `PLATFORM_EVOLUTION_PENDING=false`
- `EVOLUTION_THREAD=none`
- `REAL_DEVICE_STATUS=not_requested_not_touched`
- `ANDROID_RENDERER=none_available`

发布版本、提交和仓库收尾结果以发布脚本、统一收尾回执及本任务最终报告为准；此处不提前声明已发布。
