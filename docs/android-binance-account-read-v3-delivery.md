---
version_status: current
reviewed_at: 2026-09-11
implementation_status: implemented
acceptance_status: pending
---

# 账户绑定读取 V3 交付

主 APK 新增同一只读 grant 下的 `read_v3`：列表与当前账户不透明标识、账户类型在同一主线程响应中输出。量化据此隔离来源、带入管理目标并保护报告账户范围；产品页面仍归量化。需求见[账户绑定读取](requirements/android-binance-account-read-v3.md)。

- `account` 与既有管理回复使用同一个已核验账户的摘要；不传原始 UID、Cookie、凭据或写权限。
- 旧 read/read_v2 形状保持；新版本固定 schema 和字段。原有调用方签名、用途、平台用户、撤销及到期校验不变。
- 39 项宿主 Debug 单测与 Debug/Release 编译通过；覆盖空列表身份、旧版字段、换号/换类型旧 grant 拒绝、撤销及过期。官方发布、双包安装和实际只读导航另行记录，尚未取得本版本的设备证据。

手机无线连接未成功恢复，未重启 Win、Chrome 或 ADB 服务。没有真实金融操作。主包发布不能替代量化原生界面发布，也不代表完整账户钱包功能已完成。

关联基线：[止盈止损 V3](android-binance-protection-v3-delivery.md)已装机，读取、准备及 V2 兼容曾通过，交易/视觉待验；[策略资金](android-binance-strategy-funds-v1-delivery.md)已发布 1.1.1660，安装及实际响应待验。两项既有能力不因本次读取版本扩展而退役。
