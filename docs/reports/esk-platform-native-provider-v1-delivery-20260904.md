---
title: "正式 ESK 原生摘要提供端 V1 交付记录"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, android, quant-integration]
---

# 正式 ESK 原生摘要提供端 V1 交付记录

## 批次边界

本批基线 `278bec4fe027bda49e4d03d7526c17d08f6b4142`，合同提交
`64f93af8b` 已推送。只交付主 APK 正式来源摘要提供端；量化接收端及真实本人联调仍待完成。
需求为 [原生摘要提供端 V1](../requirements/esk-platform-native-provider-v1.md)，
共享线格式为 [21 字段协议](../contracts/esk-platform-android-snapshot-v1.md)。

## 实现与兼容

- 独立 `EskPlatformSnapshotConsentActivity`、新 action/协议、官方量化精确组件与当前签名，
  最低消费者版本 4。旧 Paper 17 字段、主项目个人正式页、QSHARE、行情和交易引擎未改。
- 用户显式确认后，复用已有正式本人 HTTPS reader/parser、原子 SessionStore 与请求门禁。
  请求最长 15 秒，授权窗口 120 秒，返回前再次检查同一会话修订和系统调用者。
- 21 字符串只有正式总量、微单位、笔数、来源/关闭能力、Nonce 与单调时效。
  既不传账户身份/凭据/付款流水，也不推导可花费、可兑付、收益或量化份额。
- 生产布局即 Preview-first XML，禁恢复/截图，原生可滚动及可换行 52dp 按钮。
  网页仅新增原生授权边界说明，不模拟系统身份或添加本人余额接口。
- 提供端返回后不能远程撤回已交付摘要；接收端必须清空过期/离页摘要并明确不是实时余额。

## 验证证据

首轮日志 `esk-formal-provider-jvm-20260904-180833-179`：137 项通过，零失败/错误，
包括原有 104 项和新协议 18 项、Caller 6 项、投影/Wire 9 项。
最终日志 `esk-formal-provider-final-jvm-20260904-181233-312`：14 个套件、145 项全部通过，
包含新增提供端接线 8 项，零失败/错误；总耗时 144.9 秒。Android Intent、真实生命周期和跨 APK 往返
未在设备执行；源码接线检查不能替代这些验收。
移动网页源码检查通过 `styles=2 scripts=1`。旧 Paper 源码 diff 为空。

UI 工作台已导入本批、发现 Views 与网页同步能力、生成并使用正式 Preview XML。
没有目标截图，无 FitRun 目标；Runtime 只返回 `BOOTSTRAP`、未连接，无空闲
rendererResourceId/lease 证据，未准备或操作物理设备。视觉验收保持延期，不阻止发布。

## 状态矩阵

| 能力 | 实现 | 验证 | 交付 | 用户验收 |
|---|---|---|---|---|
| 正式摘要合同与主提供端 | implemented | offline_passed，145 项 | 源码交付，APK 发布记录随后补齐 | deferred |
| 量化正式原生接收端 | not_started | not_run | not_started | pending |
| 私有连接与真实付款登记 | 复用既有实现、生产未启用 | user_action_required | 不改生产配置 | pending |

## 下一轮精确入口

量化只读审计基线 `origin/main=816c9b6d3525fe90bd0cbd02b8fd047a19422751`，已含 V26/V27；
其 Android 与 V24 `4eb55bdc79081914d48952a0948b1df7c8f6d288` 相同。
从量化最新主线新建独立工作树，先按该仓规则查重/认领，不复用落后 V24 根做新交付。

1. 新增 `com.elon.quant.assets.platform.EskPlatformAssetsActivity`，独立原生入口，不改旧 Paper 页面。
2. 复制并校验共享 `EskPlatformSnapshotContract.kt` 字节一致；精确主包/当前签名/非别名组件检查。
3. 消费者独立生成随机 Nonce、保存请求单调时间、一次性消费结果，只在前台内存显示。
   正在等待主确认页时仅保留本实例待处理请求，不保留旧展示；取消/重建/刷新清空请求。
4. 展示正式 ESK 总量、笔数、来源与时效，明确“本次主项目确认账户”，不并入网页登录账户。
5. 验证旧 Paper、跨协议拒绝、调用者升级、Nonce/超时/后台、合成双端往返；签名版本至少 4。
6. 沿用该仓上传流程；目前未提供项目编辑者发布凭据，不能使用 OWNER_TOKEN、伪造用户或改库角色。
   已签名 V24 `0.3.0 (3)` 未上传；既有文档记录线上仍 `0.2.0 (2)`，不是本批新发布事实。

不修改量化 V25 行情、V26 执行器、V27 网格；共享 AI_CURRENT/能力文档由主代理协调。
不开启生产 HTTPS、转入真实 USDT、记入未经核验付款、使用 Binance 密钥或签名上链。
Goal 继续有效，不能以提供端已发布替代首批用户完整资产/安装验收。
