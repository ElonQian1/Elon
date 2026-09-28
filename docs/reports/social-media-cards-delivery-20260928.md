# 跨平台媒体卡片实施记录

## 本轮范围

- 原因：上一轮只有方案。已有大封面布局只对视频号启用，B站、抖音、小红书仍走 56px 缩略图普通链接卡。
- `social_links.js/css` 负责 Win/PWA 的统一媒体外观；APK 复用 `SocialLinkCardView`，不新增独立播放器或网页抓取引擎。
- 卡片显示真实封面、两行标题、作者/平台与入口。横竖比例取实际图片，范围限制为 2:3 至 16:9，不裁掉封面内容。无图/图片失败时仅占 100px 媒体区，不出现破图和大片空白。
- 小红书仍是笔记，不假定都是视频；其入口使用查看标记。普通首页、仿冒域名不升级为媒体卡。
- 视频号继续使用原有 3:4 海报、作者头像与直接微信打开；保留“查看原网页”。没有修改其协议和后台播放器。
- 保留原消息、带参链接、B站时间位置、长按/右键/多选和 AI 回复。
- 预览层过滤小红书登录/首页元数据；抖音 iframe 元数据检查业务错误码。已有标题不能阻止同身份校验通过的成员封面回填；不扩展回填白名单。

## 证据与状态

| 项目 | 状态 |
| --- | --- |
| JS 卡片合同/点击/失效/图片失败 | 通过 `node scripts/test-social-link-cards.mjs` |
| APK 卡片交互与 URL 合同 | `SocialLinkCardInteractionTest`、`SocialLinkPolicyTest` 通过 |
| Rust 生产源码预览 harness | `scripts/validate-rust.ps1 -- test --manifest-path server/tests/social-link-preview-harness/Cargo.toml` 通过 |
| 真实公共封面浏览器布局 | 待发布后运行 `test-social-media-card-render.mjs` 与现有 Channels render 检查 |
| 发布与设备验收 | 待补本批正式发布回执 |

## 未完成边界

- 抖音群样本的公开 iframe 接口返回标题，但不提供封面。检查其公开播放器 JS 后确认封面来自独立 detail 请求；该请求在本机匿名访问被拒绝。本轮没有绕过验证、伪造封面或宣称已取得封面。
- 小红书群样本受访问参数/登录影响。成功获取真实图片时会显示大封面，拿不到时仍保留笔记卡片。没有宣称该样本封面已验收。
- 本轮不宣称 B站/抖音的实际声音/首帧播放、Win 视频号精确直达、新平台原生 handoff、YouTube/TikTok/Vimeo 已完成；相关策略继续按方案推进。
- PC 只修改共享卡片渲染器，不增加 `pc_page` 遗留入口。Win 通过 PC 前端资源发布更新，无需更换桌面壳二进制；已打开的旧页面需重载资源后生效。
