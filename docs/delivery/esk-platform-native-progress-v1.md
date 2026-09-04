---
title: "ESK 正式进度跨 APK V1 交付证据"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets, quant-android]
---

# ESK 正式进度跨 APK V1 交付证据

需求真源：[正式进度 V1](../requirements/esk-platform-native-progress-v1.md)；
线格式：[独立原生合同](../contracts/esk-platform-android-progress-v1.md)。
本文件只记录实现与验收，不定义生产政策，不提升资金操作权限。

## 状态矩阵

| 能力 | 实现 | 验证 | 发布 | 用户验收 |
|---|---|---|---|---|
| 同快照额度/最多 20 条进度共享合同 | implemented | offline_passed | 主 1519 已发布 | pending |
| 主 APK 当前账户逐页同意与只读提供 | implemented | offline_passed | 主 1519 已发布 | pending |
| 量化独立原生接收、清页与短期显示 | implemented | offline_passed | 未签名/未上传 | pending |
| 唯一新版入口、退役旧原生桥 | implemented | 两仓离线复验通过 | 量化源码已推送；主 1519 已发布 | pending |
| 网页新版 APK-only 说明 | implemented | 公开 GET 已核验新措辞 | 已发布 | pending |

## 当前基线与职责

主代码任务基线 `77e4e13176e0cd930fb821c274e47f956c18d0d3`；独立量化基线
`72ed8eac1edd06f2be5e7ab08bf5c622600d55cf`。主 `esk/platform/progress` 仅调用已有
`EskPlatformSellbackClient.page`，不改变 Store/Router、财务政策和账本。
量化 `assets/progress` 只接收显式主授权结果，不连接主私有 HTTP API。

共享协议为 35 个固定 String 加每行 6 个 String，最多 20 行；最多 155 项、
单值 128 字符、UTF-8 合计 32768 字节。显示数量来自一次同快照响应，
可申请量不是可兑付现金。每页重新确认主账户、使用新 nonce、先清旧页、不拼接。
409/快照变化不自动去掉游标重试；60 秒以内失效，后台或重建清空。
第一阶段保留旧原生协议；用户 2026-09-05 明确取消旧版兼容，当前已退役旧 Paper
17 字段和正式总量 21 字段原生桥。历史账本、主个人登记历史/卖回与公共行情不删除。
新版不显示旧审核分录笔数，该数仍在主资产页；卖回申请数不能替代审核分录数。

## 验证入口

- Android：两仓各自 JUnit 包 `com.elon.eskcontract.*`；主端
  `com.elon.app.esk.*`；量化全部 Android 合同和生命周期测试。
- `node scripts/check-esk-platform-progress-parity.js <独立量化根>`：两仓共享生产文件、
  测试向量逐字节核对，并检查旧共享生产合同/专属测试不存在；只是源码证据。
- `node scripts/test-esk-platform-progress-web-boundary.js` 和既有 sellback/profile/Web
  回归：正式入口仍是说明，没有网页 IPC、登录凭据或私有写操作。
- 主项目正常发布入口根据本次范围使用移动 PWA 静态模板和主 APK；量化使用自身
  `scripts/validate.ps1` / `scripts/validate-android.ps1` / `scripts/publish-android-to-yilong.ps1`。
  两个项目分别保存实际源码 SHA、版本、大小、SHA-256 与正式签名回执。

第一阶段实际结果（旧桥删除后的验证另记，不能沿用此测试数量）：

- 主 Android XML 共 270 项通过，0 失败、0 错误、0 跳过，包含新增共享 27 项及
  新提供端 19 项；保留原 224 项。Debug APK 构建通过，整轮 170.7 秒。
- 主初次 Gradle 在启动阶段因 Java 本地回环连接失败退出，尚未执行测试；仅给本轮
  子进程指定 `.ai-tmp/java-sockets` 后重跑通过，没有修改全局环境或应用传输保护。
- 量化自身全验为 162 Rust、42 前端通过，生产 PWA 构建通过；Android XML 为
  155 项通过（原 91 + 共享 27 + 接收端 37）、0 失败/错误/跳过，Debug 构建通过。
  未配置正式参数时 Release 仍按预期拒绝。最低主版本已按实际发布固定为 1517，
  再次完整运行 Android 155 项和 Debug 构建通过，19 秒；不是待替换占位配置。
- 九份两仓新旧合同/测试逐字节一致，其中新生产 Contract SHA-256 为
  `241cedbc40992d8559b68d9baf042064d124544383a9fe43c001026f2e70a9a0`，Rows 为
  `3eec4ca250c6f4497b17536fa16f58f15daae644f2182c1f7411e0b2d48deca8`。
