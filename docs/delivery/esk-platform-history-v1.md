---
version_status: current
reviewed_at: 2026-09-04
---

# 正式 ESK 本人审核流水 V1 交付记录

需求真源：[完整审核流水分页 V1](../requirements/esk-platform-history-v1.md)。
注册功能：`esk-platform-history-v1`；旧账户、Paper 与双 APK 摘要协议不改。

## 四类状态

| 维度 | 当前证据 | 未完成项 |
| --- | --- | --- |
| implementation_status | implemented：只读 API、完整快照分页、独立原生页、Web 说明 | 正式占用/卖回不属此切片 |
| verification_status | Android 190 项、账本 harness 58 项、正式模块 Router 11 项通过；线上只读 smoke 通过 | 实际像素/生命周期待验收 |
| delivery_status | released：源码推送、Server 0.3.1725、PWA 与主 APK 1.1.1511 已发布 | 子 APK V28 上传不属本批发布 |
| acceptance_status | synthetic_verified，真实本人未验收 | 受保护主服务连接、真实付款审核映射与审批 |

## 用户能力与边界

主 APK：个人中心 → 正式 ESK 平台登记 → 查看完整审核流水。
每页显示全账户审核总额、总笔数和当前范围；可进入下一页或回到第一页。
空账户显示“暂无正式登记”，不会把未核对付款推定为不存在。

`GET /api/me/assets/esk/platform/history` 只读取认证本人。
游标绑定本人完整已验证账本的 SHA-256；新增、变更或失效锚点返回 409。
每次完整扫描只保留当前一页（最多 100 笔）；不是全历史内存缓存，也不宣称查询加速。
全账户摘要与页小计严格分开；不以游标或审核记录冒充链证明。

Android 安全来源检查先于凭据读取；会话世代、一次性请求和跨页边界都复核。
下一页开始即清旧页，后台/退出/保存/换号/超时清除数据，不落盘、不复制、不导出。
Web 仅同步说明与 APK 下载入口，不接收正式历史、游标或主账户凭据。

## 验证证据

- Android：`EskPlatformHistoryPageStateTest`、`EskPlatformHistoryWiringTest` 与
  `EskPlatformHistoryParserTest`、`EskPlatformHistoryParserBoundaryTest`、
  `EskPlatformHistoryReaderTest`；全部 ESK 与共享协议共 190 项通过。
- 旧账户 32 项解析测试保持通过；JSON 严格读取层的无行为变化搬迁独立提交。
- 第一轮合成时间倒序及旧 Web 状态断言失败后已修正并重跑；不以构建进程退出码
  代替测试报告，最终直接复核 XML 的 failures/errors 均为零。
- 本机 Java 使用本任务临时目录后可编译；只修改构建进程环境，没有系统级修复。
- 后端 harness 58 项通过（其中新增 16 项），验证指纹
  `386f4fad7c23664caf5019f79c4ac66933ff1215effddf3dc632fe176a952243`。
  真实 Router 新增 7 组，与既有 4 组共 11 项通过，验证指纹
  `17c59d081353064d0237ac950f447b6164616e8ef0a4ecf45bcc17f44e632703`。
- 独立源码审查未发现 P1/P2；不把源码审查当作设备或生产本人验收。

## 发布与恢复

功能源码：`13323a41d3faa2525b420cb703be0ff495cddf26`；JSON 纯搬迁父提交
`85c5d3a04336dd47785f18e34486072941ec79a4`，均已推送 `origin/main`。

| 已发布对象 | 身份与校验 |
| --- | --- |
| Server | `0.3.1725`，源码 `13323a41d3faa2525b420cb703be0ff495cddf26` |
| Mobile PWA | 同一源码；模板 SHA-256 `162108f2f8702fd7783bdb8b5b444ad989af7b539b87730291f0729ca7cd9771` |
| 主 APK | `1.1.1511 (1511)`，源码 `b1acb7433d1ad9533d444636b91c29dcd7f27824` |
| APK 字节 | `39683075` bytes；SHA-256 `95ff7a66ee683363849ef3c1117ff31fcafcefa8643fcc3d18ff13616ca89978` |
| APK 签名 | SHA-256 `f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`，与主 APK 信任身份一致 |

