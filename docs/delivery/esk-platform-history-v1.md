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
| verification_status | Android 190 项、账本 harness 58 项与完整正式模块路由测试通过 | 实际像素/生命周期待验收 |
| delivery_status | 本地源码与 JSON 纯搬迁提交已具备 | 功能推送、Server/APK 发布及线上只读 smoke 待完成 |
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
  真实 Router 新增 7 组并与既有正式模块一起通过，验证指纹
  `17c59d081353064d0237ac950f447b6164616e8ef0a4ecf45bcc17f44e632703`。
- 独立源码审查未发现 P1/P2；不把源码审查当作设备或生产本人验收。

## 发布与恢复

本批最终源码 SHA、Server/APK 版本和摘要在实际发布后登记，不能提前声称已可安装。
官方发布入口负责版本、工件、上传与并发保护；不手改版本或替换生产账本。
当前 HTTP 公开源保持原样，私有读取仍失败关闭；未配置新证书或更改传输策略。
回退只需退回既有正式资产页/旧账户 API；本切片没有数据库迁移或资金写入。

## 视觉与本机状态

使用 Yilong UI 工作台导入并生成 Views Preview 骨架，复用主项目颜色与触控尺寸。
没有目标设计图；当前 Runtime 为 BOOTSTRAP、未连接，渲染设备列表为空。
默认未操作物理设备；真实像素、跨端视觉及生命周期验收为 deferred。
最终 UI 完成检查与仓库 finish 字段在发布后登记，不用无截图状态冒充视觉通过。

共享构建缓存 doctor：项目 `elon-cli`，安装/源码/启动器/Skill 一致，无活跃 Cargo 写者；
卷剩余空间约 8.74%（warning），未清理或删除任何缓存、未知文件或用户工作区。
主同步 checkout 原有已跟踪修改与未跟踪内容全部保留，仅在隔离任务根编辑。

## 下一入口

- 核对已付款用户与正式审核流程后，在批准的私有传输条件下做真实本人查账验收。
- 量化 V28 `0.4.0 (4)` 签名包已构建但未上传，仍需现有项目发布凭据；
  接收端源码 `e009c164bd04076d97d9481bf77b88110758fd62`，证据提交
  `72ed8eac1edd06f2be5e7ab08bf5c622600d55cf`，不由本批主项目发布代替子 APK 上传。
- 正式占用、官方卖回状态机另开受认领的交付切片，不直接复用 Paper 余额。
- Goal 继续有效；本批无真实入账、USDT 兑换、订单、链上签名或资金移动。
