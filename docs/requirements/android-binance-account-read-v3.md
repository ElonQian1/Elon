---
version_status: current
reviewed_at: 2026-09-11
decision_status: accepted
implementation_status: in_progress
---

# 币安账户绑定读取 V3

量化需要按当前账户保存列表选择，并从详情进入管理后重新核对同一账户。现有 V2 列表缺少账户标识，不能仅凭策略编号完成此流程。本需求承接原生量化/主 APK 会话服务边界；产品 UI 全部归量化。

在现有只读 grant 下增加固定 `read_v3`，根 schema 为 `yilong.binance_host_read.v3`。保留 V2 全部字段及金额/指标定义，增加 `account`（已核验币安 UID 的 SHA-256）和 `account_kind`（primary/sub/unknown）。标识必须与现有管理命令的 account 同源，不传原始 UID 或凭据，不增加交易权限。

调用方及 grant、用途、平台用户、账户和到期校验继续使用现有主线程 Runtime；账户字段与同一响应的行来自同一次状态。换号/撤销旧 grant 拒绝，过期行保持空，空账户列表仍可识别真实账户。read/read_v2 形状完全兼容。

修改 Provider 路由、Runtime 读取版本及 State 序列化，每个文件增量不超过 30 行；新增独立合同测试，覆盖标识与管理同源、旧版不扩字段、同编号换号、撤销/过期和空列表。发布主 APK；量化自己完成投影隔离、目标带入和确认前检查。双包安装、真实读取/换号和导航仍需各自证据。
