# 一龙开放商业与 AI 经济能力包

本目录是开放商业方向的稳定入口。它不替代现有架构决策、API 契约或实施草案，只负责说明这些材料之间的关系，并防止 AI 把“已经实现”“已接受但仍在扩展”和“尚处于提案阶段”的内容混在一起。

## 一句话定位

一龙不是再做一个封闭式美团，而是在现有 AI 应用开发、群体协作和共享节点能力之上，为商户建立可自主控制的数据与 AI 经营节点，使经过授权的消费者 AI、第三方应用和商户应用能够发现、调用并交易商业能力。

## 当前阅读顺序

1. `docs/open-commerce/capability-baseline.md`：先确认项目已经具备什么、还缺什么。
2. `docs/open-commerce/integration-architecture.md`：理解应用开发、群体 AI、开放商业、共享资源和 Sui 提案如何共用一套主干。
3. `docs/decisions/open-commerce-network-principles.md`：查看已经接受的产品原则。
4. `docs/decisions/open-commerce-network-v1-architecture.md`：查看 V1 已接受的架构边界。
5. `docs/open-commerce-network-v1-api.md`：查看当前 HTTP 与 MCP 契约。
6. `docs/open-commerce/merchant-runtime.md`：查看商户自有 ERP 运行时、签名验证、报价与订单边界。
7. `docs/decisions/open-commerce-integration-control-plane.md`：查看多平台接入状态、同步回执和 AI 开发上下文边界。
8. `docs/erp/README.md`：查看通用 ERP 蓝图、独立商户实例、AI 通用提案和可回滚升级治理。
9. `docs/open-commerce/decision-register.md`：区分已接受决定、试验方向和未决问题。
10. `docs/decisions/task-shadow-settlement-v1.md`：查看链外影子经济层已经接受的事实来源、账本和 Sui 边界。
11. `docs/decisions/sui-offchain-projection-packages-v1.md`：查看 Sui 链下投影包、双摘要、目标网络和完整性复核边界。
12. `docs/decisions/task-shadow-settlement-disputes-v1.md`：查看争议案件、追加式事件和投影阻断边界。
13. `docs/decisions/task-shadow-settlement-corrections-v1.md`：查看纠正 Matter、原子冲销与替换、净额和普通 Sui 投影阻断边界。
14. `docs/decisions/sui-correction-projection-packages-v1.md`：查看冲销与替换共同组成链下原子包、摘要复核和网络未提交边界。
15. `docs/decisions/task-shadow-settlement-lineage-v1.md`：查看从任意凭证解析根、纠正步骤和当前有效凭证的只读边界。
16. `docs/open-commerce/connector-sdk.md`：查看厂商无关连接器契约、数据边界和兼容性门禁。
17. `docs/open-commerce/consumer-developer-sandbox.md`：查看消费者发现、App 注册、授权审批和测试调用闭环。
18. `docs/open-commerce/ai-resource-control.md`：查看 AI 资源盘点、项目策略和不执行任务的路由预演。
19. `docs/decisions/open-commerce-directory-publication-v1.md`：查看商户主动发布、脱敏目录和撤回边界。
20. `docs/open-commerce-directory-v1-acceptance.md`：查看目录 V1 的验证证据和未完成边界。
21. `docs/decisions/open-commerce-developer-lifecycle-v1.md`：查看沙盒 App 停用、重新启用和授权申请撤回边界。
22. `docs/open-commerce-developer-lifecycle-v1-acceptance.md`：查看开发者生命周期 V1 的验证证据。
23. `docs/decisions/open-commerce-rate-limits-v1.md`：查看商户可控调用配额、幂等和超限审计边界。
24. `docs/open-commerce-rate-limits-v1-acceptance.md`：查看调用配额 V1 的验证证据。
25. `docs/decisions/open-commerce-app-blocks-v1.md`：查看商户级 App 封禁、授权紧急撤销和解除后不恢复信任的边界。
26. `docs/open-commerce-app-blocks-v1-acceptance.md`：查看商户级 App 封禁 V1 的验证证据。
27. `docs/decisions/open-commerce-grant-budgets-v1.md`：查看单个 Grant 总调用和总计量预算的原子预留边界。
28. `docs/open-commerce-grant-budgets-v1-acceptance.md`：查看授权生命周期预算 V1 的验证证据。
29. `docs/decisions/open-commerce-grant-expiration-v1.md`：查看商户限时授权、安全默认值和到期失败关闭边界。
30. `docs/open-commerce-grant-expiration-v1-acceptance.md`：查看 Grant 限时授权 V1 的验证证据。
31. `docs/decisions/open-commerce-consumer-relationships-v1.md`：查看消费者自主建立、限时和撤销匿名商户关系的边界。
32. `docs/open-commerce-consumer-relationships-v1-acceptance.md`：查看消费者关系凭证 V1 的验证证据。
33. `docs/decisions/open-commerce-consumer-relationship-renewal-v1.md`：查看匿名关系续期、别名轮换和幂等后继边界。
34. `docs/open-commerce-consumer-relationship-renewal-v1-acceptance.md`：查看关系安全续期与临期提醒 V1 的验证证据。
35. `docs/decisions/open-commerce-consumer-data-erasure-requests-v1.md`：查看删除请求、关系原子撤销和商户声明边界。
36. `docs/open-commerce-consumer-data-erasure-requests-v1-acceptance.md`：查看消费者删除请求 V1 的验证证据。
37. `docs/decisions/open-commerce-consumer-portability-exports-v1.md`：查看消费者关系、续期链与删除请求的不可变导出边界。
38. `docs/open-commerce-consumer-portability-exports-v1-acceptance.md`：查看消费者可携带数据包 V1 的验证证据。
39. `docs/decisions/open-commerce-consumer-preference-disclosures-v1.md`：查看消费者私有偏好档案、字段级快照和关系失效边界。
40. `docs/open-commerce-consumer-preference-disclosures-v1-acceptance.md`：查看消费者偏好档案与商户匿名披露的验证证据。
41. `docs/decisions/open-commerce-capability-contract-enforcement-v1.md`：查看能力输入输出 Schema 的有限支持、失败关闭和隐私边界。
42. `docs/open-commerce-capability-contract-enforcement-v1-acceptance.md`：查看能力契约强制执行 V1 的验证证据。
43. `docs/decisions/open-commerce-consumer-invocation-receipts-v1.md`：查看消费者账户级终态调用凭证、本人归属和摘要边界。
44. `docs/open-commerce-consumer-invocation-receipts-v1-acceptance.md`：查看消费者调用凭证 V1 的验证证据。
45. `docs/decisions/open-commerce-invocation-recovery-v1.md`：查看孤儿调用失败关闭、Grant 预算回收和迟到结果边界。
46. `docs/decisions/open-commerce-app-activity-health-v1.md`：查看外部 App 近 24 小时可解释调用证据和人工处置边界。
47. `docs/open-commerce-app-activity-health-v1-acceptance.md`：查看 App 调用活动证据 V1 的验证结果和未完成范围。
48. `docs/decisions/node-compute-sharing-supply-v1.md`：查看节点所有者显式开放模型算力及原子调度边界。
49. `docs/decisions/node-compute-sharing-token-reservation-v1.md`：查看每日实耗、活动预留和本次请求共同受预算约束的边界。
50. `docs/decisions/node-compute-sharing-runtime-health-v1.md`：查看所有者运行健康快照、告警和非经济处置边界。
51. `docs/decisions/node-compute-sharing-expired-run-reconciliation-v1.md`：查看过期执行终结、预授权回收和非终态状态边界。
52. `docs/node-compute-sharing-supply-v1-acceptance.md`：查看节点模型共享 V1 的验证证据和未完成范围。
53. `docs/node-compute-sharing-supply-v1-api.md`：查看策略字段、状态码和发现/调用契约。
54. `docs/decisions/open-commerce-schema-driven-invocation-form-v1.md`：查看消费者 PC 按能力契约填写、动作确认和失败关闭边界。
55. `docs/open-commerce-schema-driven-invocation-form-v1-acceptance.md`：查看 Schema 驱动调用表单 V1 的验证证据。
56. `docs/decisions/open-commerce-server-action-confirmation-v1.md`：查看动作能力服务端两阶段确认、输入绑定、一次消费和可信边界。
57. `docs/open-commerce-server-action-confirmation-v1-acceptance.md`：查看服务端动作确认 V1 的迁移、事务、MCP 与 PC 验收证据。
58. `docs/decisions/open-commerce-consumer-portability-exports-v2.md`：查看低敏偏好档案与历史披露进入本人数据包、旧 V1 摘要兼容的边界。
59. `docs/open-commerce-consumer-portability-exports-v2-acceptance.md`：查看消费者可携带数据包 V2 的兼容性、事务和 PC 验收证据。
60. `docs/decisions/open-commerce-consumer-portability-exports-v3.md`：查看账户级调用凭证进入本人数据包、内外双层摘要和 V1/V2 兼容边界。
61. `docs/open-commerce-consumer-portability-exports-v3-acceptance.md`：查看消费者可携带数据包 V3 的事务、隐私、兼容和 PC 验收证据。
62. `docs/decisions/open-commerce-developer-terminal-events-v1.md`：查看开发者 App 终态调用事件、游标隔离和外部通知边界。
63. `docs/open-commerce-developer-terminal-events-v1-acceptance.md`：查看开发者终态事件流 V1 的迁移、隔离和隐私验证证据。
64. `docs/decisions/open-commerce-merchant-business-evidence-v1.md`：查看商户业务回执、调用证据、ERP 关联和真实经营事实边界。
65. `docs/open-commerce-merchant-business-evidence-v1-acceptance.md`：查看商户业务证据的 HTTP、MCP、PC 与失败关闭验证范围。
66. `docs/decisions/open-commerce-business-handoff-receipts-v1.md`：查看业务证据到 ERP/CRM 的显式处理声明、幂等和事实层级。
67. `docs/open-commerce-business-handoff-receipts-v1-acceptance.md`：查看衔接回执的摘要绑定、权限、隔离和失败关闭验证。
68. `docs/decisions/open-commerce-business-handoff-queue-v1.md`：查看从证据和最新回执派生待处理、需重试状态的规则。
69. `docs/open-commerce-business-handoff-queue-v1-acceptance.md`：查看待衔接队列的 HTTP、MCP、PC 和生命周期验证。
70. `docs/decisions/open-commerce-adapter-machine-credentials-v1.md`：查看接入器受限机器身份、一次性 Token、轮换和回执权威边界。
71. `docs/open-commerce-adapter-machine-credentials-v1-acceptance.md`：查看接入器机器凭据的鉴权、版本固化和 PC 验收证据。
72. `docs/decisions/open-commerce-adapter-credential-expiration-v1.md`：查看机器凭据 1–366 天服务端有效期和到期失败关闭规则。
73. `docs/open-commerce-adapter-credential-expiration-v1-acceptance.md`：查看旧凭据回填、到期鉴权和 PC 临期提醒证据。
74. `docs/decisions/open-commerce-adapter-handoff-claims-v1.md`：查看接入器显式任务领取权、受硬期限约束的续租、主动释放、拒绝退避、暂停恢复和原子完成规则。
75. `docs/open-commerce-adapter-handoff-claims-v1-acceptance.md`：查看已落位实现和统一验证待执行范围。
76. `docs/decisions/open-commerce-developer-webhooks-v1.md`：查看开发者 App 签名 Webhook、回调白名单、耐久投递、重试和失败关闭边界。
77. `docs/open-commerce-developer-webhooks-v1-acceptance.md`：查看 Webhook V1 已形成代码和统一回归待验证范围。
78. `docs/decisions/open-commerce-developer-webhook-verification-v1.md`：查看签名 challenge、精确回显和验证后激活边界。
79. `docs/open-commerce-developer-webhook-verification-v1-acceptance.md`：查看回调验证代码及统一回归待验证范围。
80. `docs/decisions/open-commerce-developer-webhook-secret-rotation-v1.md`：查看订阅级签名密钥版本、显式轮换和重新验证边界。
81. `docs/open-commerce-developer-webhook-secret-rotation-v1-acceptance.md`：查看密钥轮换代码及统一回归待验证范围。
82. `docs/decisions/open-commerce-developer-webhook-dead-letter-retry-v1.md`：查看单条死信原地重新排队和人工重试证据边界。
83. `docs/open-commerce-developer-webhook-dead-letter-retry-v1-acceptance.md`：查看死信人工重试代码及统一回归待验证范围。
84. `docs/decisions/open-commerce-developer-webhook-event-filter-v1.md`：查看成功与失败终态事件在入队前筛选的边界。
85. `docs/open-commerce-developer-webhook-event-filter-v1-acceptance.md`：查看事件筛选代码及统一回归待验证范围。
86. `docs/decisions/open-commerce-developer-webhook-history-replay-v1.md`：查看最多 100 条历史终态通知的有界、幂等补发边界。
87. `docs/open-commerce-developer-webhook-history-replay-v1-acceptance.md`：查看历史补发代码及统一回归待验证范围。
88. `docs/decisions/open-commerce-developer-app-manifest-review-v1.md`：查看 App 资料修订、提交、审核和非生产准入边界。
89. `docs/open-commerce-developer-app-manifest-review-v1-acceptance.md`：查看 App 资料审核代码及统一回归待验证范围。
90. `docs/decisions/open-commerce-developer-app-domain-verification-v1.md`：查看主页域名 challenge、精确白名单和证明边界。
91. `docs/open-commerce-developer-app-domain-verification-v1-acceptance.md`：查看域名控制证明代码及统一回归待验证范围。
92. `docs/decisions/open-commerce-outbound-public-address-pinning-v1.md`：查看 DNS 私网拒绝、本次地址固定和出站边界。
93. `docs/open-commerce-outbound-public-address-pinning-v1-acceptance.md`：查看出站公网地址固定代码及统一回归待验证范围。
94. `docs/decisions/open-commerce-merchant-runtime-egress-pinning-v1.md`：查看商户运行时调用阶段白名单复核和地址固定边界。
95. `docs/open-commerce-merchant-runtime-egress-pinning-v1-acceptance.md`：查看运行时出站安全代码及统一回归待验证范围。
96. `docs/decisions/open-commerce-developer-app-admission-v1.md`：查看主体声明、人工风险层级和可撤销准入边界。
97. `docs/open-commerce-developer-app-admission-v1-acceptance.md`：查看 App 准入审查代码及统一回归待验证范围。
98. `docs/decisions/open-commerce-developer-production-credentials-v1.md`：查看默认关闭、一次性显示、能力限权和紧急撤销的生产凭据边界。
99. `docs/open-commerce-developer-production-credentials-v1-acceptance.md`：查看生产凭据代码状态与后置统一回归清单。
100. `docs/decisions/open-commerce-invocation-credential-provenance-v1.md`：查看调用来源证据、幂等边界和沙箱/生产隔离决定。
101. `docs/open-commerce-invocation-credential-provenance-v1-acceptance.md`：查看调用环境隔离代码状态与统一回归清单。
102. `docs/decisions/open-commerce-production-webhooks-v1.md`：查看环境绑定生产订阅、双开关和生产资格联动决定。
103. `docs/open-commerce-production-webhooks-v1-acceptance.md`：查看生产 Webhook 代码状态与后置统一回归清单。
104. `docs/decisions/open-commerce-webhook-operational-health-v1.md`：查看只读运行健康、环境聚合和生产阻断码决定。
105. `docs/open-commerce-webhook-operational-health-v1-acceptance.md`：查看 Webhook 健康代码状态与后置统一回归清单。
106. `docs/decisions/open-commerce-webhook-dead-letter-acknowledgement-v1.md`：查看死信人工确认、不可覆盖处理证据和健康告警收口决定。
107. `docs/open-commerce-webhook-dead-letter-acknowledgement-v1-acceptance.md`：查看死信确认代码状态与后置统一回归清单。
108. `docs/decisions/open-commerce-developer-production-readiness-v1.md`：查看 App 生产就绪步骤、稳定阻断顺序和只读派生边界。
109. `docs/open-commerce-developer-production-readiness-v1-acceptance.md`：查看生产就绪总览代码状态与后置统一回归清单。
110. `docs/decisions/sui-adapter-offline-handoff-v1.md`：查看标准与纠正投影的统一离线适配器交接契约。
111. `docs/sui-adapter-offline-handoff-v1-acceptance.md`：查看交接包代码状态与后置统一回归清单。
112. `docs/decisions/sui-offline-preflight-adapters-v1.md`：查看离线预检机器身份、摘要匹配和追加式报告边界。
113. `docs/sui-offline-preflight-adapters-v1-acceptance.md`：查看预检适配器代码状态与后置统一回归清单。
114. `docs/decisions/sui-offline-preflight-job-leases-v1.md`：查看显式预检任务、短时租约、漂移阻断和单事务完成边界。
115. `docs/sui-offline-preflight-job-leases-v1-acceptance.md`：查看预检任务租约的代码状态与后置统一回归清单。
116. `docs/decisions/open-commerce-consumer-portability-selective-adoption-v1.md`：查看低敏偏好字段级采用、当前值保留和快照回滚决定。
117. `docs/open-commerce-consumer-portability-selective-adoption-v1-acceptance.md`：查看字段级采用代码状态与后置统一回归清单。
118. `docs/decisions/open-commerce-consumer-data-erasure-evidence-v1.md`：查看删除完成声明之外的追加式外部证明与未核验边界。
119. `docs/open-commerce-consumer-data-erasure-evidence-v1-acceptance.md`：查看 V160 证明账本代码状态与后置统一回归清单。
120. `docs/decisions/open-commerce-consumer-portability-exports-v5.md`：查看删除证明进入 V5 数据包及 V1-V4 摘要兼容决定。
121. `docs/open-commerce-consumer-portability-exports-v5-acceptance.md`：查看 V5 代码状态与后置统一回归矩阵。
122. `docs/decisions/open-commerce-consumer-portability-multi-source-merge-v1.md`：查看多来源低敏偏好冲突预演、逐字段来源选择和安全回滚决定。
123. `docs/open-commerce-consumer-portability-multi-source-merge-v1-acceptance.md`：查看多来源偏好合并代码状态与后置统一回归矩阵。
124. `docs/decisions/open-commerce-consumer-data-vault-v1.md`：查看消费者客户端加密、平台不透明密文托管和不可找回边界。
125. `docs/open-commerce-consumer-data-vault-v1-acceptance.md`：查看数据保险箱代码状态与后置统一回归矩阵。
126. `docs/decisions/open-commerce-pluggable-ranking-v1.md`：查看消费者选择五种透明非付费排序器、稳定并列条件和候选集边界。
127. `docs/open-commerce-pluggable-ranking-v1-acceptance.md`：查看可替换排序器代码状态与后置统一回归矩阵。
128. `docs/decisions/open-commerce-consumer-data-request-followups-v1.md`：查看删除请求内部运营目标、有界催办、升级关注和非法律边界。
129. `docs/open-commerce-consumer-data-request-followups-v1-acceptance.md`：查看 V163 跟进代码状态与后置统一回归矩阵。
130. `docs/decisions/open-commerce-consumer-ranking-receipts-v1.md`：查看单次排序凭证、输入隐私、摘要完整性和未签名边界。
131. `docs/open-commerce-consumer-ranking-receipts-v1-acceptance.md`：查看排序凭证代码状态与后置统一回归矩阵。
132. `docs/decisions/open-commerce-public-data-provenance-v1.md`：查看商户声明来源、新鲜度派生和未外部核验边界。
133. `docs/open-commerce-public-data-provenance-v1-acceptance.md`：查看来源与新鲜度代码状态及后置统一回归矩阵。
134. `docs/decisions/open-commerce-consumer-freshness-filter-v1.md`：查看消费者主动筛选声明期内能力和默认不隐藏边界。
135. `docs/open-commerce-consumer-freshness-filter-v1-acceptance.md`：查看声明期筛选代码状态及后置统一回归矩阵。
136. `docs/decisions/open-commerce-capability-source-link-v1.md`：查看能力与内部同步回执绑定、能力版本失效和未外部核验边界。
137. `docs/open-commerce-capability-source-link-v1-acceptance.md`：查看 V164 来源绑定代码状态及后置统一回归矩阵。
138. `docs/decisions/open-commerce-consumer-source-requirement-v1.md`：查看消费者主动要求内部回执来源和默认不隐藏边界。
139. `docs/open-commerce-consumer-source-requirement-v1-acceptance.md`：查看来源筛选代码状态及后置统一回归矩阵。
140. `docs/decisions/open-commerce-consumer-source-filters-v1.md`：查看消费者按厂商和数据域精确过滤及未外部核验边界。
141. `docs/open-commerce-consumer-source-filters-v1-acceptance.md`：查看来源字段过滤代码状态及后置统一回归矩阵。
142. `docs/decisions/open-commerce-consumer-source-age-v1.md`：查看消费者最大内部回执年龄筛选及未外部核验边界。
143. `docs/open-commerce-consumer-source-age-v1-acceptance.md`：查看回执年龄筛选代码状态及后置统一回归矩阵。
144. `docs/decisions/open-commerce-consumer-source-filter-options-v1.md`：查看当前目录候选中的来源筛选建议和非穷尽边界。
145. `docs/open-commerce-consumer-source-filter-options-v1-acceptance.md`：查看来源建议代码状态及后置统一回归矩阵。
146. `docs/decisions/open-commerce-consumer-price-currency-v1.md`：查看币种安全价格筛选和兼容默认规则。
147. `docs/open-commerce-consumer-price-currency-v1-acceptance.md`：查看价格币种代码状态及后置统一回归矩阵。
148. `docs/decisions/open-commerce-consumer-capability-filters-v1.md`：查看能力类型与访问级别筛选及安全边界。
149. `docs/open-commerce-consumer-capability-filters-v1-acceptance.md`：查看能力筛选代码状态及后置统一回归矩阵。
150. `docs/decisions/open-commerce-consumer-preference-constraints-v1.md`：查看软偏好升级为硬约束的规则与边界。
151. `docs/open-commerce-consumer-preference-constraints-v1-acceptance.md`：查看偏好硬约束代码状态及后置统一回归矩阵。

