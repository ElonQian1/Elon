---
version_status: current
reviewed_at: 2026-09-08
evidence_status: account_observed_and_static_only
account_verified: false
execution_enabled: false
---

# 币安 U 本位网格：运行记录与参数合同研究

Win 正式安装版的浏览器研究 MCP 已读到登录页面采集的网格列表、详情，以及创建、修改设置、结束请求的业务成功响应。研究从“只有接口字符串”推进到 **5 类 `account_observed` 证据**；区间修改和独立平仓仍为 `static_only`。本报告证明取证结果，不定义交易权限，也不代表 APK 已接通。

本轮工具仅提交研究会话和读取命令，没有重放或提交交易请求。被动采集不能识别是谁触发了页面上的创建、修改或结束动作；不能将它们写成 AI 执行验收。没有核实资金、挂单、仓位的最终变化。

## 材料身份与完整性

- 正式 Win 运行身份：`0.3.69+aa6874c70b6090d13d8f22e8c16f4de55678336d`。
- 在原项目 `D:\rust\harness cli\elon cli`、原 owner/site 范围新开研究会话，复用该范围 Profile；未重启 Win 或 Chrome，未搬移登录资料。
- 会话：`10b36428eca051a967de49a223a122e9632a09230298f41747d0c20ca68cbb3a`；有效期截至北京时间 **2026-09-08 00:29:07**。后续不得凭本报告继续读取过期正文。
- 逐页扫描当时保留的 512 条请求；已到采集数量边界，不是完整网络历史。缺少某请求不能证明它没有发生。列表返回 20 行也不证明账号只有 20 个网格。
- 列表及各目标 JSON 通过 UTF-8 分片读取，校验连续 offset、同一资源身份、总字节数及 SHA-256 后解析。下面哈希对应凭据处理后的本机文本，不是原始网络字节；采集时间不证明网络执行顺序。
- [机器可读证据清单](binance-grid-runtime-contract-20260907.json)保存字段类型、请求路径、业务状态、资源身份和源码定位，全部 `executable=false`。字段集合只是样本并集，不能当作服务端必填字段或完整 schema。
- 本机证据位于 `D:\rust\active-projects\ElonNodeData\artifacts\private-grid-read-bridge-v1\research-10b36428\`。Git 清单保留证据文件名与哈希，不包含原始账号标识、策略 ID 值、持仓金额或登录凭据。

## 真实观察到的接口

实际 API origin 均为 `https://www.binance.com`，不能使用脚本 CDN origin。五份响应均为 HTTP 200、`code="000000"`、`success=true`；这证明样本的业务返回成功，不证明所有生命周期后置条件。

| 动作 | 方法与路径 | 请求证据 | 返回证据及边界 |
|---|---|---|---|
| 列表 | POST `/bapi/futures/v2/private/future/grid/query-open-grids` | 本条未取得请求正文，不推导无 body 或分页规则 | `data` 数组，20 行，23,428 字节；含 `WORKING`、`CLOSE_WITH_POSITION` |
| 详情 | GET `/bapi/futures/v1/private/future/grid/query-grid-detail` | 查询键 `strategyId`；不存值 | `data` 对象，1,363 字节；该样本 `WORKING` |
| 创建 | POST `/bapi/futures/v2/private/future/grid/place-grid` | 323 字节，15 个实际字段，见下文 | 返回 `strategyId/clientStrategyId/strategyType/strategyStatus/updateTime`；`strategyStatus=NEW`，并非已运行 |
| 修改设置 | POST `/bapi/futures/v1/private/future/grid/update-grid` | `strategyId:number`、`symbol:string`、`cps/sharing/trailingStopLowerLimit/trailingStopUpperLimit:boolean` | `strategyStatus=WORKING`、`updateStatus=SUCCESS`；未核对设置及挂单的最终状态 |
| 结束 | POST `/bapi/futures/v1/private/future/grid/close-grid` | 本条只有 `strategyId:number`，24 字节 | `strategyStatus=CANCELED`、`updateStatus=SUCCESS`；没有证明仓位清零或资金转出 |

对应响应 SHA-256：

| 动作 | SHA-256 |
|---|---|
| 列表 | `d37be4ea9e6336e2c9fd4f552cbc8db872d3b323dd184d262c518babb0ce82cd` |
| 详情 | `173e8cd65d5c5ff0c67d09697da1119eac56ebeee6cd2edb84f5e434bde9103c` |
| 创建 | `4ea2c7630e011fb027b4e8326e777200fe22c1838fe7c4514e5734f324ee0e5c` |
| 修改设置 | `71ea686bf970a74b6622aefcb1d2b2a033a22821f193fd54b904e1c834ccd447` |
| 结束 | `e3430fb79ab9838252a506cbc1a40f2595fa285b85aed87928545c88b66ba94e` |