主 APK：[公开下载入口](http://43.139.149.158:8080/app/ElonSpeed-latest.apk)。
公开 latest 入口会随后续发布更新；以上版本、摘要与源码是本批交付快照。
官方 APK 发布器在服务端发布后快进到 `b1acb743…`，该提交直接继承功能源码，
只更新其他功能的 3 份文档，ESK/Android 源码未改变；不混写两个实际发布 SHA。
本地签名验证通过，已从编译后 Manifest 确认完整流水 Activity 存在且不对外导出。
发布器验证远端 APK 字节摘要和大小；独立只读下载流再次计算完整摘要和大小并一致，
未保存远端 APK 或读取私有数据。公开版本接口确认 `1511`；
服务端 `/health` 返回 200，未登录流水接口返回 401，携带 `no-store`、`no-cache`、
`no-referrer`，错误仅为“需要真实用户登录”，没有私有账户内容。

正式发布流水线结果 `passed`，耗时 865.4 秒；日志证据名
`esk-history-publish-full-identity-20260904-193553-130`，存于共享 `.git/ai-command-logs`。
官方发布入口负责版本、工件、上传与并发保护；不手改版本或替换生产账本。
当前 HTTP 公开源保持原样，私有读取仍失败关闭；未配置新证书或更改传输策略。
回退只需退回既有正式资产页/旧账户 API；本切片没有数据库迁移或资金写入。

## 视觉与本机状态

使用 Yilong UI 工作台导入并生成 Views Preview 骨架，复用主项目颜色与触控尺寸。
没有目标设计图；当前 Runtime 为 BOOTSTRAP、未连接，渲染设备列表为空。
默认未操作物理设备；真实像素、跨端视觉及生命周期验收为 deferred。
发布后重新运行 UI 完成检查：`PREPARATION_REQUIRED / DEBUG_RUNTIME_NOT_CONNECTED`，
不是已证实的平台能力缺口，不创建平台演进任务，也不恢复 PC 自动监督。

| UI 完成字段 | 实际结果 |
| --- | --- |
| FIT_RUN_STATUS | NOT_RUN / VERIFICATION_DEFERRED |
| FINAL_VISUAL_LOSS | unavailable |
| VISUAL_ACCEPTANCE_THRESHOLD | unavailable |
| CROSS_PLATFORM_VISUAL_PARITY | not_verified / deferred |
| BUSINESS_DELIVERY_READY | false（UI 工作台门禁；不否定上述实际发布） |
| PLATFORM_EVOLUTION_PENDING | false |
| EVOLUTION_THREAD | none |
| REAL_DEVICE_STATUS | not_requested / not_run |
| ANDROID_RENDERER | none / deferred |

仓库统一收尾在本报告与注册状态推送后执行，以 `finish-ai-task.ps1` 的最终回执为准。
不能用 UI 的待准备状态替代或伪造 `FINALIZABLE`；最终回复单独报告仓库收尾字段。

共享构建缓存 doctor：项目 `elon-cli`，安装/源码/启动器/Skill 一致，无活跃 Cargo 写者；
卷剩余空间约 8.74%（warning），未清理或删除任何缓存、未知文件或用户工作区。
主同步 checkout 原有已跟踪修改与未跟踪内容全部保留，仅在隔离任务根编辑。

## 下一入口

- 核对已付款用户与正式审核流程后，在批准的私有传输条件下做真实本人查账验收。
- 量化 V28 `0.4.0 (4)` 签名包已构建但未上传，仍需现有项目发布凭据；
  接收端源码 `e009c164bd04076d97d9481bf77b88110758fd62`，证据提交
  `72ed8eac1edd06f2be5e7ab08bf5c622600d55cf`，不由本批主项目发布代替子 APK 上传。
- 正式占用、官方卖回状态机另开受认领的交付切片，不直接复用 Paper 余额。
  建议仅做 ESK 数量申请、占用及取消释放，生产默认关闭，不隐含报价、自动成交或打款；
  待确认该范围，再登记独立需求与额度/条款，不擅自启用真实占用。
- Goal 继续有效；本批无真实入账、USDT 兑换、订单、链上签名或资金移动。
