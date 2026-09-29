# PWA 聊天图片预览交付证据

日期：2026-09-29。类型：结果报告，不定义新的产品规范。

## 问题与实现

PWA 的聊天图片此前只有缩略图、下载链接和长按识别二维码，没有绑定点击预览。Android 已有原生图片查看器。本次补齐 PWA 群聊及好友聊天图片查看流程，不改变 APK。

- `social_image_viewer.js` 独立负责全屏对话框、原图加载、1–8 倍缩放、双指缩放、双击放大、拖动和恢复适合屏幕。
- 点击缩略图打开；关闭按钮、Escape 或浏览器返回关闭预览，并恢复聊天滚动位置和焦点。切换聊天、移除来源消息时释放查看器。
- 保留原下载链接和二维码入口；键盘 Enter/Space 打开预览，Shift+F10 打开二维码操作。失败显示重试及原图下载。
- 新组件使用 V2 语义颜色、安全区和至少 48 px 按钮，适配浅/深色、小屏、长图、横屏及字体放大。
- 后端只增加脚本静态路由，页面入口只组装模块；附件 API、数据和授权规则不变。

## 验证

| 检查 | 结果与边界 |
|---|---|
| 图片浏览器测试 | WebKit、Chromium 各 18 场景通过 |
| 布局 | 浅/深色 × 320/390/430 px；横屏长图、字体放大、全屏原图 |
| 操作 | 打开、缩放、拖动、缩放边界、复位、下载属性、焦点、Escape、浏览器返回、聊天位置恢复 |
| 恢复 | 加载失败后重试、删除来源图片、离开聊天 |
| 双指 | Chromium 浏览器触摸注入通过；WebKit PointerEvent 逻辑通过，不等同于 iPhone 实际手势 |
| 附件回归 | WebKit、Chromium 图片、文件、视频、录音、取消及失败恢复通过 |
| 语音回归 | WebKit、Chromium 播放、暂停、切换、重播、下载、失败重试及释放通过 |
| V2 治理 | 12 项自测和规范检查通过；不替代设备验收 |
| 实体 iPhone PWA | 待用户复验 |

语音回归曾在 `audio.ended` 已更新而 `ended` 事件尚未处理时过早断言按钮 ready；改为同时等待媒体结束和界面 ready，保持 10 秒失败上限，无固定延时或放宽断言。语音生产代码未改。

测试入口：`scripts/test-pwa-image-preview.cjs`、`scripts/test-pwa-rich-attachments.cjs`、`scripts/test-pwa-voice-playback.cjs`，分别设置 `BROWSER_ENGINE=webkit/chromium`。全部使用 loopback 合成会话，阻止外网访问，没有上传用户图片或发送生产消息。

## UI 工作台证据与限制

- 任务 `desktop_89ba048de8fb4f458d40cca8eead04a5`，没有新的截图或干净像素目标。
- `ui_capture_pwa_runtime` 捕获真实页面代码与合成群聊的全屏预览，390×844，6 步交互、0 页面异常；浏览器进程与临时目录已清理。
- 源模块 SHA-256：`9bd5d1e543f44ec1b3bf4b3f463c5ba0838ff95ddcbb6de1aaf17055a4ef6481`；routeRevision：`image-fixture-v1`。
- 截图 SHA-256：`f69711424aca6c31fea99915ed8ee8b8bff7c3ab13992ac356d35d792e75e3ce`；另检查浏览器测试的浅/深色运行截图。
- 通用完成门禁要求 Android debug runtime，返回 `PREPARATION_REQUIRED`；本次 PWA 局部修复未启动 Android 构建或装机，不宣称跨端视觉门禁通过。
- `FIT_RUN_STATUS=NOT_RUN; FINAL_VISUAL_LOSS=NOT_MEASURED; VISUAL_ACCEPTANCE_THRESHOLD=NOT_SET; CROSS_PLATFORM_VISUAL_PARITY=VERIFICATION_DEFERRED; BUSINESS_DELIVERY_READY=false; PLATFORM_EVOLUTION_PENDING=false; EVOLUTION_THREAD=none`。
- `REAL_DEVICE_STATUS=NOT_VERIFIED; ANDROID_RENDERER=NOT_USED`。业务发布结果与跨端视觉验收分别报告。

## 发布与复验

先提交推送，再运行 `publish-server.ps1 -SkipPcFrontend` 和 `publish-mobile-pwa-static.ps1`；新增静态路由必须随服务端发布。构建、版本、提交 SHA、线上资源哈希及统一收尾结果以本任务发布收据为准。

用户复验：重新打开 PWA，在群聊点击历史图片，确认全屏、双指/双击放大、拖动及下载；关闭后仍停留原聊天位置。再检查长图、好友聊天和长按二维码。

实现参考：[MDN Pointer Events 缩放手势](https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events/Pinch_zoom_gestures)、[MDN dialog](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/dialog)。
