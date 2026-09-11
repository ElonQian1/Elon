---
version_status: current
reviewed_at: 2026-09-12
---

# 创建网格 V2 只读经济参数

## 实现与来源

保留 V1 严格合同，增加 `create_reference_capabilities_v2`、`create_reference_v2`、`create_reference_poll_v2`。V2 投入参数可空，返回原动态范围、最低投入及 `profit_min/profit_max/quantity/quantity_unit/quantity_status`。调用方、操作、账号、文档、请求代次和行情有效期沿用旧边界；未增加金融执行入口。

独立数学实现复用原精确十进制、网格层级及权重；利润来自已取证模块 78387，普通/追踪数量来自 64290/30370，调用点为 2858/37403。最低投入仍使用价格 tick 精度，数量使用合约 pricePrecision；追踪数量为报价金额，不能标作基础币。行情只在追踪数量需要时额外读取单个合约 ticker，避免下载全合约列表。

来源页 SHA-256：`7e4ef73c9811d2b2eab572e43acdb409045e46c999f521a0f899196bce464747`；共同源模块的 URL/hash、合成输入与准确预期存于 `scripts/fixtures/binance-grid-create-economics-vectors.json`。原始压缩站点代码仅为本机研究产物，没有进入仓库。

## 验证状态

378 组合成数量/利润差分与原 336 组范围/最低投入差分通过；13 项 JS 行为检查覆盖动态依赖、V1兼容、生产适配器接线、缺失、账号变化和过期。12 个适配器套件及 64 项选定创建/宿主 JVM 测试通过。原签名发布及量化真实读取正在完成。

本批不提交、修改或终止真实策略，不改旧主创建 UI。视觉证据属于量化交付；未取得的实测仍为待验。
