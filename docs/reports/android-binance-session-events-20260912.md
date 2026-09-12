---
version_status: current
reviewed_at: 2026-09-12
implementation_status: implemented
verification_status: partial
delivery_status: published
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

网格 JVM 151 项、JS 88 项通过。验证包含订阅 UID、容量、清理、一次性完成和失败通知、取消无迟到通知、准备规则截止及既有读写合同。一次主 JVM 构建发生原生内存分配失败，降低 Gradle 占用后通过。

主 APK 1.1.1685(1685) 已按官方入口构建发布，服务器版本与 sourceSha 核对通过，源 `0cb9234dd7b4e3ac695e592691657bae16e7f5bb`；APK SHA-256 `f546fd4544d65a01605f537d120c98df571957b178236c6af82ac492f89f8a6f`。

量化 0.7.39(56) 随后原签名发布，源 `88cd7a97d9032cbd4252a9238ea07574e7bb6689`，发布回执 `rel_7899c7614df94551bb123963db793311`。384 项 Release、382 项 Debug（另 2 项仅正式身份）、官方 Android 合同及完整仓库验证通过。先更新宿主再更新量化。

设备清单为空，未安装本批双包；实际 Binder 跨 APK 通知、手机公开 WebSocket 连通性、功耗和视觉待验，不能以单元测试替代。没有重启 Win/Chrome 或执行金融交易。发布脚本的通用 worktree 自动清理曾报告 Branch 属性缺失，独立 finish 结果作为收尾依据；未为此更改共享清理代码或处置其他任务文件。

量化仓库 `docs/delivery/grid-session-events-20260912.md` 保存周期调度审计、公开行情来源及双端验收清单。需求和 Feature Registry 分别保存正文与工具维护的状态/证据，发布后补当前版本事实。
