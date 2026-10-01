---
version_status: current
reviewed_at: 2026-10-01
---

# 通用扫一扫 V1 验证记录

需求：[通用扫一扫 V1](../requirements/common-scanner-v1.md)。模块所有者：scan。

| 能力 | 实现 | 验证 | 用户验收 |
| --- | --- | --- | --- |
| APK 相机、选图、手电筒、切换相机、结果操作 | 已实现 | Debug 构建及 7 项 Robolectric 测试通过；双二维码真实解码、图库 Intent、游客拦截、缩略图导航通过 | 物理相机成像待验收 |
| PWA 相机、选图、结果操作 | 已实现 | 26 项共享分类与相机生命周期测试通过；390×844 浏览器真实 worker 识别双二维码 | 摄像头机型兼容待验收 |
| Win 图片右键、预览识别 | 已实现 | 类型检查、构建、lint 通过；实际浏览器右键→识别→双二维码结果通过，复制/下载菜单保留 | Windows 原生宿主待验收 |
| 服务端静态模块路由 | 已实现 | 受管 Rust check 通过；与已有来源链接回归测试兼容 | 随正式发布验证 HTTP 资源 |

识别结果不会自动导航或添加好友；Wi-Fi 与联系人只查看和复制。未登录用户不生成可搜索的设备好友码。
同时修正 PC 既有添加好友请求的 `user_id` 类型为后端接受的 `account_id`。

## 可重复验证

- `node --test shared/scan/*.test.mjs`
- `node scripts/test-social-source-links.cjs`
- `powershell -NoProfile -File scripts/test-mobile-pwa-runtime-template.ps1`：真实运行模板保留 module 语义、经典脚本类型、共享样式、扫码模块内容及按钮入口，5 项检查通过。
- Android `:app:testDebugUnitTest --tests com.elon.app.scan.* --tests com.elon.app.ProfileQrNavigationTest :app:assembleDebug`
- PC `npm run build`、`npm run lint`；开发预览 `pc-frontend/scan-preview.html` 使用合成图片，不访问业务账号。
- PWA 预览 `scripts/fixtures/common-scan.html` 加载生产扫描模块与合成图片。
- `python scripts/check-mobile-design-v2.py --self-test` 及治理检查通过；不将治理检查视为像素验收。

PWA 工具截图 SHA-256：`4acb51cb9b9220c94ab62f3311541d857faf76951c3a3a0df23d63e7a9e40ba7`，识别两个二维码，浏览器异常数为 0。
Win 浏览器实测使用生产 `ImageScanHost`、Vite worker 和共享结果面板；工作台批量回放等待结果超时，另行通过浏览器实际右键核验，不将失败回放记为成功。

## 尚未完成的原生视觉验收

UI 工作台返回 `ANDROID_EMULATOR_NOT_INSTALLED`，归类 `VERIFICATION_DEFERRED`，无需平台升级。
这是新增扫码业务页面，不是系统级主题或框架重建；仓库发布与视觉验收分开记录。

| UI 验收字段 | 结果 |
| --- | --- |
| FIT_RUN_STATUS | VERIFICATION_DEFERRED |
| FINAL_VISUAL_LOSS | NOT_MEASURED |
| VISUAL_ACCEPTANCE_THRESHOLD | NOT_EVALUATED |
| CROSS_PLATFORM_VISUAL_PARITY | SEMANTIC_ALIGNED; NATIVE_VISUAL_DEFERRED |
| BUSINESS_DELIVERY_READY | false（工作台原生运行证据未就绪） |
| PLATFORM_EVOLUTION_PENDING | false |
| EVOLUTION_THREAD | NONE |
| REAL_DEVICE_STATUS | NOT_REQUIRED_FOR_VISUAL_REVIEW |
| ANDROID_RENDERER | UNAVAILABLE |

功能注册表保留 implemented，待物理相机及原生宿主验收后再推进完整验证状态。发布版本和调试手机安装结果以发布脚本收据为准。

## 线上页面模板

服务端 `0.3.1808` 已发布扫码模块，但 `/web` 使用独立的运行时模板，单独验证静态资源返回 200 不足以证明页面启用扫码。运行模板发布器现保留内联脚本的 `type="module"`，并从共享源码嵌入扫码样式；使用 `publish-mobile-pwa-static.ps1` 发布完整模板后，必须同时核对 HTTP 和 HTTPS 实际页面中的模块及按钮。

本次服务端与 PC 产物来自 `1a382955c7979ffba6760c007dd06c3ddabf5766`，包含扫码提交 `3691e1830f727a382b3caebc4b5c55eb00f00c91`。后续发布准备快进到缓存工具提交 `84708bd2df476473b5e83a96471841d92e052aae`；其间服务端与 PC 源码没有变化。通用 PcFrontend 收尾检查要求产物包含整个新 HEAD，因此该严格 SHA 检查未通过，不重复构建相同前端来掩盖这一差异。
