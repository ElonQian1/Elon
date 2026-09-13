---
version_status: current
reviewed_at: 2026-09-13
implementation_status: implemented
verification_status: partial
delivery_status: pending
owner: quant-grid
---

# 量化前台宿主生命周期

需求：[固定绑定服务](../requirements/android-binance-host-lifecycle-v1.md)。复现主1700更新后量化原管理页connecting，既有MCP调试保活启动后同页恢复账号/列表，详情需另读。主进程当时系统事实为非冻结，不能把问题直接归结为Android cached freezer，也没有证据支持放宽读页面重载的交易保护。

新增纯绑定生命周期服务，事件能力可选声明schema；不访问Runtime、网页、账号、授权或交易，不允许启动后自行常驻。量化核验原签名和固定组件后，仅在前台订阅期间持有绑定。所有业务继续经既有Provider鉴权。静态合同、12组适配器检查、Android编译及97项原网格回归已通过；正式发布和原页更新实测待完成，不能把声明支持等同于恢复成功。

原MCP接口与量化内置检查均复用；本次临时调试服务已停止，未增加任意脚本、凭据导出或自动交易功能。未知结果保护、重载互斥和账户隔离均保持。
