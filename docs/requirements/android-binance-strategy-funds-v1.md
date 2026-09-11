---
version_status: current
reviewed_at: 2026-09-11
decision_status: accepted
implementation_status: in_progress
owner: quant-grid
---

# 币安网格策略账户资金只读合同 V1

承接量化商业化目标 G06。主 APK 在已授权的 `grid.read` 范围内读取当前策略的 USDT 保证金信息，量化拥有产品页面。此切片不读取交易所整个钱包，不扩大旧授权到现货、资金、其他策略账户，不提供划转或交易。

## 来源与范围

已缓存的官网公开前端 `page-ee55.148047e5.js`（SHA-256 `7e4ef73c9811d2b2eab572e43acdb409045e46c999f521a0f899196bce464747`）模块 26866 包装 `POST /bapi/futures/v1/private/future/strategy/user-data/get-future-account-info-asset`；构造 `{marginAsset, strategyUserIds}`。模块调用点字符索引 366815、392502 从已核验详情取得 `strategyUserId`，按该 UID 读取数组第一项的 `marginBalance`、`crossInitialMargin`。量化仅支持 USD-M/USDT，因此固定 `marginAsset=USDT`，只传当前详情中的一个策略 UID。

源码只证明调用链；本批开始时尚无该接口的真实响应证据，Win 研究命令已过期未执行。安装后必须验证响应形状与字段，不把离线 fixture 写成账号取证。未知形状或缺失策略行失败关闭；缺失金额保留 null，不借用其他账户、不取无约束第一行、不默认 0。

## 固定接口

- 新增 `report_capabilities_v1(grant)`，鉴别调用 APK 并验证当前只读 grant 后返回 `yilong.binance_report_capabilities.v1` 与明确 kinds 列表。
- 既有报告 V1 新增 `kind=funds`，固定 page=1、精确策略 ID/合约；旧客户端读取其他 kind 不变。消费者必须先确认 funds 能力。
- 主服务核验策略属于当前列表/已核验历史；适配器查询详情确认策略/合约及实际账户，使用详情的策略账户 UID。读取前后再次确认登录账号和账号类型，旧请求和已撤销状态不得发布结果。
- 上游响应只允许目标 UID 对应的一项；可选 asset/marginAsset 若存在必须是 USDT。响应中的其他账户及字段不得跨 APK。
- `coverage=strategy_margin`，rows 恰为一项：`asset=USDT`、`marginBalance:string|null`、`crossInitialMargin:string|null`。金额保持十进制精度。报告不是钱包总资产、可用余额、可提金额或最大可损失金额。
- 继承报告的超时、5 分钟失效、离屏清除、换号/撤销清除。诊断仅记录固定阶段和错误类别。

## 验收

测试覆盖请求固定路由/正文、策略 UID 选择、其他 UID 和额外字段隔离、精度、null 与零、重复/缺失行、不同币种、换号/迟到响应、上游失败和能力版本。Android 合同与消费者分别验证；原签名发布、装机、真实只读响应、页面和返回各自记录状态。不得用本切片关闭“交易所完整钱包余额”缺口。
