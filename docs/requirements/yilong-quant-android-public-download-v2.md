---
title: "官方量化 APK 新版唯一公共下载 V2"
version_status: current
status: accepted
implementation_status: verified
reviewed_at: 2026-09-05
owners: [android-platform, project-marketplace]
---

# 官方量化 APK 新版唯一公共下载 V2

## 用户结果

主项目项目广场只安装或打开正式签名的量化 `0.5.0 (5)` 及更高版本。官方
`yilong-quant` APK 使用主服务器公开下载路由，不要求用户先登录，也绝不把主项目
bearer、账号或其他凭据追加到 HTTP/HTTPS URL。旧 `0.2.0 (2)`、`0.4.0 (4)` 和
更早量化包不再作为兼容产品；已安装旧包不会被自动卸载或清除本地数据。

## 依赖与范围

- 复用[官方量化身份校验 V1](yilong-quant-android-identity-v1.md)固定的项目 ID、包名、
  Activity 和发布证书，只把最低可接受 `versionCode` 提升为 `5`。
- 复用[主服务器托管 V6](yilong-quant-android-main-hosting-v6.md)的公开、无凭据下载
  路由 `/api/store/projects/yilong-quant/downloads/android`；允许当前项目配置的 HTTP
  主服务器，因为该响应只包含公开 APK 字节，不承载用户秘密。
- 复用[正式 ESK 进度 V1](esk-platform-native-progress-v1.md)的新版唯一入口；本功能
  不恢复旧 17/21 字段原生桥、旧页面、别名或旧包回退。
- 其他项目继续原有成员鉴权：缺少登录 token 时拒绝下载，具备 token 时沿用既有
  查询参数合同。本功能不把公开规则扩展到任意项目或任意 URL。

## 下载与身份合同

1. 只有稳定项目 ID 精确为 `yilong-quant` 时使用公开下载分支；显示名称、大小写变体
   或相似 ID 均不能触发。
2. 官方分支不采用项目空间返回的 release URL、host、path 或查询参数，而是从本次
   项目空间使用的活动服务器直接构造公开 URL。活动服务器必须是无 userinfo、无
   fragment、无 query、无额外 path 的绝对 HTTP/HTTPS 来源；最终路径固定为
   `/api/store/projects/yilong-quant/downloads/android`，因此服务端 direct release
   URL、可疑 URL 和任意第三方 host 都不会被转发。
3. 官方分支不读取登录 token 作为下载前置，不调用 token URL 组装器。即使调用方
   意外传入 token，也必须忽略该值并保持下载 URL 逐字不含凭据。
4. 下载后的文件继续进入私有临时目录、只读封存、当前单签/固定包名/最低版本校验，
   再进入 Android 原安装兼容检查；不能因公开下载而跳过工件身份验证。
5. 非官方项目保持原顺序：先确认登录 token，再对清洁 URL 追加一次 token，随后才
   下载；不得把这条旧成员能力误删为全平台匿名下载。
6. 官方已安装包每次打开仍重新检查当前单签、包名、`versionCode >= 5` 和精确
   Activity。旧包只能引导用户获取新版，不能继续打开或被当作安装完成。
7. 服务端项目空间即使已经存在 published release，也必须向官方量化客户端公布固定
   store 公共路由；版本 identity 仍绑定实际 release URL，保证上传新版后既能触发
   更新判断，又不会把客户端切回 direct release 路径。

## 发布顺序

代码推送不等于用户已经获得新版。正式发布必须按以下顺序完成并分别留证：

1. 从量化仓库包含新版唯一入口的干净提交受控构建并使用既有官方证书签名
   `0.5.0 (5)`；不得复用旧 APK、Debug 包或新建临时发布证书。
2. 量化项目编辑者上传后核对回执、公开下载的包名、版本、唯一证书、源码提交、
   大小和 SHA-256。
3. 同步官方目录并发布主服务器，再发布包含本门禁的主 APK；在量化 `0.5.0` 尚未
   可下载前，不得提前发布会拒绝线上旧包的主 APK。
4. 最后在真实设备完成项目广场升级/安装、打开和双 APK 本人 ESK 逐页授权验收。

## 验收标准

1. 单元测试证明官方项目在空 token 和非空 token 下都得到同一无查询公开 URL；
   服务端 direct release、userinfo、query、fragment、错误 path 和第三方 host 均被
   规范到活动服务器的固定路由，非法活动服务器与相似项目 ID 则拒绝。
2. 接线测试证明官方分支不读取 `AuthManager.token` 作为前置且下载器不调用
   `projectApkUrlWithToken`；非官方分支仍需登录并追加 token。
3. 身份测试接受固定证书的 `versionCode=5` 及更高版本，拒绝 `null`、负数、`0..4`
   和所有错误包名/证书；安装前和每次打开的门禁保持接通。
4. Android 定向测试、Rust published-release 路由测试、Kotlin/Rust 编译、Debug APK
   构建、源码尺寸与文档模块化门禁通过；默认 Release 在没有受控签名输入时继续
   失败关闭。
5. 代码、主 APK 发布、量化 APK 发布和真实设备验收分轴报告。没有量化正式签名包与
   上传回执时只能标记代码已实现，不能声称项目广场已有新版。

## 非目标

不读取或创建签名密钥和上传 token，不发布 Debug APK，不绕过项目编辑权限，不操作
ESK、SUI、USDT、币安账户或用户资金，不更改 Paper/sandbox/live 状态，也不删除
用户资产账本、审核历史、已安装应用或应用数据。
