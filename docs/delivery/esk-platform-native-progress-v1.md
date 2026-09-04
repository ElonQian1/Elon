---
title: "ESK 正式进度跨 APK V1 交付证据"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, quant-android]
---

# ESK 正式进度跨 APK V1 交付证据

需求真源：[正式进度 V1](../requirements/esk-platform-native-progress-v1.md)；
线格式：[独立原生合同](../contracts/esk-platform-android-progress-v1.md)。
本文件只记录实现与验收，不定义生产政策，不提升资金操作权限。

## 状态矩阵

| 能力 | 实现 | 验证 | 发布 | 用户验收 |
|---|---|---|---|---|
| 同快照额度/最多 20 条进度共享合同 | implemented | offline_passed | not_started | pending |
| 主 APK 当前账户逐页同意与只读提供 | implemented | offline_passed | not_started | pending |
| 量化独立原生接收、清页与短期显示 | partial（待实际最低主版本） | offline_passed | not_started | pending |
| 网页 APK-only 说明、无私有网页写入 | implemented | offline_passed | not_started | pending |

## 当前基线与职责

主代码任务基线 `77e4e13176e0cd930fb821c274e47f956c18d0d3`；独立量化基线
`72ed8eac1edd06f2be5e7ab08bf5c622600d55cf`。主 `esk/platform/progress` 仅调用已有
`EskPlatformSellbackClient.page`，不改变 Store/Router、财务政策和账本。
量化 `assets/progress` 只接收显式主授权结果，不连接主私有 HTTP API。

共享协议为 35 个固定 String 加每行 6 个 String，最多 20 行；最多 155 项、
单值 128 字符、UTF-8 合计 32768 字节。显示数量来自一次同快照响应，
可申请量不是可兑付现金。每页重新确认主账户、使用新 nonce、先清旧页、不拼接。
409/快照变化不自动去掉游标重试；60 秒以内失效，后台或重建清空。
旧 Paper 17 字段与正式总量 21 字段生产合同及共享测试保持不变。

## 验证入口

- Android：两仓各自 JUnit 包 `com.elon.eskcontract.*`；主端
  `com.elon.app.esk.*`；量化全部 Android 合同和生命周期测试。
- `node scripts/check-esk-platform-progress-parity.js <独立量化根>`：两仓共享生产文件、
  测试向量及旧协议逐字节核对，只是源码证据。
- `node scripts/test-esk-platform-progress-web-boundary.js` 和既有 sellback/profile/Web
  回归：正式入口仍是说明，没有网页 IPC、登录凭据或私有写操作。
- 主项目正常发布入口根据本次范围使用移动 PWA 静态模板和主 APK；量化使用自身
  `scripts/validate.ps1` / `scripts/validate-android.ps1` / `scripts/publish-android-to-yilong.ps1`。
  两个项目分别保存实际源码 SHA、版本、大小、SHA-256 与正式签名回执。

实际结果：

- 主 Android XML 共 270 项通过，0 失败、0 错误、0 跳过，包含新增共享 27 项及
  新提供端 19 项；保留原 224 项。Debug APK 构建通过，整轮 170.7 秒。
- 主初次 Gradle 在启动阶段因 Java 本地回环连接失败退出，尚未执行测试；仅给本轮
  子进程指定 `.ai-tmp/java-sockets` 后重跑通过，没有修改全局环境或应用传输保护。
- 量化自身全验为 162 Rust、42 前端通过，生产 PWA 构建通过；Android XML 为
  155 项通过（原 91 + 共享 27 + 接收端 37）、0 失败/错误/跳过，Debug 构建通过。
  未配置正式参数时 Release 仍按预期拒绝。最低主版本待分配，不能签名发布占位配置。
- 九份两仓新旧合同/测试逐字节一致，其中新生产 Contract SHA-256 为
  `241cedbc40992d8559b68d9baf042064d124544383a9fe43c001026f2e70a9a0`，Rows 为
  `3eec4ca250c6f4497b17536fa16f58f15daae644f2182c1f7411e0b2d48deca8`。
- 新网页原生边界、旧卖回网页边界、个人页可见性、移动 PWA 源码四项均通过。
  另官方目录与 PC 项目首页两项通过。
- 独立审阅没有发现新增可复现 P1/P2；这不替代真实 Android 系统验收。

发布与公开验证待后续实际结果补记。所有 fixture 为合成数据，未查询真实用户、
未执行真实钱包签名或交易；APK 构建签名不是资金签名。

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

1. 本轮代码 Owner 完成并复核两仓功能，跑全部适用测试、push、主发布与合法子发布。
2. 量化最低主版本必须使用实际分配并发布的新提供端版本；未分配时失败关闭，
   不把占位值当作可用配置或发布工件。
3. 子 APK 上传依赖现有项目编辑者凭据与受保护连接，不借主管理员、SSH 或数据库绕过。
4. 私有主连接依然 HTTPS-before-token；公共主 HTTP 和 Paper 页面不变。
   未具备连接时只显示不可用，不能证明用户数量已送达。
5. 双正式 APK 经项目广场下载、安装、升级和官方签名兼容后，由本人逐页查看、
   取消授权、后台/旋转、换号与过期验收；没有此证据不能把 pending 改 accepted。
6. 正式付款核对、审批入账、生产政策开启与实际卖回结算另有门禁，不是本批合成测试。

UI 工作台已绑定当前隔离根；目前 BOOTSTRAP、无连接，能力检查为
`PREPARATION_REQUIRED / DEBUG_RUNTIME_NOT_CONNECTED`，无平台能力缺陷证据。
按发布先于可选视觉验收流程继续；未取到真实渲染帧，不宣称视觉验收或真实设备通过。
