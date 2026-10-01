# Win 网格分享 HTTP 兼容性与选币体验修复

日期：2026-10-01。范围：Win `/pc` 前端；沿用现有币安只读适配器和群快照协议。

## 根因与修复

实际 Win 前端使用 HTTP 源。读取持仓先调用安全上下文限定的 `crypto.randomUUID`，发送阶段又调用 `crypto.subtle.digest`，因此两个阶段都存在兼容性缺口。

- 请求 ID 复用 `uuid` 浏览器实现，以 `getRandomValues` 生成随机 UUID。
- 幂等键使用固定版本 `@noble/hashes` SHA-256，保持原有文档哈希语义；重试同一快照不会换键。
- 选择框增加本地 CC0 代币 Logo、缺失图标的彩色简称、方向标签、搜索、网格利润升降序和币种排序。
- 列表值来自现有 `gridProfit`，准确标注“网格利润”；选中后另读未实现盈亏，不把两者冒充策略总盈亏。
- 私有选择与收益预览不改变公开范围。默认不发送金额、数量、账户或策略编号；卡片明确显示“金额已隐藏”。

参考：[MDN randomUUID](https://developer.mozilla.org/en-US/docs/Web/API/Crypto/randomUUID)、[MDN getRandomValues](https://developer.mozilla.org/en-US/docs/Web/API/Crypto/getRandomValues)。

## 当前验证

- `node --test --test-reporter=spec pc-frontend/scripts/test-grid-share.cjs pc-frontend/scripts/test-grid-share-http.cjs`：9/9 通过。
- HTTP 能力受限环境验证：无 `randomUUID`、无 `subtle`，仍可读持仓、生成 1000 个唯一合法请求 ID、计算与 Node 一致的 SHA-256；覆盖重试键、错群回执、脱敏和精确小数排序。
- TypeScript 和 Vite 生产构建通过。
- 合成数据浏览器验收：Logo 加载、正负及缺失利润、排序、搜索、空匹配、默认隐藏金额及显式公开开关通过；390px 无横向溢出。
- Win 真实 QNT 发送验收：待本次前端发布后执行。用户已授权发送到指定当前群，保留隐藏金额和数量。

## 交付边界

implementation_status=implemented；verification_status=passed_local；delivery_status=pending；acceptance_status=pending_live。

本次未改后端或 APK。原跨平台功能的手机非空网格读取、发送验收仍需单独完成，不能由本次 Win 验收代替。
