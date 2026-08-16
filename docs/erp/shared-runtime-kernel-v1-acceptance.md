---
version_status: current
reviewed_at: 2026-08-16
implementation_status: implemented
verification_status: targeted_local_verified
---

# ERP 公共运行时内核 V1 验收

## 已验证

- `@yilong/merchant-erp-kernel` 以独立模块提供存储端口、内存参考适配器和无 UI 领域 API。
- 两个商户可在同一参考适配器中保持门店、商品、库存和订单隔离。
- 采购录入原子追加库存、采购记录和借贷相等的库存资产分录；重复请求精确重放，不同输入复用幂等键失败关闭。
- 下单按内核价格计算，库存不足时整笔回滚，成功订单固定为 `unpaid/awaiting_payment`，重放不再次扣库存。
- 开放商业 Provider 只暴露当前已启用模块的四项消费者能力，`order.create` 标记为动作能力。
- ERP `1.2.0` 样例发布清单绑定内核 `0.1.0`，物化合同透传可选运行时绑定；旧清单继续以 `runtime: null` 兼容。

## 当前边界

- `MemoryErpStore` 只用于测试和本地开发；可选 Node SQLite 参考适配器已单独完成事务、重启恢复、幂等、锁竞争和版本迁移验收，但仍不是多主或分布式生产数据库。
- 基础采购分录不等于完整财务核算、税务、付款或结账。
- 尚未把 `cofficethinking` 接入公共内核，也未验证真实商户迁移。
- 尚未发布 npm 包、部署商户运行时或执行消费者公网订单。

## 证据入口

- `sdk/merchant-erp-kernel/src/`
- `sdk/merchant-erp-kernel/test/`
- `docs/erp/sqlite-storage-adapter-v1-acceptance.md`
- `server/src/erp_blueprint/model.rs`
- `server/src/erp_blueprint/validation.rs`
- `server/src/erp_blueprint/materialization.rs`
- `examples/erp-blueprints/release-1.2.0.json`
