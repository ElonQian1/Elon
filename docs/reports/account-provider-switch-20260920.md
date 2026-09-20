---
version_status: current
reviewed_at: 2026-09-20
implementation_status: partial
---

# 账户入口与币安切换

## 本批能力

- APK 账号与安全汇集 Google 登录绑定、OpenAI 官网账户入口与币安账户入口。Google 沿用已有服务器绑定；网页登录不冒充联合登录绑定。
- 币安账户管理复用量化的唯一 `BinanceHostRuntime`，显示当前已验证账户类型、UID 尾号和账户指纹。提供完整官网、切换、核对与取消切换。
- 显式切换先持久化旧账户指纹，撤销网格与资产读取授权、销毁旧文档和读取状态。旧 UID 或仅账户类型变化不能完成切换；新 UID 经既有适配器验证后重新建立文档。未完成的切换跨进程保留，取消也不恢复旧授权。
- 量化连接页提供账户管理入口；返回时重新接管共享 WebView 和状态回调，再由用户确认新账户读取授权。没有扩大交易权限或清空其他网站 Cookie。
- Win 账号与安全汇集 Google、OpenAI 和币安入口，复用现有本机官方窗口。Win 与手机保持独立会话；手机量化从手机主 APK 授权。

## 使用路径

主 APK → 账号与安全 → 管理／切换币安账户 → 开始切换 → 在官网退出旧账户并登录新账户 → 核对账户 → 回一龙量化重新连接并授权。量化现有连接页也可进入此管理页。

取消切换只结束切换流程，保留官网当前登录；仍需重新授权量化。

## 验证与剩余缺口

- PC 类型检查、生产构建和修改文件 ESLint 已通过；Google/ChatGPT 入口合同及币安读取会话 22 项回归已通过。
- Android 编译与 `BinanceHostStateTest`、`BinanceAccountSwitchTest` 共 18 项通过；切换测试覆盖旧 UID 拒绝、类型变化拒绝、新 UID 接受、旧授权不复活、摘要不展示完整指纹。
- 本机 JDK 默认 Unix socket 临时路径使 Gradle 报 loopback connection 错误；本次构建仅通过 `JAVA_TOOL_OPTIONS` 指向任务 `.ai-tmp` 短路径恢复，不修改系统或仓库全局配置。
- 真实官网退出、另一账户登录、量化新账户回读尚未验收。没有自动执行用户真实登录、退出或交易。
- OpenAI 具体账户资料尚无统一安全投影，仍需在官网查看；Win 币安入口也不把窗口打开状态当作账户已登录。多账户同时保存、跨设备账户列表与 OpenAI 云端绑定不在本批已实现范围。
- 当前工具会话未提供 `project_feature_workflow`，没有手工修改功能注册表。

## 文件边界

Android 新增 `AccountProviderViews`、`BinanceAccountActivity`、`BinanceAccountSwitch`，入口仅接线；会话切换归既有 host。PC 只改 `features/account`，路由仍为 `/pc/account`，没有旧 PC 静态资源、后端 API 或量化产品 UI 改动。
