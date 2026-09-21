---
version_status: current
reviewed_at: 2026-09-22
implementation_status: in_progress
---

# 欧易账户余额只读合同

用户要求量化首页按绑定交易所显示账户资金。主 APK 新增 `balance_capabilities_v1` 与 `balance_v1(grant)`，固定全球正式站 GET `/api/v5/account/balance?ccy=USDT`，仅 Read 密钥可用。量化不接触密钥。

- 回执 `yilong.okx_account_balance.v1` 包含 account、account_kind、generation、observed_at_ms、quote_asset=USDT、available、equity、frozen；三个金额分别来自 details.USDT 的 availBal/eq/frozenBal。
- 缺失币种或字段为 null，零保留零，不能把 USD totalEq、可用权益或策略投入混充余额。
- 请求前后重新核验账户及只读权限；注销、撤销、凭据替换使迟到结果无效。使用现有串行网关，不添加后台轮询。
- 新版连接同意范围明确包含网格和账户余额。加密保存 balanceAllowed；旧 vault 缺字段默认 false，旧用户需在主应用确认扩展的只读范围，既有网格读取不受影响。
- 保留 v1/v2/v3 网格、历史、记录合同。新版能力可独立探测，旧主应用明确要求升级。
- 完成标准：金额与身份边界测试通过、双包构建与发布；手机余额和页面验收由用户完成。

接口依据：[OKX balance](https://www.okx.com/docs-v5/en/#trading-account-rest-api-get-balance)。币安使用已有独立 wallet_summary_read 授权及接口，本次不改动其口径。
