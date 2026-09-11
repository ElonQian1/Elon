---
version_status: current
reviewed_at: 2026-09-12
---

# 量化创建资金参考宿主接口

需求：[固定只读资金合同](../requirements/android-binance-create-funds-v1.md)。新增独立JS读取模块和Kotlin结果解析，通过既有可信调用与创建操作提供capabilities/start/poll三种固定方法。主应用不扩展产品创建页；量化显示来源、更新时间与比例草稿。

先取得真实账户模式，普通账户读取getMaxWithdrawAmount的USDT金额，组合保证金分支只提取现货USDT free。请求前后核对当前账号；绑定文档、请求及代次，取消/换号/过期/迟到失效。整条读取30秒截止、ready有效60秒；异常不当零，凭据和其他资产不跨APK。只接受精确十进制字符串或安全整数，避免已失真小数number。

创建/宿主Debug编译与测试通过；资金/创建/动态参考32项JS、管理/追踪/报告兼容48项通过。新增测试涵盖请求接线、POST正文差异、两类账户分支、重复或缺失资产、金额精度、身份变化、取消/迟到/期限及严格结果合同。正式域测试入口已纳入新模块和JVM解析测试。发布、装机和实际接口返回分别记录，不以测试证明当前账号可用。

官网公开JS来源哈希、创建资金上限关系及实际验收集中在量化[本批交付记录](https://github.com/ElonQian1/yilong-quant/blob/main/docs/delivery/grid-create-funds-20260912.md)。本次不改金融请求体或最终确认，不重启浏览器，不持久保存私有余额。完整钱包、每格数量、强平估算与组合账户实测仍待完成；注册表保持implemented，整体目标继续推进。
