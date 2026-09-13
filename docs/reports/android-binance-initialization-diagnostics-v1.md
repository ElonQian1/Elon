---
version_status: current
reviewed_at: 2026-09-13
owner: quant-grid
---

# 币安初始化诊断交付与手机读取证据

范围来自[初始化诊断需求](../requirements/android-binance-initialization-diagnostics-v1.md)。本报告记录本轮结果，不扩大网格产品验收范围。

## 交付身份

- 主 APK `1.1.1705 / 1705`，源码 `398d88a89e9bc2764d9a4457ee7211ea666f3327`。
- APK SHA-256 `786ec2463796c1997fda761f3ed0f399cb12d9453bfa4947e220f6d269b267de`，原签名核验通过；正式发布、无线 `adb install -r` 覆盖安装均成功，保留登录资料。
- 手机量化 APK 保持 `0.7.50 / 67`，源码 `741ed6dc9e065beb55c75ad23b96d53b28c204e3`。
- 12 个官网适配器套件、宿主生命周期边界及 102 项原生测试通过，包含新增 5 项诊断测试。正式 Release 构建与 lintVital 通过。

## 当前设备证据

| 能力 | 观察结果 | 结论 |
|---|---|---|
| 主 APK 内置诊断 | 取得脚本错误类别、页面挂载状态；无任意脚本或凭据导出 | device_verified |
| 本人账户与网格列表 | 固定只读 `reload` 启动新页面后，身份和列表返回 HTTP 200，子账户、1 条网格核验成功 | 本轮真实读取成功；不等于所有冷启动自动恢复已通过 |
| 量化列表与详情 | 经已有授权显示 1 条新鲜网格、收益摘要及详情入口；详情私有请求也返回成功 | device_verified |
| 量化原生创建页 | 同一子账户，NEARUSDT，资金读取成功；杠杆 1 倍，公开上限 75 倍；上下限、格数已填入 | 连接与字段可用已验证；本次未读取具体价格与格数值 |
| 参数检查与预览 | 编辑 30 USDT 测试草稿并触发只读检查后，没有形成有效预览；过程中资金状态失效，之后恢复 | failed / 继续定位，不宣称创建流程通过 |
| 清理 | 清空本轮保证金草稿、无有效 preparation、无确认勾选，返回列表 | passed；真实订单、持仓修改和结束动作均为 0 |

成功读取时，WebView 未挂载、尺寸为 0，原有 6 条脚本错误仍存在。新增控制台分类为 unknown、TypeError 和 CSP，公开脚本位置没有符合白名单的条目。上述现象不能再单独作为“接口无法工作”的原因，也不能把新增诊断当作已证明的初始化根因修复。后续应对照失败与成功时的业务请求生命周期。

验收中量化数次失去前台，系统记录包含 ADB shell UID 2000 启动主 APK 的事件；未确定具体发起进程，不能据此断言量化自身跳转。两个定向 UI hierarchy 读取均取得主 APK 内容，没有取得量化视觉证据；不重复大量 dump、不移除安全窗口限制。完整视觉、稳定预览、无调试干预的恢复及人工交易验收仍待完成。

证据保存在外置构建产物 `grid-initialization-diagnostics-v1`：`main-release.json`、`install-proof.json`、`signature-verification.log`、`tests.json`、`contracts.log`、`release.log`、`host-initialization-observed.json`、`host-after-grid-refresh.json`、`host-create-requested.json`、`device-diagnostic-connected-home.json`、`device-diagnostic-detail.json`、`device-diagnostic-create-ready.json`、`device-diagnostic-create-check-result.json`、`device-diagnostic-cleared.json` 和 `focus-launches.json`。源码、发布、装机与设备证据分别记录。
