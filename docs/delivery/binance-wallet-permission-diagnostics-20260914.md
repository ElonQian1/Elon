---
version_status: current
reviewed_at: 2026-09-14
implementation_status: implemented
acceptance_status: device_verified
---

# 钱包同意结果诊断

此前MCP的`wallet_summary.status=idle`只描述内存读取状态，不能证明用户尚未同意。进程或会话重建后，诊断缺少持久同意记录信息，容易把恢复流程误判成需要再次授权。

新增`wallet_summary.permission`，使用固定`yilong.binance_wallet_permission_facts.v1`，区分没有记录、需要验证当前身份、账号不匹配、需要恢复临时权限和当前可读。只有记录存在、身份有效、范围匹配、临时权限有效全部成立时才报告`ready`。存在记录本身不证明当前授权有效，也不作为账户身份凭证。

该观察不启动网页、不识别账户、不恢复/续期/创建grant，也不请求钱包金额。主用户和币安账户摘要、nonce、grant及金额均不进入输出；量化与主APK正式权限合同、UI和交易边界不变。需求见[只读诊断合同](../requirements/binance-wallet-permission-diagnostics-v1.md)。

41项定向测试全部通过、零跳过，耗时356.9秒：新增诊断3项覆盖五种状态、16种布尔组合和输出白名单，原钱包授权意图11项、钱包状态11项、宿主状态15项、授权绑定1项。

正式主APK 1.1.1729（1729）已发布并通过`adb install -r`从1728覆盖安装，保留登录资料。发布源码为`ca314674134aed0b34aac74371f917d080a4e7ff`，APK SHA-256为`7a2a3a7eb30d4a9dd05e0adf915bfbb24017d799c9269fb8c00511ae614ff77f`；包名、原签名、非debuggable、服务器元数据和本地产物一致。量化仍为0.7.68，本批未修改量化UI。

装机后新增MCP结构实际返回`not_granted`、`consent_recorded=false`，冷态身份为false；从量化打开钱包连接后，识别出子账户，`identity_verified=true`，权限仍为`not_granted`。一次限定语义读取确认授权页显示“已确认：币安子账户”，同意按钮可用。诊断准确区分身份识别成功与授权未保存，观察本身没有创建权限；量化资产页仍为`consent_required`，尚未取得钱包响应，不能把零条响应当成零余额。

当前授权页已准备好，首次有效同意及之后的钱包数据回读仍待用户操作。此项诊断设备验收通过，不等于钱包读取全链路或网格工具目标完成。独立UI Renderer因无空闲模拟器记为`VERIFICATION_DEFERRED`，不声明视觉验收通过。外部证据目录为`ElonNodeData/artifacts/grid-wallet-permission-diagnostics-v1`，包含测试、原签名产物校验、装机证明、两阶段权限状态与授权控件摘要。