列表样本的 `rootUserId` 全部存在且一致，只证明样本内部一致性，不替代当前项目 owner、本人账号连接及云端授权绑定。列表和详情的账号字段并不相同，不能随意互换。原始状态必须保留：前端常量包含 `CANCELLED`，真实结束响应却为 `CANCELED`；`CLOSE_WITH_POSITION` 也不能映射为“已平仓”。

## 创建参数：已连接表单、确认与请求封装

源码 C 的模块 74519 中，表单构造函数 `Dr` 保存草稿；确认函数 `jr` 调用 hook `P` 的 `mutateAsync`，USD-M 分支最终调用具名 HTTP 依赖的 v2 `place-grid`。这是静态调用链证据，不只是一条 URL 命中。

真实请求含下列字段，值未进入报告：

| 类型 | 字段 |
|---|---|
| string | `symbol`、`direction`、`marginType`、`gridType`、`gridLowerLimit`、`gridUpperLimit`、`gridInitialValue`、`orderCurrency`、`clientStrategyId` |
| number | `leverage`、`gridCount` |
| boolean | `cos`、`cps`、`autoInitPos`、`slideWindow` |

当前源码额外显示以下分支，但没有对所有组合做账号验证：

| 分支 | 已读出的前端行为 |
|---|---|
| 基础构造 | 包括上述核心字段及 `autoAddMargin`；当前真实样本未出现后者，不能写成服务端必填 |
| 方向与间距 | 常量包括 `LONG/SHORT/NEUTRAL`、`ARITH/GEO`；真实样本只支持各自已观察的值，不推导全部组合可用 |
| 初始建仓 | USD-M 且方向非中性时加入 `autoInitPos`；不能由这一处赋值认定所有 UI 模式均允许修改 |
| 跟踪 | 跟踪分支设置 `orderCurrency=QUOTE`，普通分支为 `BASE`；按方向和是否单向跟踪构造相关 stop 标志及 limit price |
| 触发与止盈止损 | 非零触发价才加入 trigger 字段；价格与 PNL/ROI 模式分别构造 stop 字段；部分字段受 feature flag 控制 |
| ROI 换算 | 模块 53944 把 ROI 百分数转换为 `initialMargin × ROI / 100` 的 PNL 字符串，再按调用精度舍入；不能将 ROI 输入直接作为 `stopTpPnl/stopSlPnl` 发送 |
| 动态挂单 | 非跟踪 USD-M 分支按接口/hook 返回阈值比较格数，生成 `slideWindow`；不能把旧 FAQ 的常数永久写死 |
| 确认阶段 | 增加生成的 `clientStrategyId` 和可选 `copiedStrategyId`；存在客户端 ID 不等于已证明服务端幂等或可以安全重试 |

金额语义已追到模块 45337 和数学模块 1298：前端计算 `initialMargin = gridInitialValue / initialLeverage`，`investedMargin = initialMargin + totalAdjustmentAmount`。因此不能把 `gridInitialValue` 直接显示为用户投入保证金。这里只确认当前前端计算，不证明交易所总冻结资金、最大损失或可用余额公式。

确认前还有表单、余额/账户及服务准入检查。前端存在切换持仓模式的独立流程；它影响账户级配置，不能隐藏在“创建网格”的普通参数修改中。本轮未执行该流程。

## 修改区间：独立的资金相关操作

源码 P 确认 `POST /bapi/futures/v1/private/future/grid/update-grid-range`，通过 `useMutation` 接到区间编辑表单。此路径本轮只达到 `static_only`。

构造字段为 `strategyId`、`symbol`、`gridUpperLimit`、`gridLowerLimit`、`gridCount`、`updateRangeCps`、`investmentDelta`、止盈止损上下限、跟踪上下限和 `tpslCps`。表单检查上下限或格数实际变化、可用资金、杠杆档位及输入错误。

`investmentDelta` 在需要追加时由前端计算和舍入，否则发送字符串 `"0"`。`updateRangeCps` 参与可用资金计算；源码含当前版本的计算系数，不能未经独立验证就复制为跨交易所业务规则。成功回调只显示“已提交”并使查询缓存失效，不足以确认区间更新完成。

该对象未提供方向、保证金模式或网格间距类型的修改字段，不能据此向用户提供这些已验证的运行中修改能力。普通 `update-grid` 与这个接口分别建模。

## 结束、保留仓位与独立平仓

模块 14393 的结束确认从详情读取 `cps`。用户改变选项后，静态代码先 `await update-grid` 更新 `cps`，仅在 `success=true` 时调用 `close-grid({strategyId})`；选项未变化时直接调用 close。采集时间来自异步落地，不能据其先后否定或证明 await 的网络顺序。

