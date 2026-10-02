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
| 移动 V2 治理 | 12 项自测和规范检查 | passed，仅治理 |

浏览器使用合成消息和模拟 API，不涉及真实群消息写入。PWA 的 UI 工作台捕获图为 390×844，
图标在左右气泡外侧，实际图片已查看；这是书签组件画面，不冒充完整生产会话验收。
当前截图 SHA-256 `9ee7bb9f329eb8fd8628598cc8b9b9361ec1ad0ccfc3a39f9ec14672d0e67973`。

## 发布与运行

待最终提交后的正式发布、装机身份和 UI 工作台检查补齐。Android 首次工作台准备拒绝脏候选，
将在干净已提交版本上继续；尚无本批原生截图，不声明原生视觉验收通过。
自有 AI/项目页面未新增接入；既有功能边界不变。
