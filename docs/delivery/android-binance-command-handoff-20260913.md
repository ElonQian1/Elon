---
version_status: current
reviewed_at: 2026-09-13
implementation_status: implemented
verification_status: partial
delivery_status: published
owner: android-session-host
---

# 币安页面重建后的命令接续

对应[正式需求](../requirements/android-binance-command-handoff-v1.md)。本批修改主 APK 创建/管理命令服务和事件成员通知。量化 UI、网站 API、原确认流程不变，量化 0.7.54 沿用已有命令；不需要为本批再发布相同量化源码。

## 问题与修复

实际命令服务的两项修复前测试均失败：旧事件订阅移除后，新 operation 的 open 仍返回 busy；旧订阅退出后，另一个页面没有收到 state 通知。第一项会让重建页面等待旧服务租约，第二项使它不能根据连接变化恢复。

现在仅在旧 operation 无活动订阅、新 operation 有对应 kind 活动订阅时交接。同一主线程先清除旧许可/准备/摘要，再改变当前 operation；未提交草稿准备取消。在途、未知、已受理和待核对记录保持原 attempt/state、session、回调和日志。旧 operation 后续请求仍拒绝，不能取消或关闭新所有者。事件成员变化发送一次状态失效通知；没有新增周期查询。

管理只在相同协议版本交接。不同版本仍要求结束原会话后重连；创建与管理及兼容官网页面继续互斥。未取得 strategyId 的未知创建仍需按原客户编号到官网核对，本批不提供无证据的自动认单。

## 验证与当前边界

- 已通过 10 项真实 Android 本地命令测试，包括原失败复现、无新订阅/错误 kind 拒绝、旧命令失效、旧创建/管理准备撤销，以及四种未决状态的原对象、日志、回调和忙碌状态保留。
- 这些测试实例化实际 HostRuntime、Commands、事件 Binder 和本地 journal；账户存储为空、WebView 始终为空，未调用真实交易入口。它们不证明真实账号、系统进程死亡或手机 UI 已通过。
- 管理 V2/V3/V4、连续重建以及既有创建/管理/事件和正式调用方策略共 177 项测试全部通过，零失败或跳过；Release 运行依赖图不包含 Robolectric。
- 新增 Robolectric 4.16.1 仅用于测试，复用量化版本；旧 Jetifier 对多版本 BouncyCastle 类的扫描失败通过精确排除该非 Android 依赖解决，其他库继续转换。配置依据见 [Google Jetifier 说明](https://issuetracker.google.com/issues/263427107)。没有降级密码库或关闭整个 Jetifier。
- 工件组 `android-binance-command-handoff-v1` 保存实际失败 XML、修复前后日志、后续验证和发布回执。

## 发布身份

主 APK `1.1.1713 / 1713` 已由正式脚本分配版本、构建和发布；源码 `c06beb9aff527b24b7de605719e2c62e56deb7db`。Release 打包和 lintVital 通过，服务器回读版本/源码/大小与哈希一致。

- APK：40,423,968 字节，SHA-256 `58635709577d7f3a8e2ba8288f8533bc3a1b492fefefb7d3f14b7c5ede006284`。
- 签名校验通过，证书 SHA-256 `f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`。
- 工件组保留正式 APK、发布回执、签名和日志。量化继续使用已发布的 `0.7.54 / 71`，该包与新主包均尚未在本轮安装。
- 发布脚本的通用 worktree 自动清理报告缺少 Branch 属性；业务发布成功，定向收尾另行执行，未改动其他工作区。

手机无线 ADB 当前不可达。仍需原签名覆盖安装，并在量化完成普通创建/管理页面重建与恢复读取。实际交易在途、确认后返回和回读须由用户执行最终动作，AI 读取结果；这些仍保留为总 Goal 的必要待验项。
