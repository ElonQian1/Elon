---
version_status: current
reviewed_at: 2026-09-08
implementation_status: partial
acceptance_status: user_action_required
---

# 本人网格设置与结束：交付证据

需求：[管理V1](../requirements/android-binance-grid-manage-v1.md)。已复用[创建V1](android-binance-grid-create-v1-20260908.md)
的本机官方会话、账号证明、原子记录和严格回执。量化新增独立管理入口，主APK原生页面
选择当前已验证列表里的策略；列表没有记录时不伪造策略。

## 已编码能力

| 能力 | 实现及边界 |
|---|---|
| 读取与选择 | 当前账号新鲜列表内选择ID，再GET详情，前后核验UID；只显示有完整类型证据的终止设置 |
| 终止处理设置 | 只改变cps，保留sharing与两个trailingStop布尔字段；必须实际变化且WORKING |
| 结束 | 只发数值strategyId；原cps必须与本人选择一致，不自动先改设置再结束 |
| 提交核验 | 账号、文档、ID、状态与全部相关设置再次比较；安全整数外ID拒绝，60秒准备失效 |
| 并发与恢复 | 创建/管理共用独占槽；另一类未核对记录阻止新操作；不明确的网页确认按未知恢复 |
| 结果 | 成功只记受理；独立详情观察目标设置/结束状态，仍不证明仓位归零、挂单撤销或资金结清 |
| 官网 | 区间、格数、追加保证金、止盈止损、独立平仓，以及缺少必要字段的情况继续使用官网 |

网络依据为[已观察合同](binance-grid-runtime-contract-20260907.md)中update-grid六字段、
close-grid单字段和query-grid-detail。源码不猜额外资金字段。既有只读Provider/MCP没有
增加写入口；可见原生页才提供最终本人确认。工具没有发送真实交易。

## 验证进度

- 合成网络9项管理测试通过，覆盖独立结束、保留其他设置、错ID/账号、换账号、取消、
  字段变化、未知/拒绝不重试和凭据不进入回执；原13项创建测试也通过。
- 最终35项原生回归通过，包含网页回执不明确时保持未知；日志binance-manage-ack-final。
  另外18项只读会话、5项诊断测试及19条旧适配器断言通过，原创建能力保留。
- 量化Debug/Release各105项通过，GitHub Actions 34212745032成功。b0cf03541caa592d30bd40da1b78ffce5235be2b
  原签名候选已安装，base.apk哈希为9ae75ea7313f04c2bf3984857c1316ca457e602698bb6b625284c3c376d2129a。
- 主APK 1.1.1573（1573）已通过正式入口发布并安装到手机；源码30cdb857d08006bf1f6a7a0a498584014e2341c8，
  服务端、本地产物与手机base.apk的SHA-256均为5c5f7041bb8eb898493f6b61a1dcc8506b22ac513530d41b0a03914dda27681a。
- 双APK设备导航尚未验收：用户已允许临时页面探针，但手机安装仍返回INSTALL_FAILED_USER_RESTRICTED
  （Install canceled by user）。正式产品更新成功，探针未安装；不能据此声称新页面已验收。
  新主APK的MCP只确认管理会话尚未打开，当前账号与列表未核验，不等同于会话已退出登录。
- 真实设置修改、结束和资金核对由用户本人测试，不以合成网络或界面可打开代替。
- 本批没有Rust源码修改；本日整仓Rust依赖构建脚本0xc0000005失败未被本批Android验证覆盖。
