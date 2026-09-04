---
title: "ESK Sui Currency 观察器 V1 交付证据"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, protocol]
---

# ESK Sui Currency 观察器 V1 交付证据

本文件只记录验证与交接，不定义经济参数或发行许可。
范围见[正式需求](../requirements/esk-sui-currency-observer-v1.md)，
操作见[使用手册](../esk-sui-currency-observer.md)。

## 当前状态

| 能力 | implementation_status | verification_status | delivery_status | acceptance_status |
| --- | --- | --- | --- | --- |
| 原发布观察器共享传输兼容 | implemented | integration_passed | pushed | accepted：内存传输合同，不是 ESK 发行 |
| 规范 Currency/历史注册/固定供应双源观察 | implemented | integration_passed | not_started | pending：真实 ESK 参数与双源 |
| 非 ESK 官方公开 schema 检查 | implemented | environment_passed | not_started | accepted：仅公开 schema，不替代 ESK 验收 |
| 主项目/量化用户资产上线验收 | partial：复用前批交付 | user_action_required | 本批不发布 | deferred：审批/受保护登录/安装授权 |

纯传输抽取已独立提交并推送：`0a9631c36226192e48986984be46a8b6dcfee5df`。
新能力代码与正式证据收尾尚待本批推送，后续状态更新不得仅凭此段认定已上线。

## 已执行验证

- Node.js 22.14.0；精确 SDK `@mysten/sui@2.29.0`。
- 新观察器 312/312 离线测试通过，其中领域校验 251 项；含合法 CLI 参数映射
  与 observed/unverified 两条真实进程退出码测试，观察边界用内存替身隔离网络。
- 原发布观察器 65/65、新共享传输 15/15 通过；旧 QUERY、返回、导出引用和
  同步校验顺序保持，DNS/TLS/超时/大小边界未放宽。
- 创世 schema/语义、供应守恒及 Move 源码绑定回归通过；本轮没有运行 Move
  Runtime、Cargo 或 Android 构建，也没有签名或广播。
- `npm ci --ignore-scripts --no-audit --no-fund` 通过，锁文件使用官方 npm registry
  URL 与完整 integrity，本目录不共享或修改 PC 前端依赖。
- SDK 派生公开 SUI canonical Currency 与实际已知对象一致：
  `0xf256d3fb6a50eaa748d94335b34f2982fbc3b63ceec78cafaa29ebc9ebaf2bbc`。
  该向量是 SUI，不是 ESK 部署参数。

测试覆盖错误链/包/币种/注册版本/创建输出、字段缺失、u64 精度、BURN_ONLY/null、
当前版本变化、重复交易 checkpoint 矛盾、双源分歧、部分失败与 CLI 输入脱敏。
两组独立审查分别负责网络兼容与官方 API/领域绑定，未发现未解决的 v1 缺陷。

## 可重复公开网络证据

运行入口为 `scripts/esk-sui-currency-observer/tests/public-schema-smoke.js`。
没有显式 `--run-public-non-esk-smoke` 参数时不联网。
混合样例包含一个非 ESK 包与公开 SUI Currency；即使每个字段均能查询，也必须
被完整 ESK 校验拒绝。它不能证明 ESK 已经发行或双方读取源达成一致。

2026-09-04 15:13:28.554 UTC，固定官方测试网查询实际运行一次，2 秒内退出 0。
`schema_pass=true`、`sdk_address_match=true`、`non_esk_reject=true`、
`rejection_code=REGISTRATION_MISMATCH`、`source_count=1`、
`NO_BALANCE_OR_CERTIFICATION=true`。公开样例的注册 checkpoint 早于不相关包的发布，
正式 ESK 校验器因此拒绝；这是负向完整性证明，不是双源 ESK 正向观察。
本机执行日志标识为 `esk-sui-public-schema-smoke-20260904-231326-549`，
摘要不含密钥、钱包或用户数据；源码提供可重复检查入口。

## 独立证据刷新

原 `esk-sui-publication-observer-v1` 的旧证据绑定含被本轮纯抽取修改的文件，
路线图也经历后续资产交付。已明确重开并重新认领以回归验证当前合同，随后通过
工具重绑实际源码和测试；不手改旧哈希，不假称旧证明自动覆盖新源码。
新功能登记为 `esk-sui-currency-observer-v1`。两者不声明完整链上发行已验收。

## 剩余缺口与下一步

1. 需要真实 ESK 的完整链标识、包/发布/注册交易、注册创建版本、已批准供应及
   审核过的第二公开 GraphQL 来源，执行双源只读观察。不得拿本地测试参数补数。
2. 需要源码对应性、逐桶分配与供应守恒、能力交接、地址所有权、委员会终局性
   及迁移历史验证；当前工具不会推进旧 manifest 或链余额。
3. 若尚未发布 ESK，实际测试网签名/发布前确认网络、钱包、正式参数与用户授权。
   本工具不持有任何签名或资金操作权。
4. 主项目/量化资产、卖回、上传、安装和本人验收继续按
   [首批用户路线图](../requirements/esk-first-user-delivery-roadmap-v1.md)进行。
   量化新版上传、真实付款审批及受保护的本人读取通道仍是独立环节；本轮不重新
   构建、冒名上传或更改生产政策。

主项目 HTTP、APK、账本与独立量化仓库本批无业务修改；不恢复暂停的 PC 自动监督。
隔离树仅处理本批源码/测试/文档；主工作区未知改动和未跟踪文件保留，最终以
统一 finish 回执分别报告，不把本机同步问题当作真实 ESK 已验收或未验收的替代。
