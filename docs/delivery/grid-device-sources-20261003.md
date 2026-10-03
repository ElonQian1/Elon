---
version_status: current
reviewed_at: 2026-10-03
implementation_status: partial
---

# 多端币安网格读取：代码批次与交付缺口

需求真源：[本人币安网格多设备来源 V1](../requirements/grid-device-sources-v1.md)。

## 本批实现

- 主 APK：币安账号页的“多端网格 · APK / Win”，来源复选框独立保存；本人设备同步默认关闭。
- Win：币安官网入口下的独立多端面板；本地观察与云端目录独立处理，刷新时按开关上传本机快照。
- 量化 APK：既有网格选项菜单增加同名入口，APK 复用本机托管授权，Win 复用正式短期只读授权。
- 后端：可信 TLS 上的本人目录与委托 Win 目录；账号隔离、幂等序号、5 分钟有效期、容量与保留上限。
- 同一策略按设备/账号分组，不重复合计收益；不把远端行送给本机交易、详情或分享命令。
- 原有本机读取、创建、管理、历史和分享保持原入口；跨设备列表没有新增交易权限。

后端/Win/Android 主实现分别为 e7d7d7f43 / 8ddab83d9 / ba7cc6a84，
Win 布局修正为 91a962376；均已推送 main。量化实现位于独立子项目的 grids/sources。

## 已取得证据

| 范围 | 结果 |
|---|---|
| Rust 合同/SQLite | private-read-projection-harness 通过，新增 6 项设备来源用例 |
| 后端、Win 壳 | 各自完整 cargo check 通过，使用 validate-rust 与当前共享缓存 |
| PC | 实际模型 4 项测试、TypeScript 检查、生产构建通过；布局修正后重跑模型与构建通过 |
| 主 APK | 网格相关 233 项测试及 assembleDebug 通过 |
| 量化 APK | Debug 702 项测试、10 项跳过，其余通过；assembleDebug 通过 |
| 子项目全量 | validate.ps1 的 Rust fmt/clippy/test 和前端测试/构建通过 |
| V2 治理 | 规范检查与 12 项自测通过，不代表像素或性能验收 |
| 隔离模拟器 | 量化新增真实 Activity 的双选、APK-only、Win-only、全不选空态通过；该轮为持久化偏好补丁前的 Debug 中间构建 |

主 APK 全量测试另有两项失败：ChatGptWebAcceptanceAttachmentNativeActionsTest 的附件 fixture 断言、
SocialAiModeSegmentedControlContractTest 的旧视觉源断言；对应源文件未由本任务修改，未夹带修复。
相关日志使用 grid-device-* 命名；没有采集账号内容、Cookie 或私密行情值。

## 未完成与准确下一步

- 发布后端已按 publish-server 尝试，分配 v0.3.1819 后构建失败：Windows 主机 build-script 的
  MSVC link.exe 把共享缓存 UNC 对象路径解释成选项，LNK4044 / LNK1561；未完成上传与部署。
  离线校验使用的进程级 rust-lld 参数不能直接覆盖 musl 发布 flags，未修改共享缓存或发布门禁。
- 主 APK、Win、量化 APK 均未正式发布；主 APK 新页未取得原生运行证据，不能用 Robolectric 代替。
- 量化当前进程未载入受控签名/上传配置；已有历史本机发布辅助文件未执行，不能把未载入等同于密钥丢失。
- 需先恢复发布环境，补主 APK 原生入口和最终量化构建验证，再按原发布脚本成套更新。
- 实际双端登录、换账号、停止同步/离线恢复由用户监督，只读验收尚未进行；没有触碰真实交易。
- Win V1 是手动刷新同步，不是常驻后台自动同步；主 APK 只复用已有观察事件，不额外持续抓取网页。

本记录只证明代码与离线检查，不证明已经上线、真实数据跨端可用、性能达标或完整交付。
