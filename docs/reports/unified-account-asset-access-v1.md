---
title: "统一账号与只读资产授权 V1：开发验证记录"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-account-access, quant-client]
implementation_status: partial
---

# 统一账号与只读资产授权 V1

依据[已接受需求](../requirements/unified-account-asset-access-v1.md)和
[正式合同](../contracts/unified-account-asset-access-v1.md)。本记录证明源码与本地验证，
不证明生产上线、用户完整入口、真实账号或双 APK 运行验收。

## 本批实现

- 主服务新增独立授权、PKCE 一次兑换、身份/正式资产读取、本人授权列表和撤销接口。
  用户、客户端、用途、父会话和有效期共同限制权限；SQLite 只保存凭据摘要。
- V289 增量迁移建立授权存储。委托读取与账本校验在同一事务内，不通过全局登录
  校验器提升权限；正式登记源保持唯一，不写量化账本，不移动资金。
- 资产投影使用精确整数串、白名单字段和版本化 JSON；分页绑定资产快照，变更时
  有界重读首页。旧主会话接口和逐页 APK 合同继续使用原权限。
- 新路由要求本服务实际 TLS 握手证据；伪造转发头不能启用。独立 TLS 标记不消耗
  现有节点会话证据。普通 HTTP 返回 426；无效浏览器 Origin 在解析正文前拒绝。
- 主 APK 新增独立授权组件：核验官方量化调用方，展示本人明确同意，仅交回短期
  授权码。量化新组件完成兑换、持续读取、翻页及撤销，凭据仅在内存。
- `sdk/asset-access/` 提供 Web/Node 共用客户端和类型声明；外部 AI 宿主可消费脱敏
  结果。令牌、授权码和验证器不能进入模型上下文、工具结果或日志。

## 本地验证

| 范围 | 命令/证据入口 | 结果 |
|---|---|---|
| SQLite 授权与正式账本 | `scripts/validate-rust.ps1 -- test --manifest-path server/tests/esk-platform-harness/Cargo.toml` | 141 通过 |
| 真实 Store/路由集成 | `scripts/validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server access_http_tests -- --test-threads=1` | 5 通过 |
| SDK 行为和实际 Rust JSON 互认 | `node --test sdk/asset-access/test/client.test.js sdk/asset-access/test/rust-wire.test.js` | 44 通过，含导出文件，无跳过 |
| 主 APK 编译与新合同 | `android/gradlew.bat -p android :app:testDebugUnitTest --tests com.elon.app.esk.platform.access.AssetAccessRequestTest --no-daemon --max-workers=2` | 6 通过 |
| 量化完整验证 | 量化仓 `scripts/validate.ps1` | 162 Rust、42 前端通过 |
| 量化 Android | 量化仓 `scripts/validate-android.ps1` | 84 测试及 Debug 构建通过，默认 Release 失败关闭 |

互认导出由 harness 的 `synthetic_delegated_wire_export_matches_formal_truth_without_credentials`
从真实 Store 产生；使用固定合成余额，不读取用户资产，不导出凭据。复现方法见
[SDK 说明](../../sdk/asset-access/README.md)。两仓使用相同公开 JSON 测试向量，SHA-256：
`3f243e578d1bed415d59c2fb8c43b8545b48389cca26e0783dcfbb51725a4c02`。

首次全二进制目标测试还遇到现有 PC 节点测试编译错误；本批最终通过的是明确指定
`--bin elon-server` 的服务端目标，不据此声称全部节点二进制测试通过。
Android 本机 JDK 临时管道问题以进程级临时目录配置解决，没有改全局机器配置。

## 尚未完成与协作边界

1. `ELON_ASSET_ACCESS_ORIGIN` 在两 APK 默认空，需可用的 HTTPS 主服务 origin 和
   实际 TLS 部署。当前 HTTP 预览不可承载此授权。尚未配置、发布或验收生产链路。
2. 量化新 Activity 已注册为私有组件，当前“我的 ESK”入口仍走旧逐页方案。用户已
   授权由原任务继续新版 APK 发布，本任务负责统一接口；新入口切换与发布待协调。
   量化代码保存在 `codex/account-asset-access-v1`，不推进其发布主线。
3. 主账号授权列表/撤销已提供 API，但主 APK 可视授权管理中心尚未接入。
   量化新页只有收到服务器撤销成功才显示已撤销；本地清理不等于远程撤销。
4. 父会话在服务端撤销或过期会让委托立即失效；主 APK 现有退出操作只清本机状态，
   不证明服务端父会话已撤销。跨端退出联动需在共同账号入口后续接入。
5. Web 同意与回调页面、外部 AI 工具宿主尚未实现；SDK 和合同不等于这些产品已上线。
   Web 跨页授权须由宿主保留同次 PKCE 状态，不能靠丢失内存状态后重新生成来续接。
6. 未构建或发布本批正式签名 APK，未进行本人账号、双 APK 或真机验收。

后续按“HTTPS 服务部署 → 双 APK 受控构建与入口协调 → 本人授权/翻页/撤销/到期验收”
推进；Web 与 AI 宿主复用同一接口，不增加第二套资产计算。
回退时先关闭新客户端入口并撤销新授权；保留 V289 授权审计数据，不破坏性降级数据库。
