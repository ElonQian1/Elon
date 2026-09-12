---
version_status: current
reviewed_at: 2026-09-12
implementation_status: implemented
verification_status: partial
delivery_status: pending
acceptance_status: deferred
---

# 币安宿主事件通信交付

正式需求：[Android 币安会话事件通信](../requirements/android-binance-session-events-v1.md)。量化 APK 的创建、管理和列表 UI 留在子项目，主 APK 只扩展会话及固定业务接口。

- `BinanceHostEvents` 经既有包名、签名和 UID 校验登记 Messenger；通知只有随机订阅标识、主题和单调修订，不传网站或账户正文。订阅表有容量、UID 所有权、Binder 死亡与具体操作租约约束。
- 前台事件订阅承接旧的查询续租；注销后按原闲置期限释放，不延长平台账号有效期。旧快照接口兼容，但查询本身不再主动调用网络恢复。
- `ElonBinanceReference` 只接收官方源、主框架、当前文档和当前请求通知；资金与动态参考仅在完成后读取 JS 结果。取消、换文档、换账号、迟到回复及超时失效原准备。
- 参数检查使用完成回调和一次性截止时间；创建/管理及报告结果、失败、到期通知量化消费者。不增加自动提交或写操作重放。
- 私有请求完成、官网原有响应观察与持续交易所推送分开：本批未证明私人网格/持仓/余额 WebSocket。

## 当前证据

网格 JVM 151 项、JS 88 项通过。验证包含订阅 UID、容量、清理、一次性完成和失败通知、取消无迟到通知、准备规则截止及既有读写合同。主 JVM 首次构建发生原生内存分配失败，降低 Gradle 占用后通过。

正式发布版本待补。量化 0.7.39 是本批对应消费者候选；必须先更新宿主再更新量化。设备清单为空，装机、实际 Binder 跨 APK 通知和手机功耗待验，不能以单元测试替代。没有重启 Win/Chrome 或执行金融交易。

量化仓库 `docs/delivery/grid-session-events-20260912.md` 保存周期调度审计、公开行情来源及双端验收清单。需求和 Feature Registry 分别保存正文与工具维护的状态/证据，发布后补当前版本事实。
