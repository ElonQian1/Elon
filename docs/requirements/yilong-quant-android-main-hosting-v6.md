---
version_status: current
reviewed_at: 2026-09-03
implementation_status: accepted
---

# 一龙量化 Android 主服务器托管 V6

## 目标

让一龙主项目在不增加物理服务器的前提下，承担“一龙量化交易”正式 APK 的项目发布存储、项目广场发现、下载、更新和打开入口。主 APK 已有的 ESK Paper 资产卡片保持主项目唯一资产视图；量化 APK 打开后从主项目现有 HTTP `8080` 服务的 `/quant/` 路径读取无凭据 Paper 状态和公开模拟结果。

APK 文件托管和量化服务运行必须是两个独立状态：主服务器保存安装包，不代表量化 PWA/API 已部署。当前批准的测试架构复用主项目 Rust/Axum、`tower-http` 静态资源托管、`8080` 公开端口、发布流程和数据目录；量化 Rust API 仍使用独立进程、独立配置和 Paper SQLite，并且只监听 `127.0.0.1:8787`。不引入 Nginx、第二个公网端口或第二套反向代理。

## 当前可复用能力

- 主 APK 已能从项目广场/项目空间显示安装、更新和打开动作，并在安装前检查 APK 包名与签名兼容性。
- 主项目已有 `project_releases` 发布记录、`DATA_DIR/project-releases` 文件存储和项目发布上传 API。
- 主项目已有 ESK Paper Android 卡片和 V15 双仓可见余额离线证据。
- 官方目录已有 `yilong-quant` 项目和加入前净化预览。

## 本批实现

1. 公开项目下载路由遇到当前 landing 指向主服务器最新 release 时，不再跳转到需要第二次鉴权的成员下载路由，而是直接读取受管发布文件。
2. 只允许公开读取 canonical path 位于 `DATA_DIR/project-releases` 下的文件；其他项目 APK 继续使用既有外部跳转或成员鉴权路径。
3. 返回前必须同时核对数据库记录的 `size_bytes` 与 SHA-256；缺失、非法或不一致时失败关闭。
4. 官方目录在服务器启动重建基础 landing 后，重新叠加当前已发布 release，避免重启把量化 APK 恢复成 `planned`。
5. 当前没有正式量化 APK 时，目录继续显示 `planned`，不能用源码或 Debug APK提前变成“可安装”。
6. 主项目 Rust/Axum 在 `/quant/` 托管量化 PWA，并只代理健康状态、运行时、研究快照、回测和 Binance Spot 公开行情总览五个无凭据 Paper 接口；行情路由只接受 `GET` 和不超过 2 KiB 的查询参数，不转发请求头、Cookie 或认证信息。不得代理本人仓位、ESK、订单、运营、导入或任何写资金接口。
7. HTTP Paper 预览必须显示明文传输风险，不接受登录、钱包、交易所密钥、主项目 bearer、Paper grant、ESK 投影或个人资料。需要本人 ESK/仓位的通道继续关闭，不能借本预览降级安全边界。

## 非目标

- 不把量化 API、Paper SQLite 或策略引擎合并进主项目进程或数据库。
- 不使用 Nginx，不改变主项目 `8080` 监听，不向公网开放量化内部 `8787` 端口。
- 不发布 Debug APK，不在仓库提交 APK、签名密钥或发布 token。
- 不把主账号 bearer 传给外部下载源，也不写入量化 APK URL。
- 不实现独立量化 APK 的本人 ESK/仓位授权；该能力需要后续一次性授权码协议。
- 不启用真实链上 ESK、用户资金、sandbox/live、收益或卖回付款。

## 验收标准

1. 主服务器受管 release 的目标 URL、路径、字节数和 SHA-256 全部一致时，公开商店下载路由直接返回 APK，且响应为 `no-store` 并携带观察用 SHA-256 header。
2. 发布文件缺失、大小不符、摘要不符或摘要缺失时返回不可用，不继续下载损坏工件。
3. 外部 HTTPS APK 仍使用不携带主账号凭据的临时跳转。
4. 官方目录启动同步后保留数据库中最新已发布 Android release；无 release 时仍保持计划状态。
5. Android 项目广场既有安装/更新/打开和签名冲突测试不回退。
6. `/quant/` 由主项目 Rust/Axum 返回 PWA；五个公开 Paper 接口可用，未列入白名单的量化 API 返回 `404`，非 `GET` 行情方法返回 `405`，内部 API 只监听 loopback。
7. PWA 响应包含禁止嵌入、禁止嗅探、最小权限和同源连接策略；API 响应不缓存，静态哈希资源可复用主项目长期缓存。
8. 代码推送不等于服务器已发布；只有 Server 发布、量化正式 APK 上传、HTTP Paper 预览和实际安装分别验收后，用户路径才能标记可用。本人 ESK/仓位能力不得由公开预览的通过状态推导。
