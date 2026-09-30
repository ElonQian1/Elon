---
version_status: current
reviewed_at: 2026-09-30
implementation_status: verified
---

# Mac 工作台测试包交付记录

## 可交付工件

- 版本：`0.1.6`；GitHub prerelease：`macos-preview-v0.1.6-r1`。
- 构建提交：`2aa17e82342da97838a34fed23696021e0836c94`。
- [下载与安装说明](https://github.com/ElonQian1/Elon/releases/tag/macos-preview-v0.1.6-r1)。
- [通用 DMG](https://github.com/ElonQian1/Elon/releases/download/macos-preview-v0.1.6-r1/elon-macos-preview-0.1.6-universal.dmg)：14,228,202 字节。
- SHA-256：`dd021c17108f8630a974f88a6093192fd9f7844ad9705d048ce7a6262c986369`。
- [最终 CI](https://github.com/ElonQian1/Elon/actions/runs/36687896941)：`success`。
- macOS 14+，一个通用包包含 arm64 与 x86_64；独立应用标识 `com.elon.desktop.macos.preview`。
- ad-hoc 签名，未 Apple Developer ID 签名、未公证；不是正式认证版本。

发布附件包含 DMG、保留资源叉的 `.app.zip`、版本清单、SHA256SUMS、Renderer smoke
回执、安装说明和完整验收范围。公开 DMG 已在 Windows 下载，实际摘要与发布校验文件、
GitHub asset digest 三方一致；这不代表已经在用户 Mac 上安装。

## 实现和验证

| 能力 | 实现 | 验证 | 交付 | 用户验收 |
|---|---|---|---|---|
| 通用桌面壳与 DMG | implemented | CI verified | released | pending |
| WKWebView 页面与原生 IPC | implemented | 真实 macOS loopback fixture passed | released | pending |
| owner/provider Profile 隔离 | implemented | UUID 分离/稳定性回归 passed | released | 真实 Cookie 隔离 pending |
| 系统浏览器跳转 | implemented | Mac 编译 passed、URL 校验回归 passed | released | pending |
| 官网 AI/交易所网页及适配器 | 复用现有实现 | Mac 编译 passed | released for testing | 真实厂商账号 pending |
| CDP 研究宿主、DPAPI 快照、会话正文读取 | 未移植 | 不支持边界保留 | not delivered | 不宣称支持 |
| 本地 Windows 节点安装/升级、自启动 | 不在首包范围 | 自启动菜单禁用 | not delivered | 不宣称支持 |

Windows 桌面完整回归 235 项通过（验证指纹 `d4a58c0…981b2c`）；测试目录修正后的
研究回归 12 项通过（指纹 `556a908…59f2d9`）。Mac 完整日志确认 228 项通过，
[前轮证据](https://github.com/ElonQian1/Elon/actions/runs/36685284363)；最终 CI 再次执行完整测试并成功。
Shell/Node 语法、源码规模、文档模块化及提交门禁通过。

最终 CI 用 `lipo` 核对双架构，`codesign --verify --deep --strict` 核对签名完整性，
检查最低系统/版本，在云 Mac 启动最终应用的真实 WKWebView 并取得 provider 列表 IPC
回执，再以 `hdiutil` 创建和校验只读安装镜像。该 smoke 不连接厂商账号或生产聊天。
CI 主机为 arm64；Intel 二进制经过交叉编译与架构检查，Intel 实机运行待用户验收。

## 发布入口和边界

入口为 `.github/workflows/macos-preview.yml` 和 `scripts/publish-macos-preview.sh`。
Mac 发布入口变更或 main 上手动触发后构建，版本由 Actions run number 分配；测试或验证
失败会阻止 release。Tauri 的装饰性 DMG 脚本在云主机出现失败，正式测试工件改由原生
`hdiutil` 封装；可信 main 的编译缓存会在失败后保存，避免重复冷编译。

独立 DataStore 要求 macOS 14+；阅读页与临时封面页共用阅读 Profile，和主账号/AI
Profile 分离。厂商登录、嵌入/弹出、后台生成、清理会话、重启恢复、语音及附件尚待
用户现场测试。当前沿用已有 HTTP 工作台，未打包本地节点或部署服务器/Windows 更新。

## 用户验收入口

按发布页 TESTING.md 和 [范围合同](../requirements/macos-desktop-preview-v1.md) 协助测试。
记录版本、芯片、macOS 版本、步骤、预期/实际及脱敏截图；不收集密码、Token、Cookie
或私有聊天正文。当前 acceptance_status=pending，收到真实反馈后更新本记录并补针对性修复。
