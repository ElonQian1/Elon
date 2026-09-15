# 外部分享与图片原文链接

状态：已接受的产品与协议设计。首次实现：2026-09-16。

## 用户流程

- Android 系统分享中的“发送到一龙”接收文字、网页链接、图片、视频、PDF；最多 6 个附件，单个 8 MB。分享内容先复制到应用私有临时目录，预览后选择并搜索好友或群聊，确认发送。文字遵守现有聊天 4000 字限制。
- 未登录时保留草稿并转到登录；登录成功回到分享预览。旋转和进程重建恢复草稿。账号发生变化时要求重新分享，防止使用旧账号的会话列表。
- 微信内部菜单由微信决定。支持其调用的 Android 系统分享；不能承诺出现在每种专有菜单中。公众号文章可通过浏览器系统分享，或复制链接粘贴到一龙。
- APK 打开外部链接时使用独立阅读页，显示所在域名，可转到浏览器或再次分享至聊天。无原生 JS 桥、文件访问和额外网页权限，不忽略证书错误，不自动打开二维码链接。
- 图片发送端本地识别网页二维码。一个候选可直接附加；多个候选由用户选择，也可不附加。接收端直接显示元数据，不启动批量扫码。旧图或失败图片提供手动识别入口。
- Windows/PC、移动网页镜像显示相同的原文入口；浏览器以新页阅读，避免公众号拒绝 iframe 嵌入。网页上传也提供预览和二维码链接选择。

## 小型扩展协议

现有 `ProjectAttachmentRef` 增加可选字段，无数据库迁移，继续使用聊天记录的 `attachments_json`。转发保留现有图片引用及此字段。旧客户端可忽略新字段。

```json
{"source_link":{"version":1,"url":"https://mp.weixin.qq.com/s/example?scene=90","method":"qr"}}
```

- `method` 为 `qr` 或 `share`；属于客户端提供的来源信息，不是安全认证。
- URL 最多 4096 UTF-8 字节，仅接受完整 HTTP(S) 地址，拒绝凭证、控制字符、反斜线。保留查询参数，不抓取文章、不保存二维码副本。
- 确认是 `mp.weixin.qq.com/s` 或 `/s/…` 时文案为“阅读原文”；其它网页为“打开链接”。各端再次校验后才建立可点击入口。
- 普通链接约增加一两百字节 JSON；最坏随 URL 上限增加约 4 KB，不因群成员人数在消息字段里复制。

## 失败与生命周期

分享确认前可取消。上传失败保留内容。消息 POST 前持久记录 `sending`；结果不明确或进程恢复时进入 `uncertain`，要求去聊天核对，不自动重发。此功能不假装服务器已经提供消息幂等键。

APK 临时草稿按私有目录保存，取消时清除，超过七天的草稿在下次导入时清理。普通聊天图片继续使用既有附件缓存策略。第三方网页需登录或不支持嵌入时保留浏览器入口。

## 模块与验证入口

- APK：`android/app/src/main/kotlin/com/elon/app/sharing/`。
- 服务端：`server/src/project_ws_protocol/attachment_source.rs`；校验后按现有好友、群聊存储路径保存。
- Windows/PC：`pc-frontend/src/features/friends/source-links/`；jsQR 1.4.0 在 Worker 中识别，设置识别超时，不阻塞聊天主线程。
- 移动网页：`server/src/assets/social_source_*.js`；同版本 jsQR 与许可证位于 `assets/vendor/`，不依赖远程 CDN。
- `node scripts/test-social-source-links.cjs`：双端 URL/元数据合同。
- `node scripts/test-social-source-browser.cjs`：实际前端组件、Worker 双二维码、选择链接、合成会话发送、截图。
- `node scripts/test-mobile-social-recovery.cjs`、`test-mobile-social-browser.cjs`：既有移动聊天回归。
- `scripts/test-chat-source-android.ps1`：运行 Android `com.elon.app.sharing.*` 的 URL、草稿恢复、HTML 分享和双二维码测试，并直接检查 JUnit XML，防止 Gradle 启动脚本遗漏失败退出码。
- Rust `attachment` 筛选：来源校验、真实聊天 JSON 往返及既有附件测试。

输入截图仅用作交互参考，不作为像素拟合目标。真实微信系统菜单和具体手机上的交互验收独立记录，不用编译通过代替真机验收。功能注册表工具在本会话不可调用，未手工修改注册表。
