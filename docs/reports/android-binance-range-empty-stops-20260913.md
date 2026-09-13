---
version_status: current
reviewed_at: 2026-09-13
implementation_status: implemented
verification_status: partial
delivery_status: published_and_installed
owner: quant-grid
---

# 普通网格空保护与区间恢复

需求：[空保护兼容V1](../requirements/android-binance-range-empty-stops-v1.md)。量化保留管理UI，主APK修复现有会话适配器及投影，不增加跨APK字段、写权限或凭据传递。

## 定位证据

量化0.7.46/主1699的策略、账户、投入和保护可读，但区间没有带入。既有Win研究库中完整且哈希匹配的成功详情（`code=000000,success=true`）将未设置的 `stopTpPnl/stopSlPnl/stopUpperLimit/stopLowerLimit` 表示为空字符串。旧区间合同拒绝空字符串，导致整个区间快照为null。12份历史样本回放覆盖普通做多/做空与追踪网格；修复后普通网格恢复，追踪仍被现有边界拒绝。

样本内容哈希如 `853408aed1407bef064fe67cb40d4c63a46dea404c1ff90c12cc38c5a086da7b`（普通做多）、`5df5e5ff394d276eedc157a2434627b269c49842a623f171ace59253678c237e`（普通做空）。只记录字段形状与完整性证据；这些是历史响应，不是当前手机的实时资产或最终写入证明。旧Win研究MCP不支持新版hosts动作，未重启或改变已登录Win/Chrome。

## 实现与验证

- 金额保护只接受原有零值或明确未设置；非零保护、未知类型、空白、指数价格、缺失必填项和实际追踪仍拒绝。
- JS与Kotlin内部保留可选字段的空字符串/null/缺失差异，准备后的基线改变仍阻断发送。对量化输出保持既有“精确价格或null”合同，摘要把未设置写清楚，避免空字符串导致客户端严格解析失败。
- 新增适配器回归在旧实现失败（24项中2项失败），修复后24项全通过；覆盖实际字段形状、无写准备、离线载荷保留、非法值和准备后保护改变。新增2项Kotlin回归覆盖解析、投影与摘要。
- 官方币安12组适配器全部通过，97项原生回归及Debug编译通过；正式构建、发布和无线安装完成。尚未进行真实交易，不将离线模拟写请求计为实盘验收。

## 正式发布与手机验收

主APK1.1.1700（1700），源码`ba7724b21c6e6223e496d8d75c98bae01b060f15`，APK SHA-256为`9e596e7ed4ad8814ab57edd14dfcf417105526fe7ff004b984ea1f7a05709a9a`。官方发布入口核对服务器版本与文件摘要；无线`adb install -r`成功，手机实际base.apk摘要与发布工件一致。量化沿用0.7.46（63，源码`d0eca77bbd0198e4f0b4fc3d9e9d37c5b3bf9128`），未为主侧修复重复发量化包。

主更新后原管理页面连接失效；通过量化返回、刷新网格并重新进入管理恢复，未重启量化进程或手动打开主页面。当前同一账户的一条策略详情核验通过，区间下限/上限/格数自动带入，原1699缺失的区间已恢复。不变参数返回“所选设置没有变化”；仅把草稿格数减一、追加投入为0、保留仓位，得到`prepared`、有效准备及摘要，确认未勾选。随后清空四项数字草稿，准备失效、确认区消失，返回新鲜网格详情。未提交、修改仓位或结束策略。

受控工件组`grid-range-empty-protection-v1`保存`main-install-proof.json`、`device-0913-new-host-range-prefill.json`、`unchanged-visible-messages.json`、`device-0913-changed-range-result.json`、`device-0913-cleared-preparation.json`与`device-0913-after-cleanup-grid.json`。检查主要复用量化内置语义接口，仅两次有界层级读取用于当前输入及无变化提示；不把状态证据当作完整视觉验收。

## 后续

本解析缺陷已通过当前正式组合的只读正向验收。主包更新时原页需要恢复操作的体验、用户最终确认及交易所效果、完整自然期限、全部管理参数限制与视觉另记，不因本问题解决而宣布G13全项完成。准备成功只证明本地合同及当前基线检查，不证明交易所已接受区间修改或最低追加投入。
