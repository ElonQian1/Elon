---
version_status: current
reviewed_at: 2026-09-11
implementation_status: implemented
acceptance_status: pending
---

# 网格策略资金宿主合同交付

主 APK 为量化增加经过当前只读 grant 校验的报告能力查询与单个网格策略账户 USDT 保证金读取。需求、官网静态调用链和精确字段见[策略资金合同](requirements/android-binance-strategy-funds-v1.md)。量化 UI 属于独立子仓库，本批不扩建主应用的网格产品页面。

- 保留原有四种报告；只有确认新能力的消费者才请求 funds。
- 先验证策略详情与当前账号，再以策略账户 UID 构造固定查询；长 UID 按精确数字序列发送，不经浮点转换。
- 响应限定一个目标策略行，只有 USDT 及两个十进制 string/null 字段跨 APK。缺失、重复、不同币种失败关闭；读取前后核验身份，换号/撤销和迟到结果不得进入消费者。
- 50 项 JS 回归、36 项宿主 Android 单测与 Debug/Release 编译通过；发布、装机及真正账号响应仍分开记录。

目前仅官网公开源码证据，尚无本接口的账号响应验收。它不是交易所整个钱包、资金划转或真实交易验收。原签名发布后需在量化完成只读查询、字段核对、返回与失效清除。Win 本轮研究命令过期未执行，已登录窗口保留。
