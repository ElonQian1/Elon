# PWA 语音播放修复（2026-09-29）

范围：修复苹果 PWA 聊天语音加载兼容性，并将语音附件改为紧凑播放气泡。Android APK 录音与播放代码保持原样。

## 证据与实现

- Android 录制使用 MPEG-4/AAC、44.1 kHz、64 kbps，输出 M4A；既有 MIME 映射已是 `audio/mp4`。
- 原聊天附件下载忽略 Range，始终读取整文件并返回 200。改为在原路径校验之后使用 tower-http ServeFile，支持 206、416、Content-Range、Content-Length、HEAD 和流式读取。
- Safari 的媒体读取需要字节范围支持，见 [Apple 官方说明](https://developer.apple.com/library/archive/documentation/AppleApplications/Reference/SafariWebContent/CreatingVideoforSafarioniPhone/CreatingVideoforSafarioniPhone.html)。这是已确认的兼容性缺口；尚未取得用户截图对应的原始音频和实体 iPhone，不宣称已在该手机复现根因。
- 新模块 `social_voice_player.js` 在点击事件中直接调用 play，避免异步获取音频后丢失用户激活；支持暂停、重播、单音频互斥、失败重试、后台暂停及移除消息后的资源释放。
- 气泡显示声波与秒数，缺少时长时先显示“语音”；下载移至“···”。加载/播放/错误状态均有可读名称，点击区域不小于 48 px。
- 保留既有附件 URL 同源修正、下载、图片/视频/文件发送、录音与旧脚本回退能力。

## 验证

| 项目 | 结果 | 边界 |
|---|---|---|
| Rust 下载测试 | 7 项通过 | 独立 harness 导入实际生产下载与路径模块，无算法副本 |
| Range | 通过 | 0–1 探测、指定区间、开放末端、后缀、末端截断、416 |
| 下载兼容与安全 | 通过 | GET、HEAD、MIME、长度、完整字节、缺失文件及路径穿越 |
| WebKit + Chromium | 每引擎 10 场景通过 | 实际 Rust 下载 handler + 合成 AAC/M4A 解码播放 |
| 布局与操作 | 通过 | 浅/深色，320/390/430 px，暂停/切换/重播/下载，无横向溢出 |
| 恢复与生命周期 | 通过 | 503 重试、消息移除释放音频、离开聊天停止播放 |
| 附件回归 | WebKit 20、Chromium 21 场景通过 | 图片、文件、视频、录音发送、取消及失败恢复；仅本地数据 |
| V2 治理 | 12 项自测及检查通过 | 不替代设备与视觉验收 |
| iPhone PWA 真机 | 待用户复验 | 桌面 WebKit 不等同于实体 Safari/PWA |

全服务器 Windows 单元测试链接曾因磁盘空间不足失败；随后由项目缓存平台回收合规缓存，改用独立下载 harness 完成定向验证。正式后端构建、发布与线上检查结果由发布收据记录，不能以 harness 代替发布成功。

合成 fixture `scripts/fixtures/voice-tone.m4a` 为浏览器 WebAudio 440 Hz、约 1.8 秒、44.1 kHz 经 MediaRecorder AAC/MP4 编码生成，无用户声音。浏览器测试通过 `ELON_VOICE_FIXTURE_PORT` 连接仅监听 loopback、180 秒自动退出的实际 Rust handler；未设置该变量时使用 Node 网络替身。

命令入口：`scripts/validate-rust.ps1 -- test --manifest-path server/tests/chat-attachment-harness/Cargo.toml -- --nocapture`、`scripts/test-pwa-voice-playback.cjs`、`scripts/test-pwa-rich-attachments.cjs`。后两者用 `BROWSER_ENGINE=webkit/chromium` 分别运行。

## UI 工作台证据与限制

- 导入任务 `desktop_ae28cfd581ed402786868421dfc7cdc3`；图 1 是现状、图 2 是样式参考，无干净像素目标。
- `ui_capture_pwa_runtime` 已捕获真实生产页面代码 + 本地合成会话，390×844；音频模块源码 SHA-256 `bb68058b53f1fee57006163237190b3e9426437624895d8fcc5abe67b3532387`，routeRevision `voice-fixture-v1`。
- 截图 SHA-256 `b20cbd3598d14f0d56bbf5b629723b0d6082e5bd49df59cb1a903b882d22ce29`；4 步执行、0 页面异常、浏览器进程与临时目录已清理。浏览器测试另保留浅/深色 PNG。
- 工作台通用完成门禁要求 Android debug runtime，返回 `PREPARATION_REQUIRED`。本次是 PWA 局部修复，未修改或发布 APK；跨端视觉验收延期，不宣称门禁通过。
- `FIT_RUN_STATUS=NOT_RUN; FINAL_VISUAL_LOSS=NOT_MEASURED; VISUAL_ACCEPTANCE_THRESHOLD=NOT_SET; CROSS_PLATFORM_VISUAL_PARITY=VERIFICATION_DEFERRED; BUSINESS_DELIVERY_READY=false; PLATFORM_EVOLUTION_PENDING=false; EVOLUTION_THREAD=none`。
- 实体设备状态为未验证，`ANDROID_RENDERER=NOT_USED`。发布是否成功与跨端视觉门禁分别报告。

## 发布与用户验收

按仓库合同先推送，再运行 `publish-server.ps1 -SkipPcFrontend` 和 `publish-mobile-pwa-static.ps1`；不得发布未提交源码。最终版本、提交与健康检查以该任务发布收据和收尾输出为准。

用户验收：重新打开 PWA，点击原有 Android 发来的 M4A，确认有声、可暂停和重播；再检查旧语音与新语音。若仍失败，保留原文件及出现时间进一步定位，不要求用户重新录制或丢弃历史消息。
