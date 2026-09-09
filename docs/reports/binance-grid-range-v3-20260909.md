---
version_status: current
reviewed_at: 2026-09-09
implementation_status: in_progress
---

# 普通运行网格区间编辑 V3

主APK增加独立的`BinanceGridManageV3Activity`，保留V1/V2组件和调用者签名验证。量化只选择受信任组件、校验V3回执；凭据、参数确认和私有请求仍由主APK托管。

官网资源`page-ee55.148047e5.js`的SHA-256为`7e4ef73c9811d2b2eab572e43acdb409045e46c999f521a0f899196bce464747`。已读取的构造器调用POST `/bapi/futures/v1/private/future/grid/update-grid-range`，字段为strategyId、symbol、gridUpperLimit、gridLowerLimit、gridCount、updateRangeCps、investmentDelta和原价格止盈止损/追踪限价、tpslCps。这是前端源码证据，尚无本批真实写响应验收。

本批仅开放普通非追踪、无正值PnL止盈止损的WORKING网格。原区间、格数、价格止盈止损、仓位终止设置和投入快照在准备与发送前各读一次；账号、文档、设置或资金快照变化则不发送。原可选字段的null与未提供分别保留。高级合同未核实的网格继续提供官网入口。

用户明确填写新区间、格数、是否按市价平掉现有仓位和追加金额（不追加须填写0）；页面可填入当前区间，但不猜测追加资金。临时上限为2,000 USDT，交易所最低追加金额尚未实现本地估算，参数和资金最终仍由交易所检查。金额均为精确十进制。

准备60秒过期；单次提交前保存不可重放记录，未知结果不自动重试。V3日志只持久化参数目标摘要，兼容旧日志；重启后不能恢复为可提交状态。读取到相同区间仅说明参数匹配，不能证明平仓、追加资金或订单重建全部完成。

接口离线61项通过，覆盖字段、精度、旧设置保留、缺项、账号切换、状态变化、未知结果和不可重放；原生Release75项通过，覆盖V3日志、到期、取消和旧版兼容。主线新增的Web Chat恢复修复与本模块不相交，重基无冲突；正式发布重新编译通过。

正式主APK1.1.1601(1601)已发布并覆盖安装，来源`11f981c11d2e789a77e6675c7544a66bfef7945f`，APK SHA-256 `3741b3990b0db317454f971c8383cc3adc67c68e30cf1570fe3c462976a53bd9`。量化0.6.6能进入新版管理页。MCP只读实测row_count=1、read_outcome=verified、detail_current=true、unresolved=false；但range_details_available=false，现有策略不具备本轮完整区间合同，不能宣称该策略可原生修改区间。未推断具体缺失字段或高级类型原因。

真实金融提交由用户验收；本批工具未执行交易。普通适用网格的真实完整字段、区间编辑结果与高级合同仍待验收，功能登记继续in_progress。
