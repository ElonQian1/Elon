# APK 聊天图片保真

## 边界

- 聊天列表继续使用小尺寸 Bitmap；全屏查看不调用 `ChatImagePreviewLoader`。
- `ChatImageViewer` 从原文件分块解码，使用固定版本 SSIV AndroidX 3.10.0。
- 支持双指、双击和原始比例；图片标注跟随缩放坐标；关闭或宿主销毁释放解码器和缓存引用。
- 保存原图复制文件字节，不保存屏幕截图或缩略图。Android 10+ 写入相册，旧系统使用文件分享面板。
- ChatGPT 预览若有独立原图回调，仍走该回调，不将预览冒充原图。

## 上传

- 群聊/好友沿用服务端 12 MiB 上限，与 Win 文件上传一致；工作附件保留 8 MiB 上限。
- 限额内保留原始编码字节、尺寸和元数据，不再次 JPEG 压缩，不缩小 PNG 长截图。
- 超限静态图片仍可压缩发送，但标记“已压缩”并显示提示；这是压缩副本，不承诺原图保真。
- 无效或超限失败不留下待发送的半文件。历史上传时已丢失的像素无法恢复。

## 缓存

- 原文件：应用私有 `cacheDir/chat_image_cache`，URL 摘要键；内存缩略图另存，不能互相替代。
- 同源并发合并下载，临时文件完整写入后原子提交；请求 30 秒总超时，单图上限 12 MiB。
- 原文件超过 80 MiB 时按最近使用回收到 64 MiB；查看期间有引用的文件不被回收，释放后继续回收。
- 查看器右上角“图片选项 → 清理图片缓存”清理未使用原文件，不删群消息、相册或待发送附件。
- 再次打开复用原文件，可离线读取已缓存图片；不承诺应用缓存被系统清理后的离线访问。

## 验证入口

- `AttachmentImageFidelityTest`：长 PNG/JPEG 字节一致、12 MiB 社交限额、超限提示、失败清理。
- `ChatImageDiskCacheTest`：冷/热复用、离线读取、并发合并、引用保护、容量回收、较小调用限额不删除原文件。
- 复跑 `ChatAttachmentExportTest`、`WebChatImageOriginalTest`，保留原图导出和官网回调合同。
- MCP `ui_control / stage_chatgpt_web_acceptance_attachment`，fixture_id=`fixed_image_fidelity_v1`。
- 外部 `ImageViewerUiAcceptance` 操作此固定 1080×6000 测试图的 open/original_scale/pinch/cache_menu/close；不发送群消息。
- 运行证据必须分别记录测试、构建、安装和真机结果；未看到像素证据不能仅凭尺寸宣布视觉通过。
