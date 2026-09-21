---
version_status: current
reviewed_at: 2026-09-22
---

# 欧易账户余额读取交付

- implementation_status: implemented
- verification_status: automated_passed
- delivery_status: published_and_xiaomi_installed
- acceptance_status: user_pending

主 APK 新增账户余额能力探测和只读合同，读取 USDT 可用余额、权益、冻结金额。请求前后核验账户，旧网格授权默认没有余额权限；新版同意页可以复用已加密保存的凭据，用户确认扩展范围后生效。现有网格/历史/记录保持兼容。

`OkxBalanceTest` 和 `OkxReadHostTest` 覆盖零与缺失值、精确十进制、币种重复/非法数值、旧授权拦截、已有凭据再次确认、请求中换号与撤销。欧易模块 Debug 9 套 / 37 项通过，失败和跳过均为 0。

量化页面接入由子项目 `docs/requirements/exchange-account-balance.md` 定义。尚未在真实欧易账户进行余额页面验收；授权确认及手机页面由用户完成。

正式主 APK 1.1.1801 / 1801 已发布，源码 `a8f8679036251cc2c720c60303a33d7d919d9235`。发布流程从主项目设备档案核验身份并执行 `adb install -r`：小米 23116PN5BC 成功安装并回读 1801；HONOR AAK-AN00 离线。没有拉起页面或执行交易。发布后自动 worktree 清理报告 Branch 属性异常，后续执行任务原始 finish 合同核验本机收尾。
