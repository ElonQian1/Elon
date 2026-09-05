---
title: "官方量化 APK 新版发布准入 V1 交付证据"
version_status: current
reviewed_at: 2026-09-05
implementation_status: verified
---

# 官方量化 APK 新版发布准入 V1 交付证据

## 交付结果

主项目已把一龙量化 Android 发布路径收紧为新版唯一合同。精确项目
`yilong-quant` 只接受 `com.elon.quant`、`paper`、规范版本名不低于
`0.5.0`、`versionCode >= 5` 和 40 位小写源码 Git SHA。服务器从上传字节
计算 APK SHA-256 与大小，并在创建目录前检查 ZIP、二进制 Manifest、主 DEX
及紧邻中央目录的 APK v2/v3 签名块结构。

历史 `0.2.0 (2)` 已从官方目录撤下。旧记录即使继续留在数据库供内部审计，也不会
进入发布列表、latest、稳定文件下载、发布 ID 下载、任务克隆、项目首页同步或项目
广场安装入口；面向用户的产品面只展示通过当前合同的实际新版。

本门禁只证明 APK 签名块结构存在，不做密码学验签。主 Android 仍负责最终核对
精确包名、唯一既有发布证书和最低版本，服务器回执不能替代设备安装验证。

## 状态矩阵

| 能力 | 实现 | 验证 | 当前交付状态 |
| --- | --- | --- | --- |
| 上传前元数据与 APK 结构门禁 | implemented | passed | 已随 Server 0.3.1729 部署 |
| 版本单调、同工件幂等、冲突拒绝 | implemented | passed | SQLite 事务内执行 |
| 旧量化发布退出列表/latest/下载/同步 | implemented | passed | 线上项目投影已无旧 APK |
| PC 正式发布操作面与回执展示 | implemented | passed | server_bundle 已发布 |
| 正式量化 `0.5.0 (5)` APK | not_provided | not_run | 仍需既有证书签名和编辑者上传 |
| 项目广场真实安装与双 APK 本人验收 | not_performed | not_run | 等待正式新版工件 |

## 实现证据

- `project_releases/admission.rs` 解析规范三段版本、精确身份、Git SHA、EOCD、中央
  目录、双 size 签名块和 v2/v3/v3.1 ID-value 结构；ZIP comment 或普通 entry 中
  的 magic 不能冒充签名块。
- 上传 handler 先取得不可伪造的进程内 `ValidatedOfficialQuantApk` 能力令牌，才
  创建发布目录；令牌绑定服务器计算的摘要和大小，普通 Store 调用没有本次工件的
  精确令牌时不能登记官方量化发布。
- Store 使用 `Immediate` 事务串行检查相同版本和相同工件。完全相同的重试返回原
  `release_id` 并清理新暂存目录；降级、同版本异工件及同工件改报版本分别失败。
- 新记录写入绑定摘要与大小的服务端准入回执；latest、列表和下载投影同时要求当前
  元数据、工件字段及该回执。手工遗留的“看似 V5”行也不能自动成为可安装版本。
- 发布 ID 下载新增 published/新版资格、托管根路径、大小和 SHA-256 复核，错误不再
  把内部文件路径返回给客户端。
- 历史文件名下载路由在任何工作区回退前进入官方专用路径，只接受当前准入版本并复核
  托管根、大小和摘要；同版本健康重传还能恢复丢失或损坏的原托管文件。
- 项目广场“可安装”筛选只承认绑定服务端回执的量化发布；任务 APK 自动同步对该项目
  直接停止并引导正式发布，不再制造实际 404 的安装地址。
- PC 页面为官方量化固定包名和 paper 渠道，要求版本码与源码 SHA；普通项目既有
  stable/beta/internal 行为保持。APK SHA 仍只由服务器计算，浏览器不能自报。

## 验证记录

| 验证 | 结果 | 证据 |
| --- | --- | --- |
| APK/Store/SQLite/可安装筛选隔离 Rust harness | `10/10 passed` | 指纹 `da59c1789264b5d4450c42b062159374139dad109af224473803e613d49e3360` |
| Server `elon-server` 非测试编译检查 | passed | 指纹 `e1372ebab7d5b0204d539fea12aa4af3129ce8cc126868552e6ae4dae0397ba6` |
| 官方目录、PC 上传、服务端源码合同 | passed | 旧目录字段、上传顺序、事务、下载门禁与 UI 字段均受约束 |
| PC TypeScript 与生产构建 | passed | `test:project-home` 和 `npm run build` 同批通过 |
| Server 全量 test target | baseline blocked | 指纹 `b762a46c3381da8c232dee0aaebe8ddf8bae5d3fef4cb4d574adfbb77fbef998`；失败位于未改动的 node-agent VFS 测试可见性/表达式 cfg，不在本批文件 |

隔离 harness 只临时引用本批真实 Rust 模块并使用临时 SQLite；验证后已删除，不进入
仓库或产品。全量测试目标的基线债务没有被伪装成通过，也没有在本功能中顺手改写。

## 线上发布回执

- 主服务器 `0.3.1729` 已从提交
  `5d2abd409736a58a9fa41ca937bd44e56b74ab1e` 发布，`/health`、服务器版本接口、
  PC `server_bundle` 发布标记与 React 页面均通过检查。
- 公开项目列表中 `yilong-quant.latest_apk_url = null`；`has_apk=true` 的可安装筛选不
  包含一龙量化。因此旧包已退出实际项目广场，而不是只修改说明文字。
- 这份回执证明新版准入门禁已经上线，不代表正式量化 APK 已经上传。当前继续显示
  “暂无可安装新版”是预期结果。

## 发布与边界

本批不读取或创建 Android 签名密钥，不上传 APK，不移动 ESK、SUI、USDT，不连接
币安或执行交易。下一步只能使用量化子项目既有正式证书构建 `0.5.0 (5)` 或更高
版本，由有编辑权限的人员在 PC 发布页上传；取得服务器回执后再执行项目广场安装、
打开、登录及本人资产验收。Debug 包、新证书、旧 0.2 包或只有目录文字都不能替代。