- 新网页原生边界、旧卖回网页边界、个人页可见性、移动 PWA 源码四项均通过。
  另官方目录与 PC 项目首页两项通过。
- 独立审阅没有发现新增可复现 P1/P2；这不替代真实 Android 系统验收。

所有 fixture 为合成数据，未查询真实用户、
未执行真实钱包签名或交易；APK 构建签名不是资金签名。

### 已发布的第一阶段主 APK（尚含旧桥）

- `com.elon.app 1.1.1517 (1517)`，代码 `50cd35f7b4c96807323b66f8442eb61e18b3cfa6`
  已推送主线；官方 app-ui receipt 的 mobile_pwa/apk 两阶段均 passed，总耗时 259.7 秒。
- 实际独立下载公开 APK 并与本地 release 对比：39,734,007 字节，SHA-256
  `b746c2bce77530a4461055b21b5f50ecaa43542254af988436f0f570e7406ede`。
  aapt 确认包名与版本；apksigner 验证 v2、单签，证书 SHA-256
  `f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`。
- 来源依据公开 `app/version.json` 与官方 receipt，不冒称 APK 内嵌 Git SHA 已验证。
  静态 PWA 来源同为 `50cd35f7`，运行时摘要
  `ffb1288ba42ae9a4a0cfe7fc31561dcb0b7ef34052645b3e62d8b91ab851c189`。
- 该包仅证明第一阶段发布；旧桥删除的正式新工件另见下方 1519 记录。

### 旧原生桥退役后的验证

- 主端精确删除21个旧native源码、布局及专属测试/脚本；新增4项退役保护测试。
  正式 Android 验证168.8秒退出0，实际XML为198/198、0失败/错误/跳过，Debug构建通过。
  测试数由270减去已退役76项再加4项；没有把旧测试数量冒充新版证据。
- 主个人正式资产、登记历史/卖回、正式/Paper后端账本目录相对此前提交无差异；
  新测试覆盖这些入口仍存在、旧组件/别名和生产合同缺席、新协议拒绝旧字面载荷。
- 量化删除20个旧native文件，首页只有“我的 ESK”；官方Android为69/69、0失败/错误/
  跳过，Gradle20秒，Debug及默认Release拒绝通过。Rust162/前端42输入不变，复用既有验证。
- 新版5份共享文件跨仓字节一致，旧共享4文件不存在；ContractTest SHA-256 为
  `a7c4eb501798690f7b8c207a839f5403546f852d0cc49794ff0499036eebd73e`。
- 新/旧网页边界、个人资产可见性、PWA源码、官方目录和PC项目首页六项通过；
  暂存源码尺寸和文档模块化门禁通过，AI_CURRENT接近大小上限仅告警。
- 量化功能提交 `a6a726a6f2bd5414c0b31310ceeffd13b9183f4f` 已推送；后续仅文档/登记
  提交到 `2a37b33d4aa044ce3ef152406d7220c2ecc44624`，没有正式0.5包或上传回执。
  新版源码交付不等于广场安装完成；仍需受控签名、上传和本人联调。
- 主提交在真实 non-fast-forward 后仅 rebase 一次，得到正式发布源码
  `21dac2f61e1e2798a51bdfcc2b71b7951c46141b`；新增上游不涉及本批 ESK 输入。
  重验网页/个人资产/PWA/共享文件通过；受影响的 ChatGPT composite-answer、image-assets
  和 tool-execution-smoke-contract 静态测试通过，没有真实发送消息或操作设备。
- 首批用户路线图同步新版唯一入口；观察器源码未变，65 项观察器与 15 项传输离线测试
  复验通过，显式重新登记文档证据，不代表 ESK 发布或余额资格验收。

### 主 APK 1519 与当前广场工件

- 官方 app-ui receipt 绑定 `21dac2f61e1e2798a51bdfcc2b71b7951c46141b`；
  2026-09-05 00:43（UTC+08）两阶段 passed，总耗时 701 秒，含正常发布排队。
  后续文档/功能登记提交不是 APK 的构建源码，不需重发相同业务包。
- 00:50:24（UTC+08）独立实际下载公开 `com.elon.app 1.1.1519 (1519)`，HTTP 200，
  39,715,943 字节，SHA-256
  `36e9fb80a342a3d39adc157956ec21628c6ecf9a61eaaa9fb279bdcc59e25112`，
  与本地 release 逐字节摘要/大小一致；aapt 确认包名和版本，apksigner 验证 v2 单签，
  证书仍为 `f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`。
  公开版本清单与 receipt 的来源均为 `21dac2f6`，不冒称已验证 APK 内嵌源码 SHA。
