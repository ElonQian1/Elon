---
version_status: current
decision_status: accepted
reviewed_at: 2026-09-14
---

# 钱包同意结果的只读诊断

承接量化首发G06/G26及用户“点击同意没有反应”的排查。现有MCP只有内存会话/读取状态，主应用进程重建或页面离开后，无法判断钱包同意是否落盘。增加固定诊断事实，使AI能先核实，再决定恢复读取还是交用户确认，避免凭`idle`反复要求同意。

- 在现有`binance_host_status`的`wallet_summary.permission`内返回固定schema与布尔事实：是否存在同意记录、当前身份是否已验证、记录是否匹配当前身份、当前临时读取权限是否有效。
- 状态区分`not_granted`、`identity_required`、`account_mismatch`、`resume_required`、`ready`。有记录不等于当前可访问；无当前身份不能声称匹配。输入矛盾时按失败关闭方向归类。
- 只读诊断不调用`begin`、`identify`、`approve`、`resume`、`read`或`refresh`，不续期、不创建grant、不打开网页、不取代本人同意，也不读取币安资产接口。
- 输出不含账户/主用户摘要、grant、nonce、token、Cookie、金额或网址；原钱包权限文件、期限、撤销和量化结果合同保持不变。
- 单元测试覆盖各状态和全部布尔组合，保证权限事实不被上游矛盾输入提升。原钱包/主会话回归通过，正式包发布后用MCP只读核验；钱包金额和用户操作仍单独验收。

文件：独立`BinanceWalletPermissionFacts.kt`（约50行）、wallet runtime和MCP状态各少量接线、独立测试（约80行）。本批不改交易、量化UI或浏览器登录状态。
