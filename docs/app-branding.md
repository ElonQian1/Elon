---
version_status: current
reviewed_at: 2026-09-14
---

# 一龙 AI 主应用名称与下载兼容

主项目显示名为 **一龙 AI**，英文品牌为 **ElonAI**，公开安装包名为 `ElonAI-latest.apk`。品牌配置的唯一权威来源是 [`server/src/assets/app_branding.json`](../server/src/assets/app_branding.json)。Android 生成资源与下载路径、Rust 下载服务和两个平台的发布脚本读取此文件；移动 HTML 中的静态名称与地址由 `scripts/test-app-branding.cjs` 检查一致性。

## 用户入口

- Android：[下载最新版](http://43.139.149.158:8080/app/ElonAI-latest.apk)。
- 下载介绍页：[安装与分享](http://43.139.149.158:8080/app/download)。
- 移动网页：[打开一龙 AI](http://43.139.149.158:8080/web)。

Android 正式包名仍为 `com.elon.app`，签名配置保持原样。改名通过正常版本升级交付，不另建应用身份；用户数据仍属于同一个应用。

## 发布和旧版兼容

`/app/ElonAI-latest.apk` 是新公开地址，`/app/ElonSpeed-latest.apk` 长期保留为旧客户端和历史分享链接的别名。两条路由都直接服务同一物理文件，支持 HEAD 和 Range；下载响应的文件名统一为新名字。

物理存储继续使用 `/opt/elon/data/app/ElonSpeed-latest.apk`。这是兼容存储键，不是当前产品名：旧发布脚本还可能对它执行原子替换。不要创建第二份 APK 或用符号链接替换这个存储键，否则旧脚本替换链接后可能导致两个地址的版本分离。

版本接口、更新通知、中继附件和发布元数据都发布新地址。共享品牌 JSON 计入 APK 输入指纹；只改配置也必须重新构建。发布顺序为服务端兼容路由、移动 PWA、正式签名 APK。回滚服务端时也应保留新路由，避免已经发布的新客户端失去下载入口。

## 子项目边界与验证

BB64A 加速器仍使用 `ElonSpeed`；外部 App 注册、协议 ID、历史迁移路径及历史发布记录不因主项目品牌变化而改写。任意用户子项目缺少合法文件名时使用中性 `app-latest.apk`，不套用主应用品牌。

- Rust `server/tests/app-distribution-harness` 直接加载生产路由，验证新旧地址同源、更新后的字节一致、HEAD、Range 与文件不存在时的 404。
- `scripts/test-app-branding.cjs` 检查移动入口和 APK 安装身份。
- `scripts/test-apk-release-freshness.ps1` 验证共享配置变化必须发布新 APK。
- `scripts/test-release-workflow-optimization.ps1` 检查发布仍写入兼容存储键。

本次仅涉及名称和分发路径，不包含真实手机安装与第三方账户操作验收。

2026-09-14 本地验证：生产路由 3 项、品牌契约 2 项、PWA 源码和两项 ESK 网页边界、APK 输入指纹与发布流程测试通过。Android 定向测试 24 项中 23 项通过；唯一失败是原有 `AppUpdateDeliveryContractTest` 对 `bg_update_primary.xml` 颜色的过期断言，测试与背景文件均未被本次修改。工作流扩展检查另受原有 `AGENTS.md` 约 760 token 超过 750 上限阻挡；AI Prompt 资产审计通过。这两项基线问题未在品牌迁移中扩大修改范围。
