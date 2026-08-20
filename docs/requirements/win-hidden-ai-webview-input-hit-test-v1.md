---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-20
---

# Win 后台官方 AI WebView 输入命中隔离 V1

## 用户问题

Google AI / ChatGPT 官方 WebView 收起到后台后，视觉上已回到一龙原生聊天界面，
但鼠标点击输入框或“新对话”仍可能把焦点交给后台官方 WebView，导致原生 UI 无法输入和操作。

## 目标

- 官方 WebView 收起后继续保留同一个 WebView2 Profile、Cookie、页面和会话实例。
- 隐藏态不得参与主窗口的鼠标命中、键盘焦点或可访问性前台交互。
- 用户主动显示官网时恢复原有嵌入区域、可见性和交互能力。
- 原生输入框、“新对话”和其他一龙 UI 在官方页后台时可正常操作。

## 范围

- 仅修改 Windows/Tauri 官方 AI WebView 的嵌入显示与隐藏策略。
- 补充静态合同与 Rust 定向测试，防止隐藏态重新 `show` 后回归为透明命中层。
- 更新 Win 用户浏览器集成说明。
- 不修改 PWA、Android、聊天布局或官方页面 DOM 适配器。

## 验收标准

1. 隐藏官方 WebView 时不重新显示该 WebView。
2. 显示官方页仍按当前嵌入边界恢复并获得焦点。
3. Win 用户浏览器合同、Rust 定向测试和 PC 生产构建通过。
4. 发布后真实桌面点击原生输入框能获得焦点并可发送测试消息。
5. 发送后停留在原生聊天 UI，官方回复继续同步回来。
