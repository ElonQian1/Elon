# 一龙 NET 量化锁定回执 V1

## 状态与结论

本文件和 `contracts/quant/net-balance-lock-receipt-v1.schema.json` 定义主项目与“一龙量化交易”之间的第一版余额锁定协议。当前只完成跨仓库契约；主项目尚未发行 NET，也没有可供真实用户使用的 NET 可用余额/锁定余额运行时，因此任何真实回执都必须失败关闭。

量化子项目当前仅在 paper 导入中接受 `simulated=true`、`status=locked` 的回执，用来验证数据绑定、份额会计、重启恢复和用户界面。Schema 合法不等于资金真实、发行方可信或用户已完成准入。

## 责任边界

| 项目 | 负责 | 不负责 |
|---|---|---|
| 一龙主项目 | 未来维护 NET 可用/锁定余额真源，原子执行余额迁移并签发版本化回执 | 计算量化内部份额、NAV 或直接修改量化账本 |
| 一龙量化交易 | 校验回执版本、参与者、金额、用途和余额变化，创建独立内部仓位 | 铸造 NET、修改主项目余额或把 NET 当作量化份额 |

## 回执语义

- `participant_ref` 是跨系统协商的脱敏标识，不得使用邮箱、电话、钱包地址或付款平台原始编号。
- `amount` 是本次锁定的 NET 数量；`purpose` 固定绑定 `quant_position/yilong-quant`。
- `balance_revision` 是主项目余额聚合的单调修订号，消费者不得自行生成。
- 锁定操作必须满足 `available_after = available_before - amount` 且 `locked_after = locked_before + amount`；JSON Schema 负责形状，消费者负责精确十进制算术。
- `previous_receipt_digest` 为同一锁定生命周期的前序回执摘要；首张回执为 `null`。
- `status=released` 为未来解锁/退出后的状态表达，不代表当前量化子项目已实现真实赎回。

## 当前 paper 使用限制

1. 只允许 `simulated=true`、`status=locked`；真实回执直接拒绝。
2. 回执参与者和金额必须与模拟参与命令完全一致。
3. 导入只保存脱敏技术数据和摘要，不导入姓名、KYC、付款截图或原始支付资料。
4. 6% 只能显示为非保证目标；内部份额只按模拟 NAV 上下波动。
5. 批量导入接口不是生产运营后台；在身份、审批、鉴权和隐私控制完成前不得公开。

## 生产升级前置条件

- 主项目正式 NET 资产模型、双分录余额账本和唯一余额真源。
- 回执发行方身份、密钥轮换、规范序列化、数字签名和重放域。
- 用户身份/授权、KYC/AML、制裁和地区准入。
- 量化独立托管、会计、NAV、对账、退出和灾难恢复。
- 双方分别通过安全验收，生产消费者默认拒绝 unsigned、unknown issuer 和 `simulated=true` 回执。

这些条件必须另行立项；本 V1 不提供绕过入口。
