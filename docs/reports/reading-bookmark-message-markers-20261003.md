---
version_status: current
reviewed_at: 2026-10-03
implementation_status: implemented
owner: conversation-platform
---

# 消息气泡旁书签标记交付

需求：[私人书签图标](../requirements/reading-bookmark-message-markers.md)。
在上一批群聊/好友三端书签上补充消息旁的 🔖，数据库和消息协议不变。

## 行为

- 原始书签始终跟随稳定消息 ID；同一消息多个书签合并图标，读屏说明保留名称。
- 当前书签继续阅读时，在实际目标旁另标「续读位置」；固定原始标记不移动。
- 长气泡从中间恢复时，标记跟随可见区域。图标位于正文之外，不进入复制、引用或 AI 输入。
- 添加、删除、撤销及消息列表复用会刷新图标；关闭会话清理标记，最新模式结束续读提示。
- APK 用薄适配器接线及独立消息标记组件；PWA 和 Win 复用共享书签 UI 与样式。

## 验证

| 项目 | 证据 | 状态 |
| --- | --- | --- |
| Android 实际适配器及回收、原始/续读映射、原有图片长按与分页、进度恢复 | `bookmark-marker-android-20261003-013955-623`，4 类 12 项 | passed |
| PWA 原有多书签、翻页、失败恢复及标记显示 | `bookmark-marker-pwa-regression-20261003-014106-368` | passed |
| 320/390/1280px 左右气泡、长消息、续读、DOM 复用、删除撤销及清理 | `bookmark-marker-layout-final-20261003-014335-307` | passed |
| Win 实际 React 会话创建、跳转及标记几何位置 | `bookmark-marker-win-20261003-014315-577` | passed |
| PC TypeScript 与生产构建 | `bookmark-marker-pc-build-20261003-014404-808` | passed |
| 最终账号隔离修改后的三套浏览器回归 | `bookmark-marker-web-final-20261003-014850-764` | passed |
| 移动 V2 治理 | 12 项自测和规范检查 | passed，仅治理 |

浏览器使用合成消息和模拟 API，不涉及真实群消息写入。PWA 的 UI 工作台捕获图为 390×844，
图标在左右气泡外侧，实际图片已查看；这是书签组件画面，不冒充完整生产会话验收。
当前截图 SHA-256 `9ee7bb9f329eb8fd8628598cc8b9b9361ec1ad0ccfc3a39f9ec14672d0e67973`。

## 发布与运行

发布源提交：`df3214eeb342b31dba60a89aeb1a5063888fd6c5`；共享组件提交为
`b04328ba3f2939dca720397475573e514f48086e`。代码已推送主线，下面三项均由正式脚本发布。

| 平台 | 发布身份与验证 | 日志 |
| --- | --- | --- |
| PWA | 已发布；运行模板 SHA-256 `6ea4ab795a5c4f81cd2ba8b0be72b5c66b2528e77e817d0a0e64b53e075baba3` | `bookmark-marker-pwa-publish-20261003-015038-786` |
| Windows `/pc` | 已发布；前端为上述源提交，兼容现有服务端 0.3.1816；`/pc` HTTP 200、health OK | `bookmark-marker-pc-publish-20261003-015107-162`、`bookmark-marker-pc-release-check-20261003-015919-892` |
| Android | 1.1.1855 / build 1855；远端文件哈希、大小、清单和来源均核验 | `bookmark-marker-apk-publish-20261003-015210-634` |

APK SHA-256：`05608fa30684f190edea35b4f01448b6a61f39587f9f0e52ed15c3f473d6252c`。
发布后按主项目设备清单安装：小米 23116PN5BC 已更新并回读 build 1855；HONOR AAK-AN00 离线，
无在线安装失败。安装收据位于本机 `.elon/apk-adb-receipts/com.elon.app-1855-d6cfc5a8990d4513bdd94a1e88cf249e.json`。
此收据只证明安装与版本，不代表本批原生画面验收。

