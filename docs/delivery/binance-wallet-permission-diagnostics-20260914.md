---
version_status: current
reviewed_at: 2026-09-14
implementation_status: implemented
acceptance_status: pending
---

# 钱包同意结果诊断

此前MCP的`wallet_summary.status=idle`只描述内存读取状态，不能证明用户尚未同意。进程或会话重建后，诊断缺少持久同意记录信息，容易把恢复流程误判成需要再次授权。

新增`wallet_summary.permission`，使用固定`yilong.binance_wallet_permission_facts.v1`，区分没有记录、需要验证当前身份、账号不匹配、需要恢复临时权限和当前可读。只有记录存在、身份有效、范围匹配、临时权限有效全部成立时才报告`ready`。存在记录本身不证明当前授权有效，也不作为账户身份凭证。

该观察不启动网页、不识别账户、不恢复/续期/创建grant，也不请求钱包金额。主用户和币安账户摘要、nonce、grant及金额均不进入输出；量化与主APK正式权限合同、UI和交易边界不变。需求见[只读诊断合同](../requirements/binance-wallet-permission-diagnostics-v1.md)。

41项定向测试全部通过、零跳过，耗时356.9秒：新增诊断3项覆盖五种状态、16种布尔组合和输出白名单，原钱包授权意图11项、钱包状态11项、宿主状态15项、授权绑定1项。手机量化0.7.68在主1.1.1728上实际返回`consent_required`，不把它记为余额读取成功。正式原签名包发布、安装和新增MCP结果待记录。
