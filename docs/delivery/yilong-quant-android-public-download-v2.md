---
title: "官方量化 APK 新版唯一公共下载 V2 交付证据"
version_status: current
reviewed_at: 2026-09-05
implementation_status: verified
---

# 官方量化 APK 新版唯一公共下载 V2 交付证据

## 交付结果

主项目 Android 源码已经切换到新版唯一下载合同。稳定项目 ID `yilong-quant`
使用精确公开 HTTP/HTTPS 路由，下载前不读取登录 token，安装器也不会把 bearer
追加到 URL。其他项目仍要求登录并沿用原成员 token 下载路径。

官方量化身份门禁的最低版本已经从 `versionCode=2` 提升到 `5`。固定包名、唯一
发布证书和 `versionCode>=5` 必须同时成立；0.2、0.4 及更早 APK 不再被主项目
视为可打开或已完成安装的产品。主项目不会远程卸载设备上的旧包或删除其数据，
正式 0.5 包应以同包名和既有证书完成 Android 正常升级覆盖。

## 实现证据

- `ProjectSpaceDownloadButton` 只为非官方项目读取 `AuthManager.token`；官方量化
  直接进入匿名公开分支。
- 项目空间把本次加载所用的活动服务器逐层传到安装策略；默认入口才回落到
  `BuildConfig.SERVER_URL`。官方分支忽略项目空间返回的 direct release URL、host、
  path 和查询参数，只从活动服务器构造固定公开路由。活动服务器必须是无 userinfo、
  query、fragment 和额外 path 的绝对 HTTP/HTTPS 来源，最终 path 恒为
  `/api/store/projects/yilong-quant/downloads/android`。
- 官方公开下载使用无 interceptor/cookie/authenticator 的独立客户端并禁止 HTTP 和
  HTTPS 重定向，避免 3xx 绕过精确路由；私有项目继续使用原鉴权客户端。
- 服务端在已有 published release 时仍把官方量化的项目空间 URL 投影到固定 store
  路由，同时让 identity 保留实际 release URL；上传 0.5 后不会切回不兼容的直链。
- `openProjectApkInstall` 只消费策略返回的 URL。公开分支原样传递无凭据 URL，
  私有分支继续只追加一次成员 token。
- 项目广场的已安装状态和按钮标签也重验官方包名、唯一证书与最低版本；旧包或错签
  包不再显示为“打开应用”。
- 安装后的私有临时文件、只读封存、包名/证书/版本检查、系统安装兼容检查和
  每次打开重验均保持原接线。

## 验证记录

| 验证 | 结果 | 说明 |
| --- | --- | --- |
| TDD 红灯 | expected failure | 实现前源码合同因最低版本仍为 2 而失败，证明测试先捕获旧行为 |
| 源码与突变合同 | passed | `22` 个回归突变全部被检测，官方活动来源、published release 稳定路由、无 token、禁重定向、已安装身份与私有鉴权接线保持 |
| Android 相关单元测试 | `19/19` passed | 下载与客户端策略 6、官方身份 9、正式 ESK 调用方身份 4 |
| Rust 路由单元测试 | `2/2` passed | 官方 published release 固定公开路由且保留实际 identity；其他项目 URL 不变 |
| Kotlin/Debug APK | passed | `:app:assembleDebug` 成功，包含本轮生产源码编译 |
| Release 缺凭据门禁 | expected rejection | 用明确不存在的 keystore 路径验证构建在签名前失败关闭；未读取或使用真实签名秘密 |
| Android 全量单元测试 | baseline debt | 运行 1283 项，27 项未改动的 UI/聊天合同失败；因此不能声称全量套件通过，需由对应模块独立修复 |

当前 Microsoft JDK 的默认 Windows AF_UNIX 临时路径会让 `Selector.open()` 报
`Invalid argument`。验证只在单次构建进程内设置 `jdk.net.unixdomain.tmpdir`，
并已确认 `C:\Windows\Temp` 可用；没有安装 JDK、修改系统设置或改动仓库配置。

## 发布与用户验收状态

- **代码：** 已实现并完成本地风险匹配验证。
- **主 APK：** 本轮不发布。线上量化下载仍是旧 `0.2.0 (2)`；提前发布最低版本 5
  的主 APK 会让现有广场工件失败关闭，却不能给用户提供新版。
- **量化 APK：** 0.5.0 源码已经在量化仓库，正式签名包、项目编辑者上传回执和
  公网 0.5 工件仍缺。不得用 0.4、Debug 包或新建证书替代。
- **服务器目录：** 待 0.5 正式上传后同步版本、大小、SHA-256 和目录信息，再发布
  主服务器与主 APK。
- **真实设备：** 待双正式 APK 可下载后执行广场安装/升级、打开、逐页授权、后台、
  旋转、换号和过期验收。当前不能声称用户已经在线获得新版。

本轮没有读取签名密钥或上传 token，没有发布 APK，没有调用币安、移动 ESK/SUI/
USDT，也没有改变 Paper、sandbox 或 live 资金状态。
