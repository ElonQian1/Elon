---
version_status: current
reviewed_at: 2026-09-13
implementation_status: implemented
verification_status: partial
delivery_status: partial
owner: quant-grid
---

# 量化前台宿主生命周期

需求：[固定绑定服务](../requirements/android-binance-host-lifecycle-v1.md)。复现主1700更新后量化原管理页connecting，既有MCP调试保活启动后同页恢复账号/列表，详情需另读。主进程当时系统事实为非冻结，不能把问题直接归结为Android cached freezer，也没有证据支持放宽读页面重载的交易保护。

新增纯绑定生命周期服务，事件能力可选声明schema；不访问Runtime、网页、账号、授权或交易，不允许启动后自行常驻。量化核验原签名和固定组件后，仅在前台订阅期间持有绑定。所有业务继续经既有Provider鉴权。静态合同、12组适配器检查、Android编译及97项原网格回归已通过，不能把声明支持等同于恢复成功。

主1.1.1702（1702，源码`45d3826b09fc97ff99d6085193773ef7da476e4f`）已由官方入口正式发布；本地、上传验证和线上版本清单的APK SHA-256一致，为`f9030090a1e777fe0ef4eebc080d76b1d6f9f8e34a3b48329aa4fd33e6e20caa`，原签名`f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`验证通过。量化0.7.48已无线安装且包摘要一致。发布脚本因未配置ADB自动目标而跳过装机；手机随后在线读取到主1701、安全锁屏，之后无线设备转为offline且重连超时。1702安装及无调试保活的原页恢复验收仍待完成，未将发布成功记作实际恢复成功。

原MCP接口与量化内置检查均复用；本次临时调试服务已停止，未增加任意脚本、凭据导出或自动交易功能。未知结果保护、重载互斥和账户隔离均保持。
