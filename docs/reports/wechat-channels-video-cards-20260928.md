---
version_status: current
reviewed_at: 2026-09-28
implementation_status: implemented
---

# 视频号卡片的视频识别与作者身份

## 用户问题与实现

此前 APK 和 Win 群聊的视频号卡片只有封面及作者文字，缺少播放标识，看起来像图片。
本批复用现有卡片和微信跳转链路，不重新实现播放器：

- 封面保持 3:4，中央常驻播放标识，左上角显示“视频号”。加载或封面失败时仍能识别视频。
- 封面下方独立作者栏显示真实作者头像、名称和“在微信中观看”；普通浏览器显示“打开视频号”。
- 作者名省略但保留可访问的完整名称；头像失败使用作者首字占位，不使用群发送者头像冒充作者。
- 作者行、播放标识和封面共享卡片原有点击行为；Android 长按和多选行为保留。
- 原网页入口保留，不显示虚构时长、播放进度或统计，不声称封面就是可播放的视频文件。

服务端从公开预览响应 `data.authorInfo.headImgUrl` 获取头像，复用固定公共 DNS、HTTPS、
下载上限及图片解码限制，转换为最长边 64px、最多 12KiB 的 JPEG data URL。
字段 `author_avatar_data_url` 为可空增量字段，旧客户端不受影响。
封面和头像并行读取，共用既有预览时限；头像失败不阻塞卡片，临时微信启动参数不进入缓存。
Win 和 PWA 复用 `social_links.js/css`，APK 使用原生 `SocialLinkCardView`。

## 验证

- 共享 JS 契约通过：预加载视频标识、头像显示/失败、非法头像拒绝、主次入口及旧平台卡片。
- Rust 生产源码 harness：22 项通过，2 项显式网络测试默认忽略。
- Playwright 使用真实公开样本封面与头像，360/1280 宽度通过：图片可解码、3:4、
  作者栏不覆盖封面、无横向溢出、点击与键盘打开、头像失败、长作者名。
- Android 15 项定向测试通过，包含实际触摸命中播放图标/作者/头像、长按、多选和头像清空。
- 生产 Rust 显式网络测试通过：真实封面、64px 作者头像下载及新鲜微信跳转参数。
- 微信跳转共享 JS 的 6 项测试通过。

## 发布回执

- 功能提交：`2780a004110212de354056a1f595865364fc3f4a`，已推送 `origin/main`。
- Server `0.3.1784` 与 PC 前端同 SHA 发布；`/pc/assets/release.json`、群聊懒加载
  JS/CSS、PWA 卡片资源实读确认新播放标识及头像字段。TypeScript/Vite 生产构建通过。
- APK `1.1.1823` / build `1823` 已发布；SHA-256：
  `ed6ae71c09dce471d3172f7f905593bc21a8062270e93e5e5a6f586a6b3cc924`。
- APK 内 177 项官网聊天资源逐项 SHA 校验、manifest 版本校验通过。
- 登记小米自动覆盖安装成功，回读 build `1823`；荣耀离线，未操作未清数据。
- 真机卡片视觉与 Win 外壳当前窗口未重新截图验收；浏览器渲染和 Android 触摸测试
  不能写成两台实机均已验收。视频号播放/Windows 精确定位仍沿用原有能力边界。
- 本机 PowerShell 5.1 发布进程丢失 `Get-FileHash` 函数可见性，切换已安装的
  PowerShell 7 后发布完成；复用已有 Gradle 产物，不跳过哈希或版本校验。

可重复检查：`node scripts/test-social-link-cards.mjs`；安装 Playwright 后运行
`node scripts/test-channels-card-render.mjs`（需要访问明确的公开视频号样本；支持
`PLAYWRIGHT_MODULE` 指定已有工具运行时）。截图仅为共享前端渲染证据，不代替 Win 外壳验收。

## 不包含

本批不改变视频号内嵌播放能力或 Windows 微信定位能力；现有边界见
[视频号接入记录](wechat-channels-handoff-20260928.md)。微信 ZIP 导入分享的双端实机完整验收
是独立事项，不因本次卡片视觉通过而标记完成。
