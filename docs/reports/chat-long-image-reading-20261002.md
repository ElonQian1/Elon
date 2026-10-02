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

本报告随实现提交；正式版本、装机及线上身份以本轮发布回执和后续追加的验收记录为准。浏览器与 Robolectric 验证不能替代已安装 Win 壳或实体手机运行验证。
