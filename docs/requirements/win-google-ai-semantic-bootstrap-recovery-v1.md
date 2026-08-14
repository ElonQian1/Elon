---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-14
---

# Win Google AI 语义桥启动恢复 V1

## 目标

修复 Windows WebView2 中 Google AI 官方会话已经连接，但一龙原生 UI 无法发送、无法接收回复，
并在页面初始化阶段抛出 `MutationObserver.observe` 非 Node 参数异常的问题。

## 根因边界

- WebView2 initialization script 在 `document.documentElement` 创建前直接运行 Google 适配器。
- Win Google 启动器没有安装 APK 已使用的消息提取器，也没有为当前文档生成稳定令牌。
- 适配器因此可能在发布首个语义快照前直接退出；若继续运行，观察器又会因空 DOM 根节点抛错。
- “账号会话已连接”只表示本地 WebView/Profile 存在，不能冒充语义桥已经可发送。

## 必须实现

- Win Google 启动器必须在每个官方页面文档生成不可预测、格式受限的文档令牌。
- Win 端必须直接复用 Android Google 消息提取器和 Google 页面适配器，不建立第二套 DOM 解析实现。
- 适配器安装必须等待 DOM 根节点可用，且在 `DOMContentLoaded` 前后重复调用都保持幂等。
- 安装完成后必须验证语义桥命令入口存在；失败时只输出脱敏诊断，不暴露 URL 查询、Cookie、正文或令牌。
- 重连路径继续使用同一初始化脚本，并在桥就绪后请求最新快照。
- ChatGPT 现有完整适配器链路、厂商缓存隔离和官方页回退不得回退。

## 验收标准

1. 失败优先合同能够证明旧 Win Google 启动器缺少消息提取器、文档令牌和 DOM 就绪门禁。
2. Rust 定向测试证明生成脚本包含消息提取器、文档令牌、`DOMContentLoaded` 门禁和桥存在性检查。
3. PC 用户浏览器合同测试证明 Google 启动器不会在 DOM 根节点缺失时直接执行页面适配器。
4. PC TypeScript、ESLint、生产构建和 Tauri Rust 定向测试通过。
5. Windows 工件绑定唯一 Git SHA 并发布；真实 Google 账号/地区下的发送和回复保留为用户现场验收。

## 非目标

- 不读取、导出或同步 Google Cookie、OAuth token、网页请求或私有接口。
- 不修改服务器、PWA 或 Android APK 的运行代码。
- 不声称静态合同和本机构建等于真实 Google 页面 DOM 已验收。

## 实现范围

- `desktop-shell/src-tauri/src/local_ai_browser/google_ai_mode.rs`
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs`
- `docs/user-browser-module-integration.md`
