---
version_status: current
reviewed_at: 2026-09-13
decision_status: accepted
implementation_status: in_progress
owner: quant-grid
---

# Android 欧易只读授权服务 V1

承接 `android-exchange-session-host-v2.md`：量化拥有交易产品界面，主 APK 在本机保管欧易正式 API 授权资料并提供固定读取接口。用户希望复用欧易官方接口，与币安网格共用量化列表和详情；本需求不包含交易执行。

## 范围及验收

- 量化从自身入口请求授权；主 APK 只提供 API Key、Secret、Passphrase 输入、账号核验和持续只读同意页面，确认后返回量化。
- 固定全球正式只读服务，环境明确标为 live；模拟及其他区域不得静默切换。只接受仅有 Read 权限的 API 授权。
- 主 APK 使用 AndroidKeyStore AES-GCM、原子文件及 noBackupFilesDir；密文绑定一龙用户与合同版本。资料不进入量化、日志、备份、聊天或 savedInstanceState。
- 校验量化包名、签名及系统调用 UID。读取绑定当前一龙会话、实际欧易 UID、版本化授权及本机授权代次；退出、换号和撤销拒绝旧结果。凭据更换重新核验，不能跨账号复用旧列表。
- 固定 GET 账号配置、合约网格运行列表及指定网格详情；禁止任意 URL、请求头、脚本、POST 或写操作。请求超时、响应大小、页数和总数有界，分页只在成功空页后完整；不以短页结束。
- 量化正常读取不依赖 Win。网络在工作线程运行；完成后返回结果，无定时轮询。私有 WebSocket、历史/成交/持仓和管理能力另行补齐，不能宣称已实时接入。
- 收益单位未核验时保留 unknown。原币安读取/授权/创建/管理无行为变化。

## 文件计划

| 模块 | 责任与预计规模 |
|---|---|
| `exchange/okx/OkxReadProtocol.kt` | 固定请求、身份、签名与输入验证，约 180 行 |
| `exchange/okx/OkxCredentialVault.kt` | 本机密文保存、读取与撤销，约 140 行 |
| `exchange/okx/OkxReadTransport.kt` | 固定 HTTPS、错误和完整分页，约 180 行 |
| `exchange/okx/OkxReadProjection.kt` | 既有模型的 Android 转换，约 200 行；使用共同样本验证跨端一致性 |
| `exchange/okx/OkxReadHost.kt`、`OkxReadProvider.kt` | 会话/授权与系统调用边界，各不超过 250 行 |
| `exchange/okx/OkxConnectActivity.kt` | 授权输入和返回，约 220 行；复用品牌组件 |
| `AndroidManifest.xml` | 仅登记两个固定组件 |

协议、密文、账户切换、分页失败和跨端样本使用离线测试。正式原签名构建、发布、安装、视觉与用户 API 授权实测分别记录；缺少用户授权资料不算真实私有读取通过。
