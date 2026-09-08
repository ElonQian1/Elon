---
version_status: current
reviewed_at: 2026-09-08
implementation_status: in_progress
---

# 主 APK 托管币安只读会话：第一批验收

需求见 [主 APK 托管需求](../requirements/android-binance-host-read-v1.md)。本批已验证手机官网响应进入主 APK，再经授权进入量化原生列表；不等于全部生命周期和交易功能完成。

## 已交付

- 主 APK `1.1.1557 (1557)` 已通过官方发布入口发布、核验远端工件并覆盖安装到现有手机，沿用正式签名和应用数据。主源码 `ee145704cad8e2c20218f0a03ae0a6e760d10f08` 已推送主线。
- 量化 PR [#8](https://github.com/ElonQian1/yilong-quant/pull/8) 已合并；安装的正式签名候选 `0.5.0 (5)` 内嵌源码 `41b9381486230a579ad2c0f2ba806fee7d78d3fb`，本批尚未发布量化商店。
- “我的币安网格 · 手机连接”进入量化原生页面，“授权查看”打开主 APK 官方 WebView；旧 Win 快照保留独立入口。
- 主 APK 保留网站会话，固定 ContentProvider 只提供 `read/detail/revoke`。Android 校验调用者 UID/包名和正式签名；量化持有十五分钟以内的内存授权及固定业务数据，网站凭据不跨 APK。
- 新认证 MCP `binance_host_status` 提供有界连接状态、代次和条数，不输出账号、策略标识、资产数值或凭据。

## 实测与离线验证

| 检查 | 结果 |
|---|---|
| 主 APK 状态测试 / 编译 | 9 项通过，Release Kotlin 编译通过 |
| 页面适配器隔离测试 | 17 项断言通过，无交易所网络 |
| 量化 Android 测试 | Debug 与 Release 各 95 项通过 |
| 量化安全合同 / PR Android CI | 通过；CI run `34177021354` |
| 主源码体积 / 文档 / 所有权门禁 | 通过 |
| 手机安装和入口 | 两个正式签名包成功安装；量化 → 主 APK 官方连接页实测通过 |
| 手机官网列表 | MCP `host_present=true`、`main_session_current=true`、`list_verified=true`、`row_count=25` |
| 授权返回量化原生 UI | `hosted_entry=true`、`read_enabled=true`、`source_grid_count=25`；选来源后 `hosted_list_visible=true` |
| 整仓 Rust 验证 | 本机磁盘门禁阻止；本批无 Rust 改动，不宣称通过 |

主 APK SHA-256：`87fea3dd858d8851365ebf5a51d654aa2cfc7f663f2c4d6f82a1673c75988811`。
量化候选 SHA-256：`7e734dc43bb37b894c7a7b5b70b21478d788e15993f1a27ec5515e4024e5162b`。
有界设备回执和安装包保存在 `D:/rust/active-projects/ElonNodeData/artifacts/android-binance-host-read-v1` 及相邻量化工件目录；未导出 UI 层级或账号正文。

## 验收边界

25 条记录来自手机 `android_webview` 当前官方响应，已进入量化原生 UI；与旧 Win → 私人云端 → APK 的 24 条历史验收分开。
详情、撤销清除、账号切换和进程恢复尚需继续实测；详情探针首次执行时手机已切到 Chrome，未产生详情点击，不能计为成功。
列表覆盖范围仅本次官网响应；空列表尚不能建立身份，完整分页与多账号迁移尚待补齐。当前单一主账号归属绑定拒绝其他主账号继承。授权过期、页面代次改变和进程恢复需要重新连接。交易写操作未提供。

## 工作方式

Win 用于前端资源哈希、定向源码索引、实际请求与响应合同研究；结论进入版本化适配器，再集中做 Android 登录、恢复和双 APK 验收。日常接口研究不用每次连接手机。
本批没有重启 Win 或 Chrome。功能登记维持 `in_progress`，未完成的手机生命周期验收不会被离线测试或安装成功覆盖。
