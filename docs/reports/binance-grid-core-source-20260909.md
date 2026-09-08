---
version_status: current
reviewed_at: 2026-09-09
implementation_status: partial
---

# 币安 U 本位网格 V2 来源与能力核对

## 证据方法

Win 已登录页面的研究会话读取实际加载的脚本资源、资源哈希和定向片段。重复检索在哈希一致的公开脚本缓存中完成。未重新启动 Win 或 Chrome，未导出登录凭据，未执行交易请求。以下偏移为本地 JavaScript 字符串索引；同一文件的 MCP 字节偏移可能因非 ASCII 字符不同。

| 资源 | SHA-256 | 使用位置 |
|---|---|---|
| page-ee55.148047e5.js | `7e4ef73c9811d2b2eab572e43acdb409045e46c999f521a0f899196bce464747` | 当前策略、挂单、配对记录、参数修改构造器 |
| 4818.ab7ba930.js | `24765919f67706cdb4e894bb491a4904e9000fa64efa5814ee808fd40fc0dcf2` | 人工创建构造器约 80100–82700 |
| 6206.f733299a.js | `38fc50b6fa632c7fd0d0fba304731ecad3d43eefb793054235aa891f59b3c512` | 历史网格原始字段及分页约 324000–328000 |
| fab4cd78.d1879f66.js | `0a743da7bdc9187d2b227de614a935dacd585f406fc21b279e4d305e99898b58` | module 82389、字节偏移 14498，策略账户持仓 |

## 固定只读合同

所有路径均在 `https://www.binance.com`。Cookie、请求头、账户原始标识及接口正文留在主 APK 的官方页面；消费者只取得版本化、限定字段的投影。

| 用途 | 请求 | 参数与响应 |
|---|---|---|
| 当前列表 | POST `/bapi/futures/v2/private/future/grid/query-open-grids` | 仅重用实际捕获的请求体；响应 data 数组，逐行 rootUserId 与精确登录 UID 核对 |
| 详情 | GET `/bapi/futures/v1/private/future/grid/query-grid-detail` | strategyId；当前列表或本账户已验证历史记录约束策略范围 |
| 历史 | POST `/bapi/futures/v2/private/future/grid/query-grid-history` | page、rows、startTime、endTime、可选 symbol；data.grids 与 data.total；端内每页 20 条 |
| 滑动窗口网格档位 | GET `/bapi/futures/v2/private/future/grid/query-grid-open-items` | strategyId，仅 detail.slideWindow=true；bidItems/askItems；PENDING 不等于委托 |
| 普通挂单 | POST `/bapi/futures/v1/private/future/strategy/streamer/um/open-orders` | 官网构造器 `{}`，返回后在宿主页筛选精确 strategyId 和 symbol |
| 配对成交 | POST `/bapi/futures/v1/private/future/grid/query-grid-matched-items` | strategyId、page、rows；data 数组与顶层 total；保留 matchedSeq 及两侧成交 |
| 策略持仓 | POST `/bapi/futures/v1/private/future/strategy/user-data/get-future-user-positions` | 官网构造器 `{}`；以已核验详情的 strategyUserId 选取账户，再筛选 symbol/BOTH，不能用登录 UID 替代策略账户 UID |

历史和记录请求前后各核验一次登录身份。后到的旧请求、账号变化、撤销、超时、错误响应不会覆盖成功新数据。未知合同显示读取失败，不能伪装成空列表。记录投影为 `yilong.binance_report.v1`，与当前列表 V2 分开，避免历史覆盖运行中列表。

## 创建覆盖及实测边界

支持本轮自有资金技术测试的做多、做空、中性、逐仓/全仓、杠杆、区间、格数、等差/等比、立即建仓、终止处理。高级参数增加触发价及最新/标记价格、价格或金额/收益率止盈止损、止盈止损时平仓、追踪上下限、自动追加保证金。

