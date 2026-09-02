# 一龙 ESK 量化 Paper 模拟绑定与释放

状态：代码已实现并完成本地合同验证；主项目发布和量化独立 HTTPS 环境仍待验收。本文不代表链上 ESK、真实资金或量化产品已经上线。

## 用户看到的流程

1. 用户在主项目资产页看到 ESK 总额、可用额、卖回占用、量化申请占用与总占用。
2. 用户提交一笔 ESK Paper 量化分配申请后，该金额进入 `submitted` 占用；用户在量化项目入口明确选择这笔申请。
3. 主项目签发最多五分钟、只绑定这笔申请和当前脱敏参与者的 `yeqa1` 授权；它不进入 URL、storage、日志或数据库。
4. 量化端验证同次 grant 与授权后创建 `accepted` 模拟 binding，并返回独立签名的 `yqar1` 回执。主项目验签成功后，资产页显示“已被量化 Paper 接收”，金额仍占用。
5. 用户在量化页面申请释放，量化端追加 `released` 回执；主项目验签后释放占用，资产页显示“已释放”。

```text
submitted -> canceled
submitted -> accepted -> released
```

只有 `submitted` 能在主项目直接取消；`accepted` 必须从量化端释放。网络中断时，量化端会保留原 accepted/released 回执，本人重新进入后可再次同步，不会重复建立 binding。

## 双仓库信任边界

| 内容 | 主项目 | 量化子项目 |
|---|---|---|
| ESK 余额与占用真源 | 唯一负责 | 不保存余额副本、不铸造 ESK |
| 单申请授权 | 用主项目 Paper 签名域签发 `yeqa1` | 只持有公钥并验证 |
| 模拟 binding | 不创建 | 追加式 SQLite 保存 accepted/released |
| binding 回执 | 只持有量化公钥环并验证 `yqar1` | 用独立量化回执 seed 签发 |
| 完整 token | 授权不落库；回执只存 SHA-256 | 授权只存 SHA-256；回执可作为非 bearer 审计工件重放 |

主项目配置 `YILONG_QUANT_ESK_RECEIPT_KEYRING_JSON`；量化端配置成对的 `QUANT_ESK_ALLOCATION_RECEIPT_SIGNING_KEY_ID` 与 `QUANT_ESK_ALLOCATION_RECEIPT_SIGNING_SEED_BASE64URL`。两套签名域、participant subject secret 和运营令牌不得复用。

## 明确不是这些东西

- binding 不是链上 ESK、基金份额、QSHARE、RWA、入金证明、成交或可提现资产；
- accepted 不会调用旧 NET 参与账本，不折算 USDT，不进入 NAV；
- released 只释放主项目 Paper 占用，不代表官方已经付款或链上卖回；
- 全过程固定 `simulated=true`、`funds_moved=false`、`quant_units_issued=false`、`nav_participation=false`、`trading_started=false`；
- 6% 仍只是非保证目标，不由这个状态机计提、承诺或支付。

## 故障与运营处理

- 缺少量化回执 signer 时，量化 runtime 不声明 binding capability，主项目不得发送单申请授权。
- 缺少或非法主项目回执 keyring 时，回执同步返回不可用，原申请不会被客户端自报推进。
- accepted/released 同步失败时，不手工改数据库；用户用同一申请重新进入，量化端重放原签名回执。
- 回滚先停止签发新 `yeqa1`，保留本人 binding 列表、release 和回执同步，直至 accepted binding 全部释放或由后续正式迁移接管。

权威需求为 `docs/requirements/esk-paper-quant-allocation-binding-v3.md`；双仓合同为 `contracts/quant/esk-paper-allocation-authorization-v1.schema.json` 与 `contracts/quant/esk-paper-allocation-receipt-v1.schema.json`。
