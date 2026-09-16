# 原文预览回填验收记录

对应需求：[social-link-readback.md](social-link-readback.md)。

## 实现

- APK 和 Windows 共用 `social_link_read_adapter.js`：公众号标题、币安文章或短帖摘要、X 精确原帖和文章、真实配图；同一 ID 才回填，过滤推荐/引用、登录和加载壳、头像和站点 Logo。
- 币安 app 分享 URL 的阅读入口解析为相同 ID 的公开 Square 网页，保留分享查询参数和原消息；X 默认打开原文，保留官方嵌入入口。
- 最多 12 次有界延迟读取，退出阅读器前补读；主页面回填即时生效，晚返回的旧 API 不覆盖。
- 本地预览最多 128 条、24 小时，APK 持久存储、Windows localStorage，按服务器和一龙账号/会话隔离，重启后可用；缓存只保留标题/摘要、作者、图片地址和来源 URL，不存全文、不上传。
- Windows 扩展既有主窗口专用状态命令的可选字段，不给外部 WebView IPC 权限，也不增加任意脚本执行接口；旧壳仍支持原文打开。
- PWA 维持官方嵌入与原文回退；不读取跨域标签页，不承诺网页端打开后可直接回填 DOM。

## 验证矩阵

| 能力 | 代码 | 验证 | 当前边界 |
|---|---|---|---|
| 页面适配与 ID 校验 | implemented | offline_verified | 12 个生产适配器浏览器 fixture 用例通过 |
| 桌面回填与持久缓存 | implemented | offline_verified | 真实 React 组件、原文路线、重载恢复、隔离/过期/128 条上限通过；原生 IPC 是测试替身 |
| 既有卡片/菜单/引用 | implemented | offline_verified | 六平台 React/PWA 回归通过，原链接、时间参数、草稿和菜单保留 |
| 桌面原生桥接 | implemented | offline_verified | Cargo check 通过；真实 Tauri WebView 原站读取待现场验收 |
| Android 宿主和缓存 | implemented | offline_verified | 18 项 sociallinks 测试通过；首次新测试 fixture 依赖/应用初始化问题已修正并重跑通过 |
| 原站真实内容 | implemented | deferred | 本环境原站浏览器超时，X oEmbed 连接超时；不能据此认定原文删除或平台禁止打开 |

## 交付

正式发布的版本、来源 SHA 与收尾结果以本次交付的发布回执为准。原站可用性与 DOM 变化属于现场验证边界，注册表暂不标为 fully verified/released。
