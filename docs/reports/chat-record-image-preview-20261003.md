# 转发聊天记录图片预览修复

## 原因与范围

- 上一轮长图阅读覆盖了普通群消息附件，没有覆盖微信导入记录的独立媒体入口。
- PC/PWA 的 `ElonRecordMedia` 仍生成 `blob:` 加 `target=_blank` 链接，没有调用应用图片查看器；Win 宿主无法依赖该临时链接新窗口完成阅读。
- APK 的 `ChatRecordMedia` 仍使用旧 `ImageView/FIT_CENTER` 弹窗，不支持上一轮长图阅读与分块原图查看。
- 本次仅接通已有查看器；不修改导入文档、附件权限、服务端 API、原始图片或群消息。

## 实现

- 共享媒体组件输出明确的图片按钮，通过 `openImage` 回调连接各端查看器；失败有提示，不再静默打开临时新窗口。
- PC 复用 `SocialImagePreview`，图片预览的 Escape 不传递给父级聊天记录弹窗；关闭后返回图片触发按钮。
- PWA 通过专用 `openBlob` 入口创建、校验同源临时图片；关闭、记录切换、账号变化均释放资源。不放宽普通远端图片入口的协议校验。
- APK 直接将经群权限读取的本地原文件交给 `ChatImageViewer`，保留长图适宽、原始比例、双指缩放和保存能力。

## 验证

- `test-chat-records-reader.cjs`：Chromium/WebKit、1280/390 两种视口，图片点击、适宽、嵌套图片、关闭返回、原位置与焦点、不打开新页、账号变化关闭和 Blob 清理通过。
- `test-record-image-preview.cjs`：真实生产 React `ChatRecordReader/RecordAsset/SocialImagePreview` 同样通过双引擎双视口；鉴权附件请求使用本地替身，无生产写入。
- `test-pwa-image-preview.cjs` 22 项公共查看器回归通过；PC 类型检查、生产构建及变更模块 ESLint 通过。
- Android `ChatRecordImageViewerTest`、`ChatImageReadingControllerTest`、`ChatRecordCardInteractionTest` 通过，验证独立记录入口使用分块查看器、关闭不退出阅读器及已有阅读模式。
- 移动 V2 治理检查与 12 项自测通过，不作为实体设备视觉验收依据。
- 浏览器历史测试使用路由拦截的真实测试源，不以 `about:blank` 的 WebKit 历史行为作为应用返回证据。

## 交付边界

测试使用本地合成图片，未发送群消息。发布、Win 本机激活、APK 装机与设备在线情况，以本轮发布器和统一收尾回执为准；离线或锁屏不得宣称真机验收通过。