- 中性不发送 autoInitPos；追踪网格使用 QUOTE 且不发送 slideWindow。
- 金额止损使用正数；收益率换算按当前人工构造器的 USDT 两位小数、止盈向下/止损向上取整，确认页展示最终金额。来源还包括 page module 24012 的输入范围及 module 53944 的换算。
- `tpslCps` 在官网受功能开关影响；源码支持不代表所有账户均获交易所开放。实际服务端受理仍须本人验收。
- 行情与合约规则读取官方公开 `fapi/v1/exchangeInfo`、`fapi/v1/premiumIndex`。价格步长按 tickSize 检查，间距预览不计算收益或承诺委托数量。
- 新参数没有本轮真实下单验收。MCP 和本轮自动测试均不点击交易提交。

## 尚未完成原生替代的部分

运行中修改区间/格数、追加策略投资、调整保证金和杠杆，以及高级设置编辑仍需进一步验证写入和回执合同，当前保留同一登录会话的全屏官网入口。已观察到 update-grid-range 与 update-grid-investment 构造器，但其新增投入、保留仓位、最低金额与运行态约束不能从静态接口名称推断。

持仓目前直接显示策略净数量、开仓价及逐仓钱包金额；尚未合成实时强平价、保证金率、净总收益及跨收益口径对账。没有完整 K 线图、资金费历史账单和 WebSocket 流。以上仍是明确缺口，不能将全屏官网或源码测试称为原生功能全部完成。

官方功能范围：[币安合约网格说明](https://www.binance.com/en/support/faq/detail/f4c453bab89648beb722aa26634120c3)，[追踪网格说明](https://www.binance.com/en/support/faq/detail/7a7bb22420404385991dee3a0930207d)。

## 当前验证

六组只读/创建/管理 JavaScript 测试共 58 项通过；分页时间窗修正后单独重跑记录组 10 项通过。量化 Android 118 项、前端 65 项和打包检查通过；主 APK 网格测试 68 项通过，Debug/Release 源码编译通过。Release lint 实际失败（23 errors / 981 warnings），23 项 error 均位于本批未改动的既有通知、语音、ChatGPT 等文件，网格目录没有 error；没有把全仓 lint 记成通过。

正式主 APK `1.1.1587 (1587)` 已发布并安装，来源提交 `7fa8521aa80107695f7f3f6a33bf76a64ac05695`，APK SHA-256 为 `0c186331fcf210feccc9d9b1a8478d108e1584ffcc5a7a172c417b013c6eba0f`，服务器已验核同一文件。量化 `0.6.0 (7)` 与后续横屏修复 `0.6.1 (8)` 均使用原签名安装，两个 PR 的 Android CI 均通过。

| 实机项目 | 结果 |
|---|---|
| 当前本人账户 | 主应用身份有效，币安子账户，当前列表 1 条，HTTP 200；不据此推断其他账户 |
| 持续同意 | 按钮为“同意并保持只读连接”，量化显示持续连接；进程恢复和撤销的本轮完整设备验收待补 |
| 历史网格 | 最近 30 天获得真实成功空响应，时间与分页状态正常；非空及多页设备证据待补 |
| 创建页 | 原生表单可滚动，NEARUSDT 公开价格、资金费率、价格步长、最低数量及最低名义价值已读取 |
| 横屏内容 | 发现固定操作栏挤空内容区，量化 0.6.1 已将操作栏、筛选和数据统一滚动；装机后被其他手机操作打断，修复后视觉效果尚未验收 |
| 视觉工作台 | 请求设备已由另一 Renderer 会话占用，返回 REAL_DEVICE_VERIFICATION_DEFERRED；停止重试，未创建平台进化任务，未宣称视觉通过 |
| 创建、修改与结束 | 本轮未执行任何真实交易，用户操作验收仍待完成 |

业务源码、正式主 APK 发布与量化签名装机已经落地，但功能注册仍为进行中。官网后备和原生功能分别计数；未知、失败、空数据、未验收不得互相替代。
