# 币安网格丰富持仓报告

状态：accepted。主 APK 托管会话，量化绘制产品 UI。

复用既有策略详情、影子账户持仓和资金读取，补充源端 positionInitialMargin、initialMargin、maintenanceMargin、unrealizedProfit、notionalValue、markPrice、liquidationPrice、leverage 等可选字段。仅读取当前策略 shadow UID 和合约；不以登录 UID 代替。缺失或非法可选值保留未知。

新增 report_read_v2 返回丰富报告，report_read_v1 保留原字段形状。量化优先 v2、旧宿主降级 v1；授权、账户隔离和响应时间门禁不变。不增加交易动作、不修改主 APK 产品表单。

验收包括影子账户过滤、额外字段精度、未知值、旧协议字段集合与新协议，以及单元测试和正式 APK。用户自行验收手机页面。
