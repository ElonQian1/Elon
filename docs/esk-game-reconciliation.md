---
version_status: current
reviewed_at: 2026-09-12
implementation_status: partial
---

# 游戏奖励的离线核算与来源合同

`tools/esk-game-reconciler` 验证独立来源签名及本地证据内容摘要，生成主项目已有 `Settlement v1` 的**未签名候选**。它没有联网、交易所凭据、签名私钥或付款入口；正式核算方审核全部材料后，才能对确切候选签名并通过已有管理员接口登记。主项目现有签名信任模型没有改变，不能宣称已强制所有核算方使用本工具。

## 上游实际状态与归属

2026-09-12 只读审查量化仓库提交 `8410ae6e25b2f0c935c2e4f004b47c8b49fb2d5c`：

- `docs/delivery/grid-income-projection-20260911.md` 描述已实现损益、浮动损益、净收益、交易费和资金费的展示投影，明确真实费用覆盖尚未完整验收；未知金额不能当作零。
- `apps/api/src/fund_readiness.rs` 仍将正式资金产品标为 `design_only`、`paper`、不接受客户资金、不启用真实交易。
- `crates/fund-product-core/src/nav.rs` 的 QSHARE 净值与 `distribution.rs` 的 ESK 团队利润分配属于不同权利。账户总 PNL、份额 NAV 或币价不等于某玩家可分配游戏利润。
- 当前没有已核验的本工具来源出口。本文 `source.v1` 是待上游接入的合同，不是声称量化现有 API 已提供这些字段。

正式接入应由量化端或其独立只读账务适配器建立：真实账户/环境和参与者归属、成交与费用完整覆盖、资金进出对账、参与者分配、已执行币种转换及准备金覆盖。主项目继续拥有账号、ESK 与游戏预算；不复制交易所密钥、OMS 或 QSHARE 账本。

## 可信配置与输入

配置 `esk.game.reconciliation.config.v1` 的字段见 [Config](../tools/esk-game-reconciler/src/model.rs)。操作方须从真实已固定主项目政策复制 `policy_digest`、`asset_type`、`asset_decimals` 和 `reconciler_public_key_hex`，再固定独立 `source_public_key_hex`、`source_id`、`account_scope`、`user_id`、`beneficiary`、`wallet_binding_digest` 及 `accounting_started_at_ms`。工具只核验此本地配置与来源一致，**不从主项目在线认证配置，也不证明钱包绑定或来源公钥的现实归属**；配置审核是独立核算方的责任。

请求为 `esk.game.reconciliation.request.v1`，含 `statement: {payload, signature_hex}` 与 `previous`。第一期 `previous=null`；后续必须提供上一期来源 envelope 和核算方签名的上一 `Settlement`。不允许跨期间缺口、跳号、来源分叉、累计交易费或其他成本倒退。更换钥匙、政策、钱包、起算点或成本口径需显式迁移，不能重置序号绕开累计亏损。费用退款应由上游采用明确审计的收入/修正口径处理，不修改历史成本。

来源 `esk.game.reconciliation.source.v1` 固定 `environment=live`、`basis=participant_cumulative_cash_v1`，所有身份及单位必须匹配配置。`live` 是来源方签名的环境声明，不是工具独立验证交易所的结果。Paper、未定义来源、未知字段、缺失/null 金额、不规范整数均拒绝。

五类证据必须按以下固定顺序提供 `kind` 和非零 `sha256`；同一完整材料可覆盖多个角色，但每个角色只能出现一次：

1. `account_ownership`：账户、真实环境、参与者与受益人归属。
2. `cashflow_reconciliation`：期初/期末资产、本金、资金进出、未实现估值及现金核对。
3. `trade_costs`：全部已结算交易及费用/资金费完整性，未知不得填零。
4. `participant_allocation`：本次单参与者分配及全池总额约束，不能重复领取别的权利分配。
5. `reserve_coverage`：本金负债、损失吸收、已承诺提现和奖励义务的资金覆盖与当前留存。

每份原始归档位于 `EVIDENCE_DIRECTORY/<sha256>.json`。哈希针对精确文件字节，不规范化 JSON；每文件最多 1 MiB，较大材料应由上游归档清单引用并由核算方继续核验。CLI 校验当前与上一期全部直接引用文件；**哈希匹配仅证明内容一致，不验证材料真实性、清单深层引用或准备金充足**。

