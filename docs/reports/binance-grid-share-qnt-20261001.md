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
- Win 真实 QNT 发送验收：2026-10-01 19:05（北京时间）已向用户明确指定的当前群发送一张卡片，保持金额、数量和个人说明不公开。
- 前端 `e9528b34ec6a1288bc8f6486e18a12c6fd7394b7` 已通过官方前端发布脚本原子发布，继续兼容后端 `0.3.1809 / 0919fb9660ab6f9acb23f1f3469cec576e3a1bc3`；Win MCP `reload_page` 回执 succeeded。
- 实际 Win 读取 38 个运行中网格，QNT 本地 Logo、利润降序列表、读取持仓及发送均成功，不再触发安全上下文 API 错误。
- MCP 从服务端群消息回读确认：`gmsg_6df220fbe3914fe7a4d0637d40fa7888`，修订 1，未撤回；快照 `ai_snapshot_87d342c265a6499ab2f17218c3a09d33`。本轮仅执行一次发送。
- 从已发送卡片打开详情成功；参数页可读，收益页将金额显示为“未公开”，没有总收益率时显示“未读取”。未操作币安交易、未修改账户、未触发群 AI 发言。
- 现场发现预览复选框被通用表单样式撑开，已在网格分享样式内限定复选框尺寸和文字对齐；不改变隐私开关语义。

## 交付边界

本次 Win 修复：implementation_status=implemented；verification_status=passed；delivery_status=published；acceptance_status=passed_live。

本次未改后端或 APK。原跨平台功能的手机非空网格读取、发送验收仍需单独完成，不能由本次 Win 验收代替。
