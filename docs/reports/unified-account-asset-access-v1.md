---
title: "统一账号与只读资产授权 V1：开发验证记录"
version_status: current
reviewed_at: 2026-09-06
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

以下表格是 2026-09-05 原实现批次的历史验证，不替代后续修改的当轮测试。

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

## 2026-09-06 SDK 接续批次

当前主任务按用户既定分工接续本功能，旧任务继续既有 APK 与 Sui 收尾。
本批基线 `e4c8e1c503ad0e707e035990357dd653986c906c`。
功能 plan/check_drift 确认原 16 条证据和需求全部 current、0 漂移；原认领已过期，
通过正式工具重新认领后开展增量修复，不登记重复功能，不据此升级为完整产品验收。

本批只修改共享 SDK、独立测试、SDK 使用说明及本文，服务端、迁移、Android、
量化分支和旧 APK 发布路径均不改动。三个独立复现的缺口如下：

| 触发 | 原行为 | 本批修复 |
|---|---|---|
| 读取或授权尚未结束时撤销 | 读取中因 request_in_progress 拒绝；尚无令牌时直接抛错，待完成的授权仍保留 | 无条件清本地并中止旧流程；有旧凭据才尝试远端撤销，仅有效服务器回执证明远端成功 |
| 同一快照翻页 | 只查单页重复，跨页重复进度 ID 或循环游标仍可接受 | 单条分页链检查已见 ID/游标，任何重复失败并清理，首页/新快照/期限缩短/clear 重置 |
| 原始 JSON 有重复字段 | 普通 JSON.parse 先覆盖，再核对身份和字段 | 有界严格解码拒绝重复及转义等价键、危险键、BOM/非法 UTF-8、孤立代理字符及过深输入 |

SDK 只保留当前分页链的去重标识，不建立新余额或累计资产业务规则。
为限制内存，单链最多记录 10,000 个请求 ID，超过返回 pagination_limit 并清本地授权；
这不更改服务器总记录合同，也不能把已读部分说成完整历史。JSON 最多 32 层，
仍保留现有响应字节限制；模块不依赖 Node 专用 Buffer 或第三方库，可供浏览器加载。

新授权可以中止尚未完成的旧撤销；此时返回 cleared，远端结果未确认。
旧请求的迟到成功或失败都不能恢复旧资产，也不能清除新授权。
普通读取之间继续串行，只有明确撤销优先中止读取。
尚无令牌时撤销仍返回 authorization_required，不发送撤销 HTTP，但同时取消待完成的
授权和兑换；旧兑换的迟到令牌不能重新恢复权限。

### 当轮验证记录

- 最小三个回归先在原核心实现上全部失败，日志 `unified-access-sdk-red-20260906-175808-066`；
  修复接线后 3/3 通过，日志 `unified-access-sdk-three-green-20260906-175945-243`。
- 当前 Rust Store 的合成导出通过项目 validate-rust 入口重新执行，非旧成功回执复用，
  日志 `unified-access-sdk-rust-wire-20260906-175721-282`，约 34.5 秒。
  验证指纹 `8ca84339d74e630e4d5daa19e7452e9d9ae33b8b7f46d4d713987107ba869a61`。
  导出只含实际序列化的合成身份和资产响应，生成时断言不含令牌、授权码或用户材料。
- 尚无令牌的两条独立撤销回归先 0/2 通过，日志
  `unified-access-sdk-revoke-red-20260906-180904-732`；补充修复后纳入完整套件通过。
- 完整 SDK 套件含新增 25 条回归和本次 Rust 导出互认：69/69 通过、0 失败、0 跳过，
  日志 `unified-access-sdk-full-20260906-180955-770`。覆盖跨页重复/游标循环、
  10,000/10,001 条边界、全部分页重置路径、严格 JSON/Unicode/深度以及撤销竞态。
- 独立只读复审通过；源码体积门禁检查 9 个文件通过，文档模块化检查 2 个文件通过、
  0 警告，暂存改动空白检查通过。本批采用 CodePushed 收尾，不触发服务或 APK 发布。

### 本批确认的剩余事项

量化团队资产分支 `b5b0b70` 与统一授权 consumer 分支 `ec9db422` 是共同基于 `295ba23`
的两条独立分支；量化主线仍未包含这些集成。新 consumer 私有 Activity 已存在，
首页仍进入旧逐页流程。其传输目前将所有 409 视为快照变化，后续接入应收紧为精确错误码，
并补真实 HTTPS 传输和页面生命周期验证。公共 PWA 继续不暴露本人资产。

另从现有 V289 SQL 在纯内存合成库复现了 INSERT OR REPLACE 可覆盖已消费码/审计的
数据库约束防御缺口。正常 API 未使用该语句，grant/token 唯一约束仍在，
没有证明 HTTP 可重复领 token 或远程越权。本批不改服务器；后续如加固，
应采用对既有数据库生效的增量迁移及回归，不能只重写已经执行过的 V289。

生产 TLS、两端编译 origin、正式 APK、主授权管理页面、退出联动、Web/AI 宿主和本人验收
仍分别未完成。SDK 源码交付不等于服务器部署、安装或实际用户闭环。

## 2026-09-06 重复撤销回归修复

后续复审发现前批 69 项测试未覆盖同一授权连续调用 revoke 的情况。第一次调用已清理
本地令牌，第二次调用虽然报 authorization_required，仍因 finally clear 中止第一次远端
请求，导致无法确认撤销。本批基线为 `a327112ffc19efe4da35c43486793323ab822ce7`；
独立确认该缺陷后重新认领原功能，未登记或接入其他功能。

修复单独记录当前授权代次的撤销请求，重复调用在清理前返回 request_in_progress，
不取消首个请求，也不发送第二个请求。主动 clear 或新授权仍可取消旧撤销；
成功、失败及显式清理均解除标记，旧请求只能清理所属代次。调用者仍须等待首次请求的
有效服务器回执，重复点击错误和本地清空均不是远端已撤销的证明。

- 新增 4 项独立回归：连续撤销、旧回执与新授权隔离、失败后重新授权并撤销、
  主动清理后取消尚无令牌的授权。修复前 2 通过/2 失败，日志
  `asset-access-double-revoke-red-20260906-182959-562`。
- 修复后完整 SDK 73/73 通过、0 失败、0 跳过，日志
  `asset-access-double-revoke-full-20260906-183018-004`；独立只读复审通过。
- 通过 validate-rust -Force 重新运行实际 Store 合成导出，日志
  `asset-access-double-revoke-wire-20260906-182839-732`；上述全套已读取本次导出。
  验证指纹仍为 `8ca84339d74e630e4d5daa19e7452e9d9ae33b8b7f46d4d713987107ba869a61`，
  服务端源码未变化。导出不包含凭据或真实用户资产。

本批只修改 SDK 客户端、一个回归测试文件、SDK 说明和本报告；采用 CodePushed 收尾。
不推进 APK 入口、TLS 配置、正式发布或本人验收，整体功能保留待接入状态。

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