## 金额与序列

所有金额均为确切 payout 币种最小单位、规范十进制字符串；无隐式 USDT/USDC/ESK 汇率：

```text
累计净利润 = cumulative_realized_trading_pnl_units
           + cumulative_net_funding_units
           - cumulative_trading_fees_units
           - cumulative_other_costs_units
           - held_reserve_units
```

资金费正值是收入，负值是费用。交易损益是已平仓交易毛损益，包含累计亏损，但不混入资金费或本金进出，不能只累计盈利交易。交易费不能同时扣进 gross PNL；收入与费用必须按本合同采用唯一口径。`held_reserve_units` 是当前额外留存金额，不是历次留存之和，也不重复包含已经由主账本历史预留扣减的奖励。准备金释放会增加净额，必须附新覆盖证据。

`principal_liability_units` 和 `unrealized_pnl_units` 被明确排除，保留在候选分解中供审核。工具保留负净额；历史已发奖励**不在此处再次扣减**，由主账本以“累计净利润 − 全部历史预留”计算剩余额度。算法使用 i128 中间值，最终限制在主项目 i64 范围，溢出失败。

原“10 USDT 本金保障”仍需独立本金负债、可即时兑付准备金和损失承担机制；本工具不实现或证明该承诺。准备金不足、无法覆盖成本、未知净利润时，上游不得签发可分配正收益。

## 运行与签名交接

通过项目受管入口构建，程序使用以下四个位置参数：

```text
esk-game-reconciler CONFIG REQUEST EVIDENCE_DIRECTORY NEW_OUTPUT
```

Config 上限 8 KiB，请求上限 32 KiB。先验签及核算，再检查归档，最后 `create_new + sync_all` 保存候选；任何已存在输出都拒绝覆盖，包括输出等于输入。失败不打印账号/材料内容。磁盘写入故障可能留下不完整新文件，操作者应保留检查，换新输出名重新核验，不能将该文件当作已签名结算。

候选包含原来源签名、核算分解、原 `Settlement` 类型和 `settlement_signing_bytes_hex`，所有付款/已签名/独立事实验证标志均为 false。材料摘要绑定签名来源的完整 payload 与固定来源公钥。保留配置、原请求（含上一期证明）、当前与上一期归档以及候选，核算方复核后对**输出中准确字节**签名；不得从展示金额重新拼装交易。

来源签名字节：UTF-8 `ESK_GAME_RECONCILIATION_V1\nsource\n` + 按 Rust `Statement` 字段顺序的紧凑 JSON。核算摘要使用相同前缀、用途 `calculation`、紧凑 `Calculation`；均无尾换行。Ed25519 严格验签，摘要 SHA-256 小写 hex。

最终 Settlement 签名使用已有 `ESK_GAME_REWARDS_V1\nsettlement\n` 合同。得到 `Signed<Settlement>` 后才可走[主账本流程](esk-game-rewards.md)。本工具不提供“读取任意 JSON 后用生产私钥签名”的命令。

## 验证与交付边界

当前测试使用生成密钥、合成账务、本地文件和真实主项目 SQL/密码学源码，覆盖协议字节一致性、2000 单位只预留一次、亏损恢复、准备金释放、身份/期间/签名错误、证据缺失/篡改/超限及两个 CLI 并发输出。它们不构成量化真实利润或真实主账号资金的联合验收。

受管测试 9 项库测试、4 项真实 CLI 测试通过，验证指纹 `869237dec9fae8e2c7719bb92a7d55fc858b52c54cd94478c15f2e3cca1c18e9`；格式化工作流回归通过。初次验证因缺少新包 Cargo.lock 在构建前拒绝，使用仓库锁文件初始化函数生成并固定依赖后通过，未绕开锁文件或修改全局 Cargo 配置。

严格 Clippy（全部目标、`-D warnings`）通过，指纹 `d8786e9467ad3a84e0c4563bcfdf194f96fe765d518e0c8bbc7bb2dad74e33a7`。该命令初次因固定 1.97.0 工具链缺少 Clippy 而失败，安装同版本组件后重验通过。Windows 与 Bash 显式文件格式检查均通过；增量源码及文档门禁通过。

该工具作为独立代码交付，不更改在线后端、不触发资金或服务器发布。下一步是量化侧实现并验收上述来源出口，再执行独立核算方审核交接及公开测试网联测。
