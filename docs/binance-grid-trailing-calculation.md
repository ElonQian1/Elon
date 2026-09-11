---
version_status: current
reviewed_at: 2026-09-11
implementation_status: partial
---

# 币安追踪编辑计算与保留合同

本批是管理 V4 的规则和参数构造基础。两个新网页模块尚未加入生产资源加载，主服务 V4、公共输入读取、日志升级和量化 UI 尚未接线；没有发布新版 APK，没有调用真实交易接口。

## 取证和可重复验证

同一代次（4）缓存的公开资源经 URL、时间和 SHA-256 核对。运行纯函数的 55 个可达模块无冲突；执行环境无网络、浏览器、账号或凭据。

| 来源 | 作用 | 资源 SHA-256 |
|---|---|---|
| page-ee55.148047e5.js：59977、37403、64290、15323 | 编辑输入、每格报价量和追踪上限 | 7e4ef73c9811d2b2eab572e43acdb409045e46c999f521a0f899196bce464747 |
| common/1b35116a.3979569e.js：30370、48043 | 网格价格序列及中性权重 | 832e914a4a45ef74f6c710e27fd86232b9b9ea3a29660af8e13a7f597ef5695e |

[`binance-grid-trailing-vectors.json`](../scripts/fixtures/binance-grid-trailing-vectors.json) 保存 288 个独立官网计算结果：96 个上限、144 个报价量、48 个完整编辑范围。包含多空/中性、等差/等比、小币价格、不同精度、169 格窗口两侧、追加与扣减投入、非终止除法和极窄区间。公开样例由研究脚本生成，不是用户账户记录；测试不需要官网在线或导出其完整代码。

## 不可再混淆的输入

- 59977 的编辑表单把当前详情 gridLowerLimit/gridUpperLimit 传给函数参数 initialLowerLimit/initialUpperLimit；不能据参数名去猜另一个未取得的初始区间。
- 表单原始 qtyPerOrder 对 U 本位 QUOTE 策略取自 perGridQuoteQty，但实际传给上限的是 37403 重新计算的量，不能直接用保存的 perGridQuoteQty，更不能猜测详情存在 qtyPerOrder 字段。
- 37403 使用方向、累计投入、上下限、标记价、原触发价、格数、初始杠杆、adjustCoef、LOT_SIZE.stepSize 推导的数量精度、symbol.pricePrecision、最新价及 windowCount。工作中编辑停止价不修改触发价。
- adjustCoef 和 windowCount 来自固定公共 grid/coef 的 data 对应字段。规则模块要求明确数值；未取得时不把官网显示层的默认系数冒充已核验配置。
- 45337 的投入计算经 Number 输入、BigNumber 除法、toNumber、BigNumber 加法、toNumber；累计投入是未按显示精度取整的 investedMargin，不是 investedMarginDisplay。内部预览按已核验计算兼容；精确资产/收益账本仍使用既有独立领域规则。

## 舍入与边界

2617.px 是 16 位向零截取，ms 的等差步长显式使用 32 位；tu 默认 16 位四舍五入。计算包装保留最多 38 个十进制数字。GEO 使用官网相同的 JS pow/log，位移数量要能解析为 Kotlin Int；超过范围返回不可计算。上限先按符号精度截取，按网格步长回推，结果取 8 位，显示/校验上限再向下取符号精度。

用户输入的价格校验单独使用完整十进制定点值，不能沿用计算包装的 38 位截取；否则大整数价格尾部的不对齐小数会被吞掉。本批有失败复现与回归。上移价严格大于当前上限且不超过计算上限，下移价严格小于当前下限且不低于最小价，tick 余数必须为零。源码计算退化到无可填写范围时保持不可用，不补造价格。

## 新模块与接线边界

- `binance_grid_trailing_rules.js`：纯上限、每格量、编辑输入组合和精确价格校验；固定公共上下文同 symbol，观察时间不早于 20 秒、未来偏差不超过 5 秒。无网络、存储或账号访问。
- `binance_grid_trailing_contract.js`：运行中已开启追踪的快照和草稿。复用保护载荷构造，仅修改开启方向的停止价，保留价格/PNL保护、trigger、tpslCps、cos/cps、sharing、autoInitPos；禁止方向开关、额外字段和无变化载荷。
- 完整基线比较包括身份、状态、投入、区间、保护及原处理方式。它只是纯比较函数，不能代替会话适配器的重读、授权校验和实际写前检查。

## 下一批必须完成

1. 主端后台取得同 symbol 的公开规则、标记/最新价；保留 min/maxPrice、stepSize、pricePrecision 与源时间，不在 UI 线程联网。网页固定 coef 读取加入有效期及账号前后核验。
2. V4 读取/准备接入以上模块；规则来源和完整基线一起绑定。最终提交前重读并校验，任何未知结果不自动补发。
3. 管理 journal v5、旧客户端在途恢复守卫，以及量化 V4 表单、能力协商、草稿恢复和只读 MCP。
4. 双包原签名发布安装，真实读取/准备、页面与返回验收；最后由用户提交并由 AI 回读核对。G20 保持 in_progress。

验证入口：`scripts/test-binance-grid-create.ps1 -AdapterOnly`；默认不带此开关仍继续现有 Android 定向编译测试。本批只运行适配器验证，未以之声称 Native 编译或设备验收通过。
