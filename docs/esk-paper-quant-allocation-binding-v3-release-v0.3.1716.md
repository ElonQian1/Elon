# ESK Paper 量化申请绑定 V3 发布回执

状态：主项目已发布；量化子项目代码已推送，独立公网环境未部署。

## 发布结果

- 主项目版本：`v0.3.1716`
- 主项目提交：`00e5756d1d73dd149ea0b2e2b2e31f71b19debe1`
- 量化子仓提交：`origin/main@424b530`
- 后端 health/version smoke：通过
- PC 前端生产构建与 bundle 硬门禁：通过
- 功能登记：`esk-paper-quant-allocation-binding-v3` 已推进为 `released`

## 用户边界

主项目现在包含 ESK 余额、量化申请选择、accepted/released 状态和签名回执同步代码。量化子项目包含模拟 binding 的接收、查询、释放与回执重放代码。量化独立 HTTPS origin 和双方生产签名配置尚未验收，因此跨站量化入口继续失败关闭。

本次发布不发行链上 ESK，不导入真实付款，不移动资金，不创建基金份额、NAV、订单、交易、收益或可提现余额。操作与恢复说明见 `docs/yilong-quant-esk-allocation-binding-v1.md`，本地专项验收见 `docs/esk-paper-quant-allocation-binding-v3-acceptance.md`。