## 专题地图

| 专题 | 状态 | 权威入口 |
|---|---|---|
| AI 应用开发与发布 | 已实现并持续演进 | `AI_PROJECT.md`、`AI_ARCHITECTURE.md` |
| 多人、多 AI 协同开发 | 已有实现，持续收口 | `docs/群体ai开发/群体AI开发功能需求与架构设计.md` |
| 商户节点、能力、授权、调用和审计 | V1 已接受并实现 | `docs/decisions/open-commerce-network-v1-architecture.md`、`docs/open-commerce-network-v1-api.md` |
| 商户自有 ERP 受控运行时 | 参考实现已完成，生产配置依赖环境 | `docs/open-commerce/merchant-runtime.md`、`docs/open-commerce-merchant-runtime-v1-acceptance.md` |
| 商户调用证据与 ERP/CRM 衔接 | V1 证据层、人工回执、派生待办队列、限时机器凭据及显式扩权的单任务租约代码已落位；租约支持受限原因主动释放并等待统一验证，生产入库适配器、官方授权、外部回读和签名证明待逐项实现 | `docs/decisions/open-commerce-merchant-business-evidence-v1.md`、`docs/decisions/open-commerce-business-handoff-receipts-v1.md`、`docs/decisions/open-commerce-business-handoff-queue-v1.md`、`docs/decisions/open-commerce-adapter-machine-credentials-v1.md`、`docs/decisions/open-commerce-adapter-credential-expiration-v1.md`、`docs/decisions/open-commerce-adapter-handoff-claims-v1.md` |
| 通用 ERP 蓝图与智能提案 | V1 已实现，代码执行仍走既有项目流程 | `docs/erp/README.md`、`docs/erp/acceptance-v1.md` |
| 商户数据来源、健康度和同步回执 | 控制面已实现；能力与合格内部回执的显式绑定代码已形成但未编译，具体平台适配器和外部核验待逐项验收 | `docs/decisions/open-commerce-integration-control-plane.md`、`docs/decisions/open-commerce-capability-source-link-v1.md`、`docs/open-commerce-integration-control-plane-acceptance.md`、`docs/open-commerce-capability-source-link-v1-acceptance.md` |
| 连接器 SDK 与兼容性门禁 | 已实现 V1，尚不包含具体大厂适配器 | `sdk/open-commerce-connector/`、`docs/open-commerce/connector-sdk.md` |
| 消费者发现、关系授权与第三方应用接入 | 跨项目目录、商户声明来源与新鲜度、能力与内部同步回执来源绑定、五种用户可选透明非付费排序器、可复核未签名单次排序凭证、匿名关系、低敏偏好披露、客户端加密敏感数据保险箱、删除请求、有界催办与升级关注、商户提交的未核验外部证明、V5 可验证导出、隔离导入、单包字段级采用、多来源偏好冲突选择、Schema 调用、动作确认、授权与预算、调用事件和封禁主干已形成；来源绑定、来源新鲜度、排序器、凭证、跟进、保险箱、证明、V5、多来源合并、生产准入与 Webhook 等批次未编译，生产能力默认关闭。外部平台签名与官方回读证明、运营方签名排序凭证、第三方排序器 SDK、自动删除通知与仲裁、密码找回、多设备密钥同步、商户或 AI 授权解密、完整订单迁移、外部删除执行与回执核验、跨运营方身份互认与生产公共网络仍未完成 | `docs/decisions/open-commerce-directory-publication-v1.md`、`docs/decisions/open-commerce-public-data-provenance-v1.md`、`docs/decisions/open-commerce-capability-source-link-v1.md`、`docs/decisions/open-commerce-pluggable-ranking-v1.md`、`docs/decisions/open-commerce-consumer-ranking-receipts-v1.md`、`docs/decisions/open-commerce-consumer-relationships-v1.md`、`docs/decisions/open-commerce-consumer-preference-disclosures-v1.md`、`docs/decisions/open-commerce-consumer-data-vault-v1.md`、`docs/decisions/open-commerce-consumer-data-request-followups-v1.md`、`docs/decisions/open-commerce-consumer-data-erasure-evidence-v1.md`、`docs/decisions/open-commerce-consumer-portability-exports-v5.md`、`docs/decisions/open-commerce-consumer-portability-imports-v1.md`、`docs/decisions/open-commerce-consumer-portability-selective-adoption-v1.md`、`docs/decisions/open-commerce-consumer-portability-multi-source-merge-v1.md`、`docs/decisions/open-commerce-developer-terminal-events-v1.md`、`docs/decisions/open-commerce-developer-production-credentials-v1.md`、`docs/decisions/open-commerce-production-webhooks-v1.md` |
| AI 资源盘点、策略与路由预演 | 控制面已实现，尚未接管真实任务调度 | `docs/open-commerce/ai-resource-control.md`、`server/src/ai_resource_control/` |
| API Token 保管与节点计算计量 | 已有实现，尚不是公开 Token 交易市场 | `docs/token消费统计.md`、`server/src/store/node_ledger.rs` |
| 节点模型算力显式共享 | V1 已实现，包含模型白名单、并发、每日预算、所有者健康快照和过期执行回收；尚不是完整算力市场 | `docs/decisions/node-compute-sharing-supply-v1.md`、`docs/decisions/node-compute-sharing-runtime-health-v1.md`、`docs/decisions/node-compute-sharing-expired-run-reconciliation-v1.md`、`docs/node-compute-sharing-supply-v1-acceptance.md` |
| 链外影子用量、验收后双分录、争议纠正和 Sui 投影包 | V1 已实现、默认关闭；纠正可追加冲销与替换、解析当前有效凭证并保存链下原子包；标准包与纠正包可导出统一离线预检交接文件，并由限权机器身份领取短时任务、阻断摘要漂移和追加预检报告。所有包、任务和报告仍不签名、不提交网络、不移动资金 | `docs/decisions/task-shadow-settlement-v1.md`、`docs/decisions/task-shadow-settlement-corrections-v1.md`、`docs/decisions/task-shadow-settlement-lineage-v1.md`、`docs/decisions/sui-correction-projection-packages-v1.md`、`docs/decisions/sui-adapter-offline-handoff-v1.md`、`docs/decisions/sui-offline-preflight-adapters-v1.md`、`docs/decisions/sui-offline-preflight-job-leases-v1.md`、`sdk/open-commerce-connector/src/sui-preflight.js`、`docs/task-shadow-settlement-v1-api.md` |
| 商户 AI 经营、营销内容和业务应用生成 | 产品方向，按真实连接器逐步实现 | `docs/drafts/requirements/open-commerce-network.md` |
| 消费者 AI 与任意商户节点互联 | 商户主动发布、跨项目发现、消费者限时匿名关系、安全续期、低敏偏好字段披露、关联数据删除请求、有界催办与升级关注、商户提交的追加式未核验证明、含证明的 V5 可验证导出、Schema 驱动 PC 填写、App 授权、调用、配额、活动证据和手动封禁已形成；完整订单迁移、生产身份、自动外部通知、真实删除适配器、证明核验、平台仲裁、自动风控和联邦治理仍待完成 | `docs/open-commerce/consumer-developer-sandbox.md`、`docs/decisions/open-commerce-consumer-relationships-v1.md`、`docs/decisions/open-commerce-consumer-preference-disclosures-v1.md`、`docs/decisions/open-commerce-consumer-data-erasure-requests-v1.md`、`docs/decisions/open-commerce-consumer-data-request-followups-v1.md`、`docs/decisions/open-commerce-consumer-data-erasure-evidence-v1.md`、`docs/decisions/open-commerce-consumer-portability-exports-v5.md`、`docs/decisions/open-commerce-consumer-invocation-receipts-v1.md`、`docs/decisions/open-commerce-schema-driven-invocation-form-v1.md`、`docs/drafts/open-commerce-network-roadmap.md` |
| 闲置算力公开市场 | 模型推理供给控制已实现；异构任务、竞价、赔付和真实结算仍是提案 | `docs/decisions/node-compute-sharing-supply-v1.md`、`docs/drafts/open-commerce-network-sui-agent-economy.md` |
| Sui 链上结算、网络资产和收入权益 | 提案；当前仅有未提交网络的链外信封和可复核投影包 | `docs/drafts/open-commerce-network-sui-agent-economy.md`、`docs/decisions/sui-offchain-projection-packages-v1.md` |

## 文档治理边界

- `docs/decisions/` 只记录已经接受的决定。
- `docs/open-commerce/` 记录稳定的能力地图、融合架构和决策状态，不承诺未实现功能。
- `docs/drafts/` 保存需求、路线图、Sui 和经济层提案，默认不作为当前实现事实。
- `docs/inbox/conversations/` 保存讨论来源或讨论摘要，只用于追溯，不直接定义产品。
- 功能是否已经实现，以实现引用、测试和验收记录为准，不能只依据讨论文案判断。

## 长期维护规则

新增能力先更新 `capability-baseline.md` 的状态和证据，再更新对应专题文档。跨领域变化只在 `integration-architecture.md` 记录连接关系，详细协议仍放在各自模块中。单篇文档接近模块化审查阈值时，优先建立目录入口并按职责拆分，不继续扩张为巨型总文档。
