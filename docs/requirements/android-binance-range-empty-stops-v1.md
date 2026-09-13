---
version_status: current
reviewed_at: 2026-09-13
decision_status: accepted
owner: quant-grid
---

# 币安普通网格区间空保护兼容 V1

承接[量化G13需求](https://github.com/ElonQian1/yilong-quant/blob/main/docs/requirements/grid-range-empty-protection-v1.md)与Android会话托管V2。Win研究库的完整成功详情样本以空字符串表示未设置的金额/价格保护；旧区间解析拒绝这些值，导致普通网格区间快照缺失。当前手机仍须更新后验收，历史样本不作为实时账户数据。

仅调整固定区间合同：未设置或零金额保护允许读取；原可选价格保护的空字符串、null、字段缺失和合法精确值分别保留。真实非零金额保护、追踪网格、缺必填字段、非法类型/精度、账户或基线改变继续拒绝。JavaScript、Kotlin解析及离线准备/执行载荷须一致，空值不能扩大必填价格的合法范围。

量化拥有管理UI，主APK只托管会话和固定接口；跨APK投影没有新增字段，未设置的保护继续用既有null表示，摘要明确显示“未设置”，不把内部空字符串直接交给客户端的严格数字解析器。固定执行载荷仍保留提供方原值，不复制表单或凭据。补对应回归，原签名发布主包，在当前量化读取区间与只读准备后清理草稿；真实金融确认与效果对账仍由本人完成，不自动执行交易。
