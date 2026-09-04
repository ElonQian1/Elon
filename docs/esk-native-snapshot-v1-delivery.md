# ESK 原生只读快照 V1 交付记录

日期：2026-09-04。需求：[ESK 原生快照](requirements/esk-native-snapshot-v1.md)。
本文件只记录证据，不改变产品或资金权限。

## 本批实现

- 主项目独立原生确认页、官方量化当前签名/组件验证、会话变更撤销、独立 HTTPS 只读读取器。
- 量化独立原生“我的 ESK”入口、官方主 APK 当前签名/组件验证、一次性随机 nonce、内存快照与生命周期清空。
- 17 个纯 String 字段；精确微单位、守恒、旧结果/未知字段/超长/异常状态失败关闭。
- 源 API 限定 asset_account.v2 Paper、未上链、未移动资金；HTTP 零读取凭据/零创建请求。

## 验证与交付状态

| 层级 | 状态及证据 |
|---|---|
| 双仓合同 | 静态一致检查 passed；源码、18 项测试和协议文档字节一致 |
| 量化完整验证 | `scripts/validate.ps1` passed（Rust、前端及主托管构建）；Android 31 项测试、Debug 构建及受控 Release 拒绝测试 passed |
| 主项目定向验证 | 最终 44 项单元与 Debug 构建 passed；包含 17 项 provider/transport、18 项共享合同、9 项官方身份策略 |
| 独立安全联审 | 未发现可确认 P1/P2；不代替双 APK 运行验收 |
| 源码推送 | 主项目 `44894ebf2ed1ee513117e5036b27dad2dd8b930e`、量化 `1d4b5cd9c644c3a9b49c20b12e6c776346a348ee` 已推送；后续交付文档不改变 APK 内嵌来源 |
| 主 APK 发布 | `1.1.1502 (1502)` 已发布；远端大小、哈希、版本与内嵌源码核验通过，签名仍为原官方证书 |
| 量化 APK | `0.3.0 (3)` 已用原独立签名构建，Debug 与 Release 各 31 项测试通过；缺现有编辑者发布凭据，尚未上传；项目广场仍为 `0.2.0 (2)` |
| 原生视觉/生命周期 | `ui_verify_with_fallback` 超时，Runtime 仍 BOOTSTRAP；`VERIFICATION_DEFERRED`，没有真机操作 |
| 个人资产公网 | 未开通：编译配置仍 HTTP，新增私有读取失败关闭；未修改服务器或证书 |

## 发布证据

主 APK 下载：<http://43.139.149.158:8080/app/ElonSpeed-latest.apk>。
发布时 `app/version.json` 为 `1.1.1502 (1502)`，源码 `44894ebf2ed1ee513117e5036b27dad2dd8b930e`。
大小 `39634554` 字节；SHA-256 `95a5bffb93e653272b246e1f7c0e1bd1405eaf59805c1e61ab857bb83f6fdba5`。
证书 SHA-256 `f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`。
正式发布记录为 `CODE_SYNC_STATUS=synced`、`APK_RELEASE_STATUS=published`、`SERVER_RELEASE_STATUS=not_attempted`；
没有部署后端、改证书或操作真机。公开下载的 latest 地址会随后续版本变化，应以本批固定哈希核对。

量化 APK 大小 `631512` 字节；SHA-256 `b98f771cb827e522c09d25945707c490c0755d8ff62424de93cae32fb780f01e`；
内嵌源码 `1d4b5cd9c644c3a9b49c20b12e6c776346a348ee`。
证书 SHA-256 `019a3d95366fb4c6fe578c1f7f26fb96e462dc54f41b9a7c7b5a715052e418bb`。
量化干净源码 Paper E2E 通过；现有 V23 API/PWA `b8c421bbb36483aa94c740e9171527ddae0fe073` 保持原状。
缺口是 `YILONG_PROJECT_RELEASE_TOKEN` 的现有安全加载入口，而不是签名文件；
不得新建账号、会话、owner token 或数据库记录绕过发布鉴权。发布前须满足 `HEAD=SourceGitSha`，
后续文档提交之后应重新构建或回到内嵌来源的干净工作树，不能伪写来源 SHA。
双 APK 功能登记维持 `verified`，不以主 APK 单端发布标记整体已发布或 Goal 完成。

## 后续验收

1. 用户提供现有项目编辑者发布凭据的安全加载入口后，受控上传量化正式包并完成目录下载哈希复验；不能发布 Debug 包代替。
2. 用户确认私有接口保护方案后另批配置并验收，不把 HTTPS 域名猜测成已批准环境。
3. 两个正式 APK 完成安装更新、当前账户确认、取消/切换/过期/后台/旋转/重建实测。
4. 历史付款核对与真实入账属于独立审批流程；本批不能证明实际付款资产、发行或兑付已完成。

本批按交付技能拆分传输、协议、身份、生命周期和 UI；UI 技能限定隔离模拟器，
未取得图像证据时不宣称视觉通过。没有参考图，FitRun/loss/threshold 不适用。
原生 IPC 无网页对应物；PWA 不新增敏感桥接，公共行情与已有投影保持原状。

## UI 工作台回执

`FIT_RUN_STATUS=not_run`，`FINAL_VISUAL_LOSS=null`，`VISUAL_ACCEPTANCE_THRESHOLD=null`；
`CROSS_PLATFORM_VISUAL_PARITY=not_verified_native_only`，`BUSINESS_DELIVERY_READY=false`，
`PLATFORM_EVOLUTION_PENDING=false`，`EVOLUTION_THREAD=none`。
双端收尾工具仍报告 Runtime 准备未完成，尚无真帧或双 APK 往返证据；不宣称视觉通过，
不将其误报为平台能力缺失，不启动平台改造或操作真机。
