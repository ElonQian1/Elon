---
version_status: report
reviewed_at: 2026-09-15
---

# APK / PWA 社交聊天恢复验收记录

## 根因及整改边界

Windows 前轮修复使用 PC 独立入口，未自动覆盖 APK 的原生 Kotlin 聊天及移动 PWA 模板。

| 遗漏 | 本轮实现 |
|---|---|
| APK 消息仅内存保存，PWA 会话每次先清空 | 缓存目录与最近 120 条消息，打开先展示再同步；账号、服务器隔离 |
| 待发消息阻断 APK 收消息，切换会话丢发送回执 | 读取独立于待发；回执归回原会话；迟到确认去重，不自动重发 |
| 无读取总超时、重复轮询积压 | 12 秒读取期限，每通道一个请求，合并推送触发，保留一次后续刷新 |
| 恢复前台只等轮询或僵死 WebSocket | APK 立即同步；PWA visibility/pageshow/focus/online 恢复并重建推送，轮询独立兜底 |
| PWA 每次重建消息 DOM | 按 ID 复用消息和媒体节点，保留阅读位置；补齐附件展示及失败下载入口 |
| PWA Service Worker 只有安装壳 | 仅缓存公开 HTML、JS、CSS；API、用户媒体和写入不进入 CacheStorage |
| 旧请求覆盖新会话、账号或编辑版 | 请求代际及会话校验；可见消息和磁盘缓存均保护较新编辑/撤回版本 |
| 网络失败与权限失败混为一谈 | 临时失败保留缓存并重试；权限拒绝清除对应内容；退出删除聊天缓存 |

APK 缓存上限为每账号 60 项、12 MB、单项 2 MB、有效期 7 天。PWA 缓存上限为 40 项、约 180 万字符、单项 100 万字符、有效期 7 天；容量不足不阻断在线收消息。待发只在当前进程保存，服务器仍是权限及内容真源。

缓存不等同于完整聊天备份。语音和图片文件仍从原附件地址读取；离线可阅读已缓存文字，不承诺未下载的媒体可离线播放。

## 模块及验证入口

- APK：`SocialChatSnapshotStore`、`SocialChatReadChannel`、`SocialChatMessageState`、`SocialChatStatus`；群聊、私聊、目录及认证退出接入。
- PWA：`social_chat_cache.js`、`social_chat_recovery.js`、`social_chat_view.js`、`mobile_shell_worker.js`；主模板缩减旧读取编排，新增公开资产路由并支持 runtime 模板内嵌。
- 修正主模板一个既有桥接作用域错误：资产桥接在主闭包内注册，避免页面启动时 `api is not defined`。
- `node --test scripts/test-mobile-social-recovery.cjs`：11 项通过，包含缓存持久化/过期/容量、账号与迟到响应、请求合并、权限、修订/撤回、待发回执、后台恢复、目录部分失败、响应体超时、退出防迟到写入及脚本语法。
- `scripts/test-mobile-social-browser.cjs`：使用完整移动模板、Edge 手机视口和隔离 HTTP 服务，通过真实离线页面重载、恢复联网、后台恢复、媒体 DOM 身份保留、编辑标记、权限清理及退出清缓存；不访问生产账号、不发送生产消息。
- Android 定向单测：9 项通过（缓存/发送状态 4 项、读取通道 3 项、既有群消息修订 2 项），包含旧账号请求结束后同通道可重用、权限拒绝删除缓存及畸形数据防崩溃。
- Rust：通过 `scripts/validate-rust.ps1 -- check --manifest-path server/Cargo.toml --bin elon-server -j 1 --locked`；公开静态资源路由可编译。生产构建与发布单独核对。

## 交付身份

- 源码：`bfe6d8b0640f3cd5774a755b4ebff974df9134f5`，已推送主线。
- APK：`1.1.1766` / code `1766`，正式 release 已发布，来源为上述 SHA；下载入口 HEAD 200，远端大小及 SHA-256 已核对。工件 SHA-256：`0109c9e1565805e22ad72d42517165910b179382035f2da7e63e5dfe89b6cbef`。
- 服务端：`0.3.1758`，线上版本 API 的 Git SHA 与上述来源一致，发布健康检查通过。
- 移动 PWA：独立 runtime 模板发布通过，HTTP runtime 激活检查及远端模板大小/哈希校验通过；模板 SHA-256：`44323c4c0d6906daa4ff8a05dc36fd64654e8db09c8b6a4e789e490df3a08eb5`。
- 过程中的内存不足及中断构建未覆盖旧线上版本；完成环境恢复后，正式发布重试成功。统一收尾与新增桌面右键交互任务一起执行。

真实用户手机、OEM 省电策略和运营商网络尚未实测。上述浏览器验收使用合成群成员、消息和占位品牌素材，不作为真机视觉验收。
