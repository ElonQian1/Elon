# 三端聊天长图阅读

## 实现边界

- Win 社交图片预览改为独立 `SocialImagePreview`，几何运算位于 `socialImageGeometry`。
- PWA 复用 `social_image_viewer.js`，保留历史返回、焦点、二维码菜单及原图下载。
- APK 复用 SSIV 原文件分块解码，由 `ChatImageReadingController` 管理阅读模式，不放大聊天缩略图。
- 高宽比至少 2.5 且适宽后超过可视高度 1.5 倍时自动进入长图阅读；从顶部开始，可主动切回整图。
- PC/PWA 适宽上限 1000 CSS px；手机适配可视宽度。普通照片默认整图。
- 桌面整图模式滚轮缩放；长图模式滚轮上下阅读、Ctrl/Meta+滚轮缩放。保留拖动、双击和缩放按钮。
- APK 保留双指、双击、原始比例、标注、原文件缓存和保存，工具栏不盖住长图首尾。
- 不修改上传、附件协议或群消息；没有向真实群聊发送验收内容。

## 可复跑验证

- `scripts/test-pwa-image-preview.cjs`：Chromium / WebKit 各 22 项；普通图、长图、50000px 超长图、浅深色、320/390/430px、缩放、首尾、旋转、错误重试、关闭和焦点。
- `pc-frontend/scripts/test-social-long-image.cjs`：实际生产 React 组件与本地图片，Chromium / WebKit 各 8 项；自动适宽、滚轮阅读及缩放、首尾、整图、320/390/1024px、焦点返回。
- WebKit 移动环境不支持自动化鼠标滚轮，相关项为 WheelEvent 逻辑验证；手势项为 PointerEvent 逻辑验证，不代表实体 iPhone 手势验收。
- Android `ChatImageReadingGeometryTest` 5 项，`ChatImageReadingControllerTest` 1 项；后者绘制真实 SSIV View 并核对缩放、源坐标和模式切换。
- `ChatImageDiskCacheTest`、`ChatAttachmentExportTest`、`WebChatImageOriginalTest` 定向回归通过。
- PC 生产构建、变更模块 ESLint、移动 V2 治理自测及规范检查通过。
- 测试图片均为本地构造，不保存群聊私人长截图。截图位于任务临时目录，收尾可清理。

## 交付

- 实现提交：Win `70284cc04`、PWA `0aeea05a6`、APK `d1b96ff9e`，均已推送 `origin/main`。
- PWA 静态模板发布通过；源 `d1b96ff9e`，SHA-256 `f1fc7f6a860c50855cca2dd4c065844ca14eb2078f9ec39c4b1e52556dbf864f`。PC `/pc` 前端同源发布通过，兼容现有服务器 `v0.3.1815`。
- APK `1.1.1853 / 1853` 发布并自动安装到注册小米；荣耀离线，未操作。
- 小米通过 MCP 放入本地 `fixed_image_fidelity_v1` 测试附件，外部语义测试通过 open、reading_toggle、pinch、close。1080×6000 真机截图确认 ROW 0 从顶部适宽显示，底部工具不遮挡阅读区。
- 验收后移除测试附件，确认 pending_count=0、fixture_staged=false、upload_started=false；未发送消息、未清除登录态。
- Windows 本地组件首轮因 APK 发布临时版本修改而等待，重试因 C 盘 TEMP 不满足容量安全线被拒绝。D 盘容量预检可用，后续构建仅将本次 TEMP/TMP 放到任务 D 盘临时目录，不降低容量门槛、不删除其他缓存。
- 浏览器与 Robolectric 验证不替代已安装 Win 壳或实体 iPhone 验证；Windows 本地发布及激活状态另附回执。
