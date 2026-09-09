---
version_status: current
reviewed_at: 2026-09-09
---

# 网格创建选币页

原创建页只有输入一位后的自动补全，没有可浏览的完整列表。选币仍由主APK托管，量化APK继续使用既有创建入口，无需复制界面或登录信息。

新增原生选币页：搜索置顶，全部/自选/最近选择为第一层；交易所分类和名称/24h成交额/涨幅/跌幅排序为第二层；合约行展示币种、分类、最新价格、涨跌幅、USDT成交额与收藏按钮。最近选择只代表选择过，不代表交易过。收藏最多100个，最近选择最多12个，仅本机保存合约代码。

分类仅来自币安公开exchangeInfo的underlyingSubType，缺失为未分类，不硬编码AI/Meme等币种归属。[官方合约与24小时行情定义](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data)。只显示已验证的TRADING USDT永续合约，但账户能否创建该网格仍由原准备与确认合同决定。

列表独立于单币行情响应先显示。行情采用精确十进制；缺失/过期不当零，排序排在已知值之后。查询失败明确保留回退；换币清空价格类参数及杠杆，原准备结果继续失效。关闭表单后丢弃迟到回调，不触发订单。

## 验证与边界

- 实现：独立筛选模型、公开行情解码、原生选币页及创建表单接线。
- 离线：最终创建模块4个套件、24项Release单元测试通过（含新增搜索/分类/精确排序/收藏/最近/过期行情/换币测试），无失败；修改后的生产源码编译通过。正式签名发布构建通过。
- 发布与手机：主APK 1.1.1612（1612）已通过官方发布入口上线，服务器版本及SHA-256与本地正式包一致。本轮未安装到手机，不声称真实选币验收；量化APK没有改包，继续复用主APK创建界面。
- 网络：本机FAPI公开合约请求15秒超时。这不证明手机必然失败；也不能用离线夹具声称已取得真实分类与行情。
- 视觉：工作台已导入现有页面；自动识别全局APK/Web对齐能力。本模块为既有原生跨APK确认入口，没有新增Web交易页。发布后仅一次非真机Renderer准备在30秒内未返回；完成检查仍为DEBUG_RUNTIME_NOT_CONNECTED/PREPARATION_REQUIRED。按发布顺序规则记录VERIFICATION_DEFERRED，不重试或占用手机，不声称像素验收通过。
- 不包含币种全名/中文名外部数据库、静态热门名单、推荐收益评分或AI自动选币。AI/Meme等标签只有交易所实际返回时才出现；成交额与涨跌幅属于排序维度。

## 发布身份与验收状态

- 源码：`1d21a4d1b3c43f95c45e084a20cc1f11c584b044`，已推送main。
- APK SHA-256：`8e73e9502cd502b5c3747bdcf19b67ad14709b00509a71706a0035dba5bc816b`。
- 最终测试日志：`binance-symbol-picker-final-tests-20260909-200622-243`；发布日志：`binance-symbol-picker-release-20260909-201853-268`，退出码0，617秒。
- 手机此前的网格读取验收见[上批报告](binance-report-recovery-20260909.md)，不能冒充本批选币页证据。后续需用新主APK确认列表联网、滚动搜索、分类和收藏及换币回填。
- `FIT_RUN_STATUS=NOT_RUN`；`FINAL_VISUAL_LOSS=NOT_MEASURED`；`VISUAL_ACCEPTANCE_THRESHOLD=NOT_SET`；`CROSS_PLATFORM_VISUAL_PARITY=NOT_VERIFIED`。
- `BUSINESS_DELIVERY_READY=false`（UI工作台判定，视觉验收延期）；`PLATFORM_EVOLUTION_PENDING=false`；`EVOLUTION_THREAD=none`；`REAL_DEVICE_STATUS=NOT_REQUESTED`；`ANDROID_RENDERER=VERIFICATION_DEFERRED`。
- 功能登记保留implemented，发布成功与视觉/真实数据验收分别记录。统一仓库收尾结果另存交付目录。