UI 将 `cps=true` 解释为结束后按市价平仓，`false` 解释为自行处理剩余仓位。旧账号与策略子账号的提示分支不同，`cos` 还参与挂单取消说明；**文案和代码分支不能代替真实挂单、仓位核对**。

另有独立 `POST /bapi/futures/v1/private/future/grid/close-grid-position`。已追到：模块 82389 的 `lj` 导出 → 18383 的 USD-M hook → 67893 的平仓面板 → 18139 的数量/价格构造。这一条仍是 `static_only`，不是结束接口的别名。

- 面板区分市价和限价平仓，支持部分数量。
- 构造 `symbol/type/side/quantity/positionSide/strategyUserId/strategyId`；限价分支另外提供 `timeInForce=GTC` 和 `price`。`isCM` 仅选分支，hook 在发请求前移除它。
- 平仓方向由持仓正负决定；数量来自实时持仓及输入检查。这里记录字段与静态分支，不提供可执行订单模板。
- UI 成功提示同样只是“订单已提交”；限价挂单尤其不能算作仓位已经关闭。服务端去重、拒绝行为、残余数量和撤单合同仍待验证。

## 源码版本与最小定位

| 标记 | 资源与 SHA-256 | 本地 UTF-8 offset |
|---|---|---|
| C | `4818.ab7ba930.js`；`24765919f67706cdb4e894bb491a4904e9000fa64efa5814ee808fd40fc0dcf2` | 66500：创建 hook；79300：创建构造 |
| P | `page-ee55.21718cb8.js`；`de6da80226b765083ad5bfa49ed741e24ec4148587673d88bf14412c786cc446` | 391600：区间构造；349180/355780：结束；1196473：保证金；1389166：枚举；169636：ROI |
| W | `4dd8fa7a.e193caf7.js`；`db9498381816b948bca1714190e2d0be69328bed703503810eb6d011cc27b382` | 0/3700：结束导出/封装；12400/15000：独立平仓 |
| M | `0ea9e839.4f78b70f.js`；`5706010994076314b5114776cc79d3bcea8c916aae3157708e3bf10c8cc8a815` | 0：除法/加法函数绑定 |

P 的独立平仓定位另为 329250、91095、131909；JSON 清单包含完整 resource ID、读取区间和证据文件哈希。片段证明所读代码，不等于已对整个脚本做全量审计。

## 现有架构复用与下一验收点

量化基线 `f28f9ca6302917d48201c449a3804c4192235e49` 的 `crates/grid-center-adapters/src/binance.rs`、`binance_input.rs` 已有只读列表/详情解析。它保留 `provider_status`，只把已支持的 `WORKING` 映射为运行，其他状态保留未知；金额使用 Decimal，未核对的资金和收益项目保持未知。本轮只核对源码，未把这份真实正文送进 Rust 可执行程序，不能宣称完成真实解析验收。

后续接入可以复用通用 GridBot、来源/连接身份、Observation、读取中心和 OKX 展示；币安路径、字段语义、状态差异保留在币安适配器。方向、间距和状态的枚举扩展应分别附证据，不能让静态候选自动升级为可执行能力。创建、设置修改、区间修改、结束意图和剩余仓位处理需要各自的能力与后置条件。

本报告未改量化代码或 f28 APK 候选身份，也未启用交易能力。当前分层验收状态：

| 层次 | 状态与下一步 |
|---|---|
| Win 研究链路 | 本轮读取已验证；过期后按同一项目/owner/site 新建会话再取证，不能沿用旧正文作为新鲜资产 |
| 接口合同 | 5 类观察证据、2 类静态调用链；补错误合同、超时结果核对、幂等与最终挂单/仓位证据前，写能力未验收 |
| Win 私人投影 binding | 当前旧版接口 404；需要正式新版激活并核对节点、desktop 和 owner 绑定 |
| 量化同步 worker | 独立构建此前在 num-traits build-script 两次 `0xc0000005`，没有可运行 exe；本轮未重复构建 |
| 云端与量化 APK | 可信 HTTPS 和签名候选已有；缺真实节点 ACK、商店上传和手机授权读取验收，整体 `in_progress` |

Win 激活及 APK 工件事实继续以[桥接交付记录](private-grid-read-bridge-v1-20260907.md)为准。本轮恢复了研究，不代表解决了更新器与同步部署问题。

证据清单与本机来源完成 88 项一致性/链接断言，覆盖 5 个运行样本、2 个静态操作及 16 个源码片段的哈希、身份和 UTF-8 区间；结果保存为上述本机目录的 `contract-validation.json`。这不是 88 项交易或 APK 测试，本轮没有重新构建或部署应用。