- 实际 APK Manifest 检查 35 个 activity/alias：新版 `EskPlatformProgressConsentActivity`
  存在且唯一；旧 `EskSnapshotConsentActivity`、`EskPlatformSnapshotConsentActivity`
  及指向它们的别名均为零；主个人资产、审核历史和正式卖回三个 Activity 各保留一份。
  独立核验日志 `esk-retired-public-apk-1519-independent-20260905-004859-281` 为 passed、
  85.1 秒；这证明公开安装包组成，不代表已安装或本人账户验收。
- 公开 PWA 实际 GET 200，显示“新版量化仅保留‘我的 ESK’原生资产入口”、
  “旧版原生接口不再支持”及新版子 APK 尚待正式签名/上传/双 APK 联调的说明。
- 2026-09-05 00:33（UTC+08）实际下载项目广场子 APK 仍为
  `com.elon.quant 0.2.0 (2)`，618588 字节，SHA-256
  `c17ab5abe800547f41acc95594021abb6cec92fc14cb6de3b2db202ce4b94b89`；
  HTTP 200/no-store，官方 v2 单签。没有把旧 0.4 包上传冒充新版，也未删除线上下载。
- 本次收尾只更新两份命中文档，18 条 Markdown 相对链接通过；不全读或整理无关文档。

### 已确认的旧基线测试问题

额外运行的 `scripts/test-quant-esk-asset-projection.js` 与
`scripts/test-yilong-quant-paper-public-deployment-sync.ps1` 失败：前者仍限定两个 capability
和客户端全部 planned，后者要求已删除的旧索引路由及所有端尚未部署。当前 schema 已有
V3 单申请授权，V20 公开 HTTP Paper 与 Android 已 available。相关脚本、schema、目录、
启动组件相对本轮基线无差异，旧索引缺项也已在基线存在；不是本轮 progress 引入。
未删断言或改生产状态使它们转绿，不作为本批通过证据。后续需重新认领原功能，
按已接受的 V3/V20 需求维护测试，并标明 V9 文档的历史适用范围；原 HTTPS 私有授权
门禁不能因公开预览已上线而移除。

## 剩余验收与远程协作

1. 两仓代码已推送、主新版已发布；量化 Owner 接续受控正式签名与项目编辑者上传。
2. 量化最低主版本为已真实发布的新协议首发 1517；后续退役旧桥不改变新协议。
3. 子 APK 正式签名输入与现有项目编辑者上传凭据当前未安全注入；只查存在性布尔，
   不读取秘密文件，不借主管理员、SSH 或数据库绕过。0.5 Debug 不是正式上架包。
4. 私有主连接依然 HTTPS-before-token；公共主 HTTP 和 Paper 页面不变。
   未具备连接时只显示不可用，不能证明用户数量已送达。
5. 双正式 APK 经项目广场下载、安装、升级和官方签名兼容后，由本人逐页查看、
   取消授权、后台/旋转、换号与过期验收；没有此证据不能把 pending 改 accepted。
6. 正式付款核对、审批入账、生产政策开启与实际卖回结算另有门禁，不是本批合成测试。

UI 工作台已绑定当前隔离根；目前 BOOTSTRAP、无连接，能力检查为
`PREPARATION_REQUIRED / DEBUG_RUNTIME_NOT_CONNECTED`，无平台能力缺陷证据。
按发布先于可选视觉验收流程继续；未取到真实渲染帧，不宣称视觉验收或真实设备通过。
1517 发布后的一次可选模拟器调用观察超时；随后只读状态仍 BOOTSTRAP、无帧/源码证明。
没有重启或绕过工作台；完成门禁 businessDeliveryReady=false、completionReady=false、
platformEvolutionPending=false；1519 发布后再次只读检查仍为同一准备缺口。
FIT_RUN_STATUS=NOT_REQUIRED_WITHOUT_CLEAN_TARGET；
FINAL_VISUAL_LOSS/VISUAL_ACCEPTANCE_THRESHOLD=unavailable；
CROSS_PLATFORM_VISUAL_PARITY=unverified；REAL_DEVICE_STATUS=not_requested；
ANDROID_RENDERER=VERIFICATION_DEFERRED；EVOLUTION_THREAD=none。仓库发布与视觉分开验收。