### 发布后整合检查

发布记录提交合并主线时，另一个任务加入聊天记录图片查看修复。
首次 `finish-ai-task -Kind AndroidFeature` 因这些新 Android 输入尚未发布而拒绝收尾，
本批书签实现仍与已上线源 `df3214e` 一致。2026-10-03 02:18 只读核对的线上版本为
1.1.1856，源 `b2eaac5a2b202ad2553feacaf5757a1c76de023b`，包含书签实现提交。

- 整合 build 1857 已编译成功，但 PowerShell 5 上传前校验出现 `Get-FileHash` 不可用，未上传。
  日志：`bookmark-marker-integrated-apk-publish-20261003-020734-518`。
- 改用 PowerShell 7 重试，排队后被发布 API 以 HTTP 409
  `invalid-stage-transition: terminal release stage cannot transition` 拒绝，未开始重建。
  日志：`bookmark-marker-integrated-apk-retry-20261003-021629-038`。
- 当时同一源 `37c4411bdae06085f7934d5fb3d2596bb2754f99` 的 build 1858 由另一构建节点持有，
  不能将其排队或构建状态当作发布完成。继续以线上身份和统一 finish 实际输出为准。
- 首次收尾输出 `BUSINESS_STATUS=not_checked`、`LOCAL_MAIN_STATUS=not_checked`、
  `TASK_WORKTREE_STATUS=clean`、`FINALIZABLE=false`，原因是整合版本覆盖不足，并非书签功能未上线。
  最终收尾日志另存本机 `.elon/task-finish-evidence/reading-bookmark-markers-20261003-finish.log`；
  未取得最终 `FINALIZABLE=true` 前不得宣称工程收尾完成。

## 视觉验收边界与后续入口

UI 工作台任务：`desktop_84cfa88d52314f39807a4fa37ca48eab`。
干净源版本上的模拟器准备返回 `VERIFICATION_DEFERRED / STOP_AFTER_CLIENT_POLL_BUDGET`，
operation `runtime_prepare_8d94fada63044b359424d03ed553064c`、session `live_189b904db6e24117aff58db621cf90ed`。
没有重启构建或绕过租约操作设备。最终 `ui_check_workflow_completion` 能力门槛通过，
但 `NATIVE_RUNTIME_SOURCE=MISSING_OR_STALE`，跨端视觉证明尚缺。
本批业务代码已发布，原生画面与三端视觉一致性验收延期；功能注册项保留 `implemented`，不冒充全面验收。

| UI 字段 | 结果 |
| --- | --- |
| FIT_RUN_STATUS | NOT_REQUIRED_WITHOUT_CLEAN_TARGET |
| FINAL_VISUAL_LOSS | N/A，未执行目标图拟合 |
| VISUAL_ACCEPTANCE_THRESHOLD | N/A，无目标图数值门槛 |
| CROSS_PLATFORM_VISUAL_PARITY | DEFERRED，工作台原始门槛 MISSING_OR_FAILED |
| BUSINESS_DELIVERY_READY | false，缺原生来源/画面与跨端视觉证明 |
| PLATFORM_EVOLUTION_PENDING | false |
| EVOLUTION_THREAD | N/A |
| ANDROID_RENDERER | EMULATOR |
| REAL_DEVICE_STATUS | NOT_REQUIRED（视觉验收）；发布装机另见上表 |

后续从同一功能项和已发布源提交恢复 UI 验收，重新取得有效模拟器运行来源证明，采集标记消息画面，
与现有 PWA 组件图核对后写入跨端验证；不要因仅缺截图而重复发布。
PWA PNG 与工作台 JSON 已另存本机 `.elon/task-finish-evidence/`，文件名前缀为
`reading-bookmark-markers-20261003`；实际组件截图哈希见上文。没有证明工作台能力缺口，未派发平台演进任务。

自有 AI/项目页面未新增接入；既有功能边界不变。
