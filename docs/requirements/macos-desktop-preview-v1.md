---
version_status: current
reviewed_at: 2026-09-30
implementation_status: in_progress
---

# macOS 工作台测试包 V1

## 目标与范围

用户授权在无本地 Mac 的情况下，通过 GitHub Actions 构建并发布给 Mac 用户协助验收。
首批为独立标识的桌面工作台测试版，复用生产 `/pc` 和现有 Tauri 壳，支持
macOS 14+、Apple Silicon 与 Intel；不打包本地 Windows 节点或后台服务。

新增入口：`.github/workflows/macos-preview.yml`、`scripts/publish-macos-preview.sh`。
流水线只对 Mac 发布入口变更或手动触发运行，不把所有 Windows 提交变成 Mac 发布。
版本由 Actions run number 分配，工件绑定完整 Git SHA，并提供 SHA-256 校验值。
只有测试、双架构构建、签名完整性检查和 DMG 检查通过才发布 GitHub prerelease。

## 平台边界

| 能力 | 首批范围 | 验收状态 |
|---|---|---|
| 安装、启动、工作台页面、菜单栏托盘、窗口、快捷键 | 构建及交接 | 真实用户待验收 |
| 项目、频道、服务端聊天 | 复用生产页面 | 真实账号待验收 |
| 官网 WebView、JS 适配器、嵌入与弹出 | 保留实现供测试 | WebKit、厂商账号待验收 |
| AI/交易所账号 Profile | macOS 14+ 独立 WKWebsiteDataStore | 账号/厂商隔离待现场验证 |
| 系统浏览器跳转 | macOS `/usr/bin/open`，保留 URL 校验 | 现场待验收 |
| 阅读页与临时封面页 Profile | 共用独立阅读 DataStore，与主账号/AI 隔离 | 现场待验收 |
| Windows DPAPI 快照及会话正文读取 | 未移植 | 不宣称支持 |
| 研究 MCP 的 WebView2/CDP 宿主 | 未移植，继续显式拒绝 | 不宣称支持 |
| 自启动、Windows 节点安装/升级/进程接管 | 不在此包范围，自启动菜单禁用 | 不宣称支持 |
| 语音、附件、OAuth、后台持续生成、清理会话 | 保留现有路径供人工测试 | 不宣称通过 |

不改生产服务器、不更新 Windows 发布版本、不导出厂商 Cookie 或凭据。
测试版使用独立 bundle identifier，不能视为 Windows 会话迁移工具。

## 签名与网络

无 Apple Developer ID 证书时使用 ad-hoc 签名，未公证；必须在发布说明中明确。
不关闭 Gatekeeper。测试者确认来源及摘要后，按 macOS 官方“仍要打开”流程尝试启动；
系统拒绝时记录错误，不能把启动失败当成验收通过。
当前工作台为 HTTP IP 地址，沿用现有生产入口；Mac Info.plist 仅为 WebView 内容
提供 ATS 兼容例外。这不代表传输已加密，HTTPS 迁移另行交付。

## 验收标准

1. CI 在 macOS 上运行桌面 Rust 回归，构建包含 arm64/x86_64 的通用 `.app` 和 `.dmg`。
2. 签名完整性与 DMG 镜像检查通过；发布含版本、SHA、架构、最低系统及校验文件。
3. 不同 owner/provider 使用不同 DataStore，相同 Profile 标识稳定；阅读页独立隔离。
4. 用户安装后能打开工作台、切换页面、关闭后从菜单栏恢复并退出。
5. 用户自行登录并验证官网聊天、重启登录状态、双账号隔离及清理会话。
6. 未适配项、ad-hoc 未公证状态及用户测试结果分别记录，不以构建代替验收。

## 人工验收反馈

请回传：包版本、Mac 芯片、macOS 版本、安装/首次打开结果、工作台登录与导航、
官网登录与发送/回答、嵌入/弹出、后台与重启恢复、账号隔离、清理会话、语音/附件。
失败记录步骤、预期/实际及脱敏截图；不要提供密码、Cookie、Token 或私有聊天正文。
