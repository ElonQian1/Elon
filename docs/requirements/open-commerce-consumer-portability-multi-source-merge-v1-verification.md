# 消费者多来源偏好合并 V1 验证闭环需求

## 目标

把已经存在的消费者多来源偏好合并 V1 从 `implementation_uncompiled` 推进为有当前证据的可交付能力。验证范围覆盖冲突预演、人工逐字段选择、原子采用、安全回滚、HTTP 权限、历史数据库升级、PC 工作台合同和既有单包采用兼容；验证发现缺陷时只修复该能力边界内的问题。

## 验收标准

1. 只允许当前消费者选择 2 至 10 个本人项目内、未删除且包含偏好档案的隔离导入包；重复、越权、跨项目和无偏好来源失败关闭。
2. 预演固定展示五个允许字段的当前值、全部来源候选、信任状态、差异和冲突数量，不自动裁决冲突，也不恢复关系或写入商业系统。
3. 用户必须逐字段明确选择来源并确认采用；未知字段、重复字段、未参与来源、无变化值和过期档案修订失败关闭。
4. 采用在单一事务中保存采用前快照、全部参与包、逐字段来源摘要和结果修订；审计只保存身份与摘要，不写入偏好明文。
5. 无后续修改时可恢复原档案或删除本次创建的档案；合并后修订已变化、重复回滚或其他用户回滚必须拒绝覆盖。
6. 从 V141 磁盘数据库升级后，旧偏好和单包采用记录保持可读，V161 多来源表及索引只创建一次，完整迁移可重复打开。
7. HTTP 路由覆盖未登录、非项目成员、同项目其他成员、跨项目、采用、列表和回滚；响应不泄露其他消费者导入包或偏好。
8. 既有单包选择性采用回归继续通过；PC 多来源选择、预演、逐字段来源、采用历史和回滚入口具备静态交互合同并通过 TypeScript 生产构建。

## 非目标

- 不自动决定冲突字段，不跨设备代替用户审批。
- 不恢复消费者关系，不迁移订单、支付、ERP 或其他经营数据。
- 不接入外部平台，不执行真实资金、链上资产或生产部署。
- 不把静态 PC 合同或本机测试表述为真实浏览器、公网或生产数据库验收。

## 实现范围

- `server/src/open_commerce_portability_merge_*`
- `server/src/store/open_commerce_consumer_portability_merges.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityMergePanel.tsx`
- `pc-frontend/src/features/open-commerce/openCommerceClientApi.ts`
- `pc-frontend/src/features/open-commerce/openCommerceClientTypes.ts`
- `scripts/test-open-commerce-portability-merge-pc.js`
- `docs/open-commerce-consumer-portability-multi-source-merge-v1-acceptance.md`
