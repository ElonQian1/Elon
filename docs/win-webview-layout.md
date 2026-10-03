# Windows WebView 布局合同

所有可移动的子 WebView 共用 `desktop-shell/src-tauri/src/webview_layout.rs`。
商店/项目网页和文章使用阅读标签；币安及 AI 官网使用会话宿主。两种宿主的账号、
Profile、导航权限保持独立，只共用布局规则。

## 根因与修复

店铺网页弹出后出现顶部/左侧留白、右侧裁切，来自宿主坐标错用。
锁定的 Wry 0.55.1 Windows 实现中，child WebView 的 `reparent` 修改 HWND 父窗口，
却未更新 `bounds()` 使用的原始 parent；Tauri 2.11.4 runtime 的 `set_size` 先读
`bounds()` 再写回，因此能把相对旧窗口的坐标带回新窗口。

移动到新宿主后，统一用 `set_bounds` 一次写入位置和尺寸，避免读回旧位置。
原生 resize/DPI 事件必须确认 WebView 仍属于事件的宿主；最小化的零尺寸事件忽略。
嵌入布局使用逻辑坐标，独立窗口使用物理客户区尺寸，不能重复乘 DPI。

## 新 WebView 接入

1. 嵌入：经输入校验后调用 `webview_layout::place`。
2. 弹出：完成 reparent 后调用 `fill_host`；页面实例和 Profile 不重建。
3. 窗口 resize/ScaleFactorChanged：调用 `resize_hosted`，传事件源宿主 label。
4. 后台停放：使用共用 `park`；前端布局事件不可改写已弹出的页面。
5. 不在业务模块直接调用 `set_position`、`set_size` 或 `set_bounds`。

## 自动检查与原生回归

`node scripts/check-webview-layout.cjs` 扫描所有桌面 Rust 业务模块。
pre-commit 检查暂存源码；Windows 发布脚本在构建前执行全量检查。
新增窗口若绕过共用模块会失败，不依赖开发者记住历史事故。

原生回归入口是 `desktop-shell/src-tauri/examples/webview_layout_smoke.rs`。
通过 `scripts/validate-rust.ps1 -- run --manifest-path desktop-shell/src-tauri/Cargo.toml
--locked --example webview_layout_smoke` 运行；先设置 `WEBVIEW_LAYOUT_SMOKE_PROFILE`
为任务临时目录中的独立 Profile。脚本仅创建隐藏的空白测试窗口，不使用业务账号。

`--legacy` 重现旧分步写法，预期以偏移失败。正常模式直接读取 Win32 子窗口矩形，
检查三轮弹出/收回、改变尺寸、最大化/还原、停放、旧宿主事件以及四种缩放下的
物理矩形。四种缩放是矩形输入覆盖，不能冒充真实跨显示器 DPI 切换验收。
修改布局模块、升级 Tauri/Wry 时应重跑该原生回归，并记录实际屏幕环境的验收边界。

2026-10-03 本机原生证据：在实际 200% 缩放环境，旧模式第一轮弹出预期
`[0, 0, 1800, 1280]`，实测 `[800, 400, 1800, 1280]`，退出码 1；
共用布局模式三轮共 27 项矩形检查通过，退出码 0。测试仅使用隔离的空白页面，
没有打开或读取币安、商店及 AI 的账号数据。
桌面壳 236 项 Rust 测试、阅读标签模型、交易所与 AI 浏览器合同检查通过。
