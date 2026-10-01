---
version_status: report
reviewed_at: 2026-10-01
---

# 网格分享移动展示与公开金额验收

## 本轮实现

- Win/APK 新分享公开所有已读取的金额和数量；历史隐藏快照继续尊重原边界，需要重新采集发布关联新版本。
- APK 卡片、选择器、详情使用移动 V2 色彩、本地代币图标、实际盈亏、区间位置、触控标签及可选择的完整数值。使用当前生产 View 渲染链路，未替换全局框架或主题。
- 移动 PWA 实际入口是独立 `grid_share.js`，不复用 Win React。已增加展示模块和限定作用域的样式，保留群成员鉴权、只读请求、失效撤回与来源切换取消。
- 同源图标按服务端固定白名单提供，来自 Android 共用的 CC0 资产；没有向外部图标服务发送代币或账户请求。
- 收益指标按已读取的 ROI、策略总盈亏、网格利润依次选择并标明口径。未知不是零；摘要缩短的小数明确标记，详情保留原始精度。

## 验证结果

| 范围 | 证据与状态 |
|---|---|
| Android | `:app:testDebugUnitTest --tests "com.elon.app.grid.share.*" :app:assembleDebug` 通过；10 项测试，包括公开默认值、旧隐藏兼容、精度、排序和浅深主题/字体 UI |
| Win React | 9 项定向测试与 TypeScript/Vite 生产构建通过 |
| 移动 PWA | `scripts/test-grid-share-pwa.cjs` 通过 12 组浅深主题、320/390 宽度、1/1.5/2 倍字体布局；覆盖权限拒绝、重试、跨群、来源变化、图标、金额精度与键盘操作 |
| 既有 AI 分享 | `scripts/test-ai-conversation-share-pwa.cjs` 通过 320/390/1280 宽度与权限、撤回、账号切换、只读及 XSS 回归 |
| PWA 像素 | UI 工作台运行生产移动模块的合成数据夹具，390×844 截图 SHA-256 `488bc999366b8f2179fdd81e36982e3a3f96b8d5319597c42e8b2c3bfe4e2043`，已目视检查 |
| Rust | 格式检查通过；定向图标测试在 Cargo 前被 `RUST_CACHE_CAPACITY_INSUFFICIENT` 阻断，未声称测试通过 |
| Android 原生画面 | 单次有界准备返回 `RENDERER_CAPACITY_UNAVAILABLE`；`REAL_DEVICE_VERIFICATION_DEFERRED`，不能称真机视觉通过 |
| V2 治理 | 自测 12 项与治理检查通过；令牌生成检查发现既有 `orbital_mobile_theme.css` 基线不同步，本任务没有修改全局令牌 |

本报告只证明已执行的本地验证。正式发布、手机安装和新的全公开 QNT 群消息应以发布回执和消息回读为准，不能由合成截图推断。手机原币安子账户保持不变，非空网格采集实测缺口仍保留。
