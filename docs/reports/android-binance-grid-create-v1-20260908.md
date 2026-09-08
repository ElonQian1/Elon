---
version_status: current
reviewed_at: 2026-09-08
implementation_status: partial
acceptance_status: user_action_required
---

# 本人币安网格人工创建：实现与验收

需求：[人工创建 V1](../requirements/android-binance-grid-create-v1.md)。本批落实用户本人
测试的原生创建链路，AI 没有提交真实交易。最终运行和资金后置条件须由用户提交后核对。

## 实现

- 量化 `HostedGridActivity` 新增入口，经双向签名/组件和随机 nonce 检查，进入主 APK
  `BinanceGridCreateActivity`。只有这一可见原生页提供本次最终提交，旧只读 provider
  和 MCP 不增加写操作。回执是独立的 `yilong.binance_create_result.v1`。
- `BinanceGridDraft` 为唯一原生参数和金额计算源。保证金乘杠杆得到 gridInitialValue，
  十进制不经 JS Number；多空、逐仓/全仓、等差/等比、立即建仓、停止处理均显式选择。
  本轮输入保证金上限 2000 USDT，不等于最大损失保证；不自动追加保证金或改变账户模式。
- 首轮是基础静态网格，未提供止盈止损、跟踪、触发或运行中修改。确认页明确显示这些
  缺项和停止委托的作用范围。服务端可拒绝不满足实际交易规则或准入的参数。
- 页面闭包只保留当前实际网格查询的请求上下文；固定身份 GET 精确核对 UID 摘要，
  固定公开 coef GET 取得 windowCount。缺配置不猜默认值；确认前后变化拒绝发送。
  凭据不进入原生、日志、IPC 或测试夹具，redirect/method override 被拒绝。
- `BinanceCreateAttempt` 在网络前消费一次意图，`BinanceCreateJournal` 在不参与系统
  备份的 app 私有目录原子落地状态。重启只恢复未知/受理状态，没有可重发的草稿。
  创建没有自动重试；结果未知只能读查询或由本人在官网核对后结束本机记录。
- 受理响应必须匹配本次 clientStrategyId。NEW 只显示受理，单独固定详情 GET 再核对
  账号和策略 ID。WORKING 不代表所有订单、仓位、费用或资金已对账。

## 官网取证补充

9 月 8 日在原项目 owner/site Profile 新开研究会话，保留原 Win/Chrome 窗口及登录资料。
会话 96942f693513e824a726967884f0cf45afe204d77a4b9c1ca2aab97fd6e6a61e 仅用于读取，
不是交易执行器；旧私有正文未作为当前账号或资产使用。

| 证据 | 定位与结论 |
|---|---|
| 最新配置模块 | page-ee55.148047e5.js，SHA-256 `7e4ef73c9811d2b2eab572e43acdb409045e46c999f521a0f899196bce464747`，offset 1184001；模块 4629 调用 `/bapi/futures/v1/public/future/common/grid/coef` 并读取 data[field]；64121 指定 windowCount |
| 客户端编号 | 4818.ab7ba930.js，SHA-256 `24765919f67706cdb4e894bb491a4904e9000fa64efa5814ee808fd40fc0dcf2`，offset 11959，模块 71603；manual_mm_web 前缀、随机数/时间拼接并裁到32字符。此形状不证明服务端幂等 |
| 保证金模式 | 新 page 脚本 offset 1388297 附近的 61759 模块明确包含 ISOLATED/CROSSED；同脚本 offset 104963 附近保留停止时取消该合约未成交委托的说明 |
| 创建字段与状态 | 延续[已观察合同](binance-grid-runtime-contract-20260907.md)中的15字段样本，新增派生配置依据；字段组合和服务端错误仍须本次用户测试 |

## 分层验证

| 能力 | 当前证据 |
|---|---|
| 原生金额/状态与旧只读 | 最终文件冻结后 Android 编译与24项 JUnit通过；脚本同时核验 XML 中失败/错误为0，耗时404秒 |
| 页面适配器 | 12项创建测试、18项只读会话、5项诊断及19条旧适配器断言通过，均合成网络；包括换账号、配置变化、重复、未知结果、取消、XHR和凭据不外传 |
| 量化入口/结果 | 标准 Android 验证102项 Debug通过；正式签名候选9af152d的102项Release测试通过，安装身份另记 |
| 整仓 Rust | 量化 fmt通过，clippy遇rust_decimal/num-traits构建脚本0xc0000005；未修改Rust，不宣称整仓通过 |
| 真实下单 | user_action_required；AI不提交创建、修改、结束或仓位操作 |

## 用户测试入口

量化 APK → 手机币安网格 → 创建网格（本人确认）→ 主 APK 填写参数 → 检查参数与当前
账号 → 核对摘要后由本人决定是否创建。若显示未知，先在官网核对，不重复点击创建。
主 APK 缺新 Activity 时量化显示更新要求。测试结束可返回量化并重新授权读取当前列表。

APK源码、正式发布、安装哈希与实际入口检查在取得回执后补记；本文件不预告成功。
