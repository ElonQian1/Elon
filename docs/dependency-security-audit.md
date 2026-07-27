# 依赖安全审计策略

本文档记录依赖安全审计的当前策略。CI 对 npm 与 RustSec 漏洞使用 `Strict` 模式阻断；RustSec warning 仍先报告不阻断，避免传递依赖的 yanked warning 直接卡住主线。

## 当前范围

- PC frontend：基于 `pc-frontend/package-lock.json` 执行 `npm audit --json`，输出漏洞数量、严重等级和前 10 条风险。
- Rust server：CI 固定安装 `cargo-audit@0.22.2` 后使用 `cargo audit --json` 执行 RustSec 审计；本地未安装 `cargo-audit` 时默认输出 skipped 报告，并用 `cargo metadata --locked` 确认依赖图可解析。

## 当前基线

- PC frontend 已升级到 `vite@8.1.5`、`@vitejs/plugin-react@6.0.4`、`eslint@10.8.0` 与 flat config；`npm audit` 中除受控例外外不得有任何漏洞。
- `react-router@7.18.1` 的 `GHSA-qwww-vcr4-c8h2` 修复边界被 advisory 标记为尚未发布的 `8.3.0`。`.github/dependency-audit-exceptions.json` 仅以精确版本和 source `1124282` 临时豁免该项至 `2026-08-09`；PC frontend 只使用 `BrowserRouter`，不启用 React Server Components、SSR 或服务端 route actions。
- Vite 8 要求 Node `^20.19.0 || >=22.12.0`，`pc-frontend/package.json` 与 CI 均显式固定该运行要求。
- PC frontend lint 基线已升级到 `eslint@10.8.0` flat config，直接声明 `@typescript-eslint/*`、`eslint-plugin-react-hooks`、`eslint-plugin-react-refresh` 和 `globals`，不再依赖旧 `.eslintrc.cjs`。
- Rust 侧 CI 已固定安装 `cargo-audit@0.22.2`，并通过 `-RequireRustAudit` 防止 CI 静默 skipped；漏洞数量已进入 `Strict` 阻断模式。
- `quick-xml` 已升级至 `0.41.0`，修复 `RUSTSEC-2026-0194` 与 `RUSTSEC-2026-0195`。Android XML 资源文本和 launcher 属性解析都已适配新版解码 API，并有资源文本实体回归测试。
- `rsa@0.9.10` 仍受 `RUSTSEC-2023-0071` 影响，且上游尚无可用修复版本。它用于微信支付签名，以及 Windows Desktop review broker 的进程内 RSA-3072 临时密钥。后者只能经受保护 Codex Desktop 祖先进程的本机命名管道调用，Elon executor 祖先进程会被拒绝；密钥不落盘、重启轮换。该例外以编号、包名、精确版本和到期日绑定，必须在 `2026-08-09` 前复查上游补丁或迁移方案。
- RustSec 当前 warning 基线为 0 warnings；原 `spin v0.9.8` yanked warning 已通过升级 `axum@0.8` / `tower@0.5` / `tower-http@0.6` 并刷新 `server/Cargo.lock` 到 `spin v0.9.9` 清理。
- `server/Cargo.lock` 已纳入版本控制，用于保证 RustSec 审计和 CI 构建解析同一套服务端依赖图。

## 执行命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-dependency-audit.ps1
```

只检查 npm：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-dependency-audit.ps1 -SkipRust
```

只检查 Rust：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-dependency-audit.ps1 -SkipNpm
```

要求 Rust 审计工具必须存在：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-dependency-audit.ps1 -SkipNpm -RequireRustAudit
```

允许在实时拉取 RustSec advisory database 失败且本地已有缓存时使用 stale fallback：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-dependency-audit.ps1 -SkipNpm -RequireRustAudit -AllowStaleRustAdvisoryDb
```

安装固定版本 `cargo-audit`：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-cargo-audit.ps1 -Version 0.22.2
```

## CI 策略

- CI 使用 `Strict` 模式阻断 npm vulnerabilities 与 RustSec vulnerabilities。
- PC frontend job 在 `npm ci` 后运行 npm 依赖审计阻断、ESLint 10 lint、构建、bundle budget 和前端脚本测试。
- Rust server job 缓存 `cargo-audit` 二进制与 `~/.cargo/advisory-db`，并安装固定版本 `cargo-audit@0.22.2`；随后使用 `-Mode Strict -RequireRustAudit -AllowStaleRustAdvisoryDb` 运行 Rust 依赖审计。如果工具不可用则失败。
- `Strict` 模式当前只按漏洞数量阻断；RustSec warnings 仍输出 `DEPENDENCY_AUDIT_RUST_WARNING`，但不进入阻断计数。
- 如果 `cargo audit` 无法拉取 RustSec advisory database 或无法产出 JSON 报告，检查必须失败，避免把基础设施失败误报为 0 漏洞。
- stale fallback 只在已有本地 advisory-db 缓存时提供抗网络抖动能力；首次 CI 或缓存缺失时仍必须成功获取 RustSec 数据库。

## 临时例外

- 例外只能写入 `.github/dependency-audit-exceptions.json`。npm 例外必须绑定 package 名、`package-lock.json` 精确版本、advisory source、到期日和风险说明；RustSec 例外必须绑定 advisory id、package 名、`Cargo.lock` 精确版本、到期日和风险说明。
- 例外过期、重复或不再匹配当前审计报告都会让脚本失败，强制删除失效记录或完成复查。
- 依赖链上的父漏洞被例外覆盖时，只有完全由该父漏洞派生的 npm audit 条目才会被豁免；新的 direct advisory、版本变化或其他 source 仍会阻断。
- 当前 React Router 例外到期前必须复查上游是否发布可安装修复，并优先升级或移除例外，而不是延长日期。
- `RUSTSEC-2023-0071` 例外到期前必须检查 `rsa` 是否已有补丁；若无补丁，需重新确认所有私钥操作仍不对网络攻击者暴露可测时序。例外不覆盖任何其他 RustSec advisory。

## 收紧路径

1. 将 RustSec warning 分级：当前 warning 基线为 0，后续可考虑新增 warning 阻断。
2. 发布前复用 `check-dependency-audit.ps1 -Mode Strict`，保持本地和 CI 同一条漏洞基线。
3. 后续如引入安全例外，必须写入本文件并注明依赖链、影响面和复查日期。
