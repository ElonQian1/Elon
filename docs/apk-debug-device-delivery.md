---
version_status: current
reviewed_at: 2026-09-20
---

# APK 发布后的调试手机交付

## 必须执行

服务器正式 APK 发布和版本检查成功后，`apk-publish-postflight.ps1` / `.sh` 调用 `apk-adb-autodeploy.*`，必须检查登记手机并更新在线设备。该步骤属于发布交付，不要求先打开手机画面、登录网站或执行业务操作。

未显式指定配置时，通过发布流程已经使用的 SSH 连接，只读查询服务器 `project_android_devices` 中 `project_id=elon-self` 的记录。当前主项目登记小米 23116PN5BC、荣耀 AAK-AN00 两台手机；实际序列号、端点以设备档案为准，不复制进文档。旧 `~/.elon/apk-adb-targets.json` 可通过参数或环境变量显式使用，不让旧的单机清单覆盖主项目默认设备。

显式参数或 `ELON_APK_ADB_TARGETS_FILE` 指定的文件不存在时失败，不能静默替换白名单。档案读取失败、空清单或 ADB 缺失也必须明确失败。旧配置 schemaVersion=1、serial、hardwareSerial、maxAttempts、retryDelaySeconds、launchAfterInstall 继续支持。

## 探测、安装与验收

1. 按硬件序列号去重。优先检查有线及已连接 ADB，然后尝试登记的无线地址和匹配该硬件序列号的 mDNS connect 服务；不扫描整个局域网，不自动配对或复制 ADB 私钥。
2. 只有 `device` 状态且 `ro.serialno` 匹配才安装。IP 相同不代表同一手机，模拟器与偶然连接的其他手机不安装。
3. 使用已发布的正式 APK，保留数据执行 `adb install -r`。安装前检查已有版本，较新版本保留，禁止降级、卸载或清空数据。
4. 安装后回读正确包名的 `versionCode`，必须等于本次版本。默认主项目档案不强制拉起应用；旧本机配置的显式拉起选项保留。
5. 一台失败后仍检查其他手机；所有命令有超时，安装有有限重试。

## 结果合同

每次产生 `elon.apk_adb_receipt.v1` 回执，写入发布机 `~/.elon/apk-adb-receipts/`，包含时间、包名、目标版本、APK SHA-256 和逐台结果。

- `updated`：安装成功且版本回读匹配。
- `newer_installed`：手机已有更高版本，保留现有安装。
- `offline`：未发现可用连接；报告具体手机，允许服务器发布继续有效。
- `unauthorized`：需要手机确认调试授权，发布后检查失败。
- `identity_mismatch` / `probe_failed` / `failed`：身份、探测或安装失败，不宣称已交付手机。

总体状态 `partial_offline` 表示部分手机离线，`offline` 表示全部离线；在线安装失败会返回非零。`AllowAdbVerificationDeferred` 仅保留调用兼容，不再吞掉这些错误，也不执行全局 `adb kill-server`。

发布结果分开汇报服务器、各手机和真实业务验收。安装与版本验证通过不等于视觉、登录、账户切换或交易验收通过。

## 本批验证（2026-09-20）

真实读取主项目两台设备档案。小米通过无线 ADB 安装并回读 `1.1.1785` / `1785`；荣耀的登记无线端点不可达，也未发现对应 USB 连接，记录 `offline`。安装工件来源为线上正式包，SHA-256 为 `d0d104770b9c41dbec853d2144dac7518ff19883127fe35f8215417ad66a0132`。

Windows 安装替身回归与 8 项发现/编排场景通过；Shell 入口语法与 Python 4 个测试方法通过（覆盖 USB、无线/mDNS、离线/未授权/错误身份）。真实 USB 切换和 Linux 安装尚未现场验收。
