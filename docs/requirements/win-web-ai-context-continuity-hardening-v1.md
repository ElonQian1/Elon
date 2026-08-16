---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-16
---

# Win 网页 AI 上下文连续性加固 V1

## 目标

加固一龙 Windows 生产首页在账号切换、客户端重启和超长聊天下的网页 AI 上下文连续性，
同时确保 Google 查询、登录跳转参数和一龙 owner 标识不进入可枚举目录或调试日志。

## 必须实现

- WebView2 Profile 和 DPAPI 快照目录使用 SHA-256 生成的 128 位十六进制 owner 指纹，
  不再把 64 位非密码学散列作为新账号目录。
- 已存在的旧 64 位 owner/provider 目录按厂商迁移到新目录；目标已经存在时不覆盖，
  正在使用导致迁移失败时继续使用旧目录并在后续重试，不能丢失 Cookie 或缓存。
- 完成态语义快照超过 2 MiB 时先淘汰较旧的会话副本，再从当前聊天最旧消息开始裁剪，
  保留最近上下文及 `messageWindowStart/observedMessageCount` 边界。
- 流式回答、输入草稿、命令结果、Cookie、token、请求头和原始响应继续禁止持久化。
- 宿主导航和页面生命周期日志只允许记录 `scheme + host + path`，不得包含 query、fragment、
  userinfo、搜索问题或登录跳转参数。

## 非目标

- 不改变厂商官方网页的登录、Cloudflare、地区或账号开放规则。
- 不读取或发送用户真实会话内容，不调用厂商私有接口。
- 不修改服务器、APK 或 PWA 运行逻辑。
- 不用静态测试替代 ChatGPT/Google AI 真实多轮上下文现场验收。

## 验收标准

1. Rust 测试证明 owner 指纹稳定、跨账号隔离、使用 SHA-256 且旧 Profile 数据可迁移。
2. Rust 测试证明超过持久缓存上限的完成态聊天保留最新消息和正确窗口边界。
3. Rust 测试与 PC 合同证明导航日志不包含 query、fragment 或 userinfo。
4. 既有 DPAPI、上下文边界、厂商切换热缓存和适配器资产对齐回归继续通过。
5. PC TypeScript/Vite、ESLint、用户浏览器合同和 Tauri Rust 定向检查通过。
6. Windows 发布工件绑定唯一 Git SHA；真实消息收发与多轮上下文只在用户授权后验收。

## 实现范围

- `desktop-shell/src-tauri/Cargo.toml`
- `desktop-shell/src-tauri/src/local_ai_browser.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/owner_profile.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/snapshot_cache.rs`
- `desktop-shell/src-tauri/src/local_ai_browser/tests.rs`
- `pc-frontend/scripts/test-local-ai-browser-contract.cjs`
- `docs/user-browser-module-integration.md`
