---
version_status: current
reviewed_at: 2026-09-08
implementation_status: implemented
acceptance_status: device_verified
---

# 管理页只读调试与空列表验收

需求：[只读调试V1](../requirements/android-binance-grid-read-debug-v1.md)；既有创建、管理、
正式安装和交易边界见[上一批验收](android-binance-grid-manage-v1-20260908.md)。

## 真实问题

手机MCP先确认子账户、有效账号证明和已核验空列表，HTTP 200，row_count=0，详情请求0。
用户本人新建网格后，同一手机子账户返回row_count=1，列表接入正常。旧版没有管理页
按钮点击/读取结果记录，不能仅凭列表响应声称某次按钮点击或策略详情已验收。

源码同时确认空列表时读取/检查按钮仍可能启用；初始空列表还会被表单去重提前返回，
使占位文案继续提示选择策略。这里是原生空态与可观测性问题，不是币安拒绝只读请求。

## 实现

- 区分未核验、已核验空列表、有数据未选择和已选择；空列表明确提示连接已成功，
  禁用依赖策略的读取和准备。原账号的未核对记录仍可查询，不要求它继续存在于列表。
- 固定工具binance_manage_read只支持status/select/read/reload。select仅接受当前列表
  的零基索引；没有策略ID、URL、JS、交易参数、准备、确认、提交或清理记录入口。
- 读取复用现有管理会话的身份、文档和策略核验。MCP记录序号、结果/原因、列表状态、
  选择/按钮状态与详情是否仍属于当前页面；不包含策略内容或凭据。页面失去前台、
  正在读取或已经准备交易时，不允许工具干扰；有未核对记录只能查询。
- binance_host_status增加独立manage_page，修正原先把管理页误标成创建页的问题。
  只读桥采用弱引用，页面关闭后不保留Activity；历史读取结果与当前可读性分开报告。
- 页面关闭时使用同一已登录Host的独立只读会话，reload可恢复账号与列表；只按索引
  选择当前账户已有策略。读取复用原检查器，取得共享槽后才临时绑定回调，完成/失败/
  超时释放，正在进行的原生创建或管理操作优先。没有本机交易记录写入或隐式准备。

## 验证

原创建13项、管理9项、只读会话18项、诊断5项及旧适配器19条断言通过。原生最终44项通过，
含新增9项空列表、选择、恢复、参数白名单、只读排他和读取结果测试；最终日志
binance-read-debug-final-test，耗时412.1秒。最终只读连接状态的接线由正式Release编译检查覆盖。

主APK 1.1.1577（1577）已正式发布，源码871485d06b185766204d130a5fa02e379d433db2，
本地产物与服务器SHA-256均为142d222caf81babde5d00e09cd49a4c4e24d3c55fce08a4b641d2a7595bfac7c。
正式发布日志binance-read-debug-publish，552.7秒。

首次安装因手机无线ADB离线而延期。2026-09-08晚用户接回USB后，确认两种连接对应同一
设备，恢复无线ADB并通过无线安装1.1.1577。实际安装base.apk的SHA-256与上述正式工件
一致；量化0.5.0（5）的已装包也再次核验，仍为上一批b0cf03541caa源码候选，SHA-256为
9ae75ea7313f04c2bf3984857c1316ca457e602698bb6b625284c3c376d2129a。

## 同一手机的真实验收

| 路径 | 观察结果 |
|---|---|
| 不打开管理页 | status/reload/select索引0/read/status完成，surface=host_read_only、row_count=1、read_outcome=verified、detail_current=true；完成后busy=false |
| 量化到管理页 | 量化管理入口可见并正常打开主APK管理页，page_open/page_resumed=true；未选择时读取与准备均禁用，选择当前列表索引0后读取可用 |
| 管理页MCP读取 | read_sequence=1、read_outcome=verified、detail_current=true、operation_phase=idle、unresolved=false；没有准备或提交交易 |
| 返回量化 | 固定返回按钮正常返回，量化明确显示“本次未提交管理操作” |
| 量化授权与列表 | 原生15分钟只读授权返回量化，来源显示1个网格，列表可见且来源新鲜 |
| 量化真实详情 | 点击首个网格，主端detail_count=1；量化显示“已包含币安详情响应”，方向、价格区间、网格数量、杠杆、间距、每格数量6类参数可见；不将可见字段个数解释为每个值均有上游数据 |
| 失效与恢复 | 验收期间旧列表超过5分钟后清除陈旧记录；重新加载官网后重新授权，恢复1个网格并取得详情。首次后台刷新未取得新鲜列表，前台官方重新加载后成功，未清除登录资料 |

上述观察均来自同一手机子账户：main_session_current=true、account_kind=sub，最终
list_verified=true、row_count=1、detail_count=1、active_grant_count=1。授权按原15分钟
合同到期。Win账号不是本批证据来源。

主端MCP负责读取回执；仅页面导航使用此前已获准的固定动作语义探针，本次安装成功，
验收后已卸载并确认包不存在。未导出界面层级、网站凭据、账号/策略ID或资产金额。
本批没有运行时代码修改，不重复构建；保留原44项原生和各脚本验证身份，不冒称本轮重跑。

结构化证据包括device-installed.json、device-read-result.json、device-manage-read-result.json、
device-return-manage.json、device-quant-final-approve-hosted.json、device-quant-detail-verified.json
及device-host-final.json，保存于本机同名功能工件目录。最后一次完整核验为北京时间23:04。

## 仍未覆盖

真实创建、修改设置、结束和挂单/仓位/资金核对仍由用户本人测试；用户自建网格不证明
它经过本项目创建接口。量化商店上传尚未完成，范围、格数等官网回退能力保持原状态。
无真实金融操作、Win/Chrome重启或登录资料清除。新MCP只读详情与双APK导航验收通过，
不等于整个实盘交易模块完成。
