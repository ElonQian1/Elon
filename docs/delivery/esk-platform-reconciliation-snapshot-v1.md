---
title: "ESK 正式付款占用快照 V1 交付证据"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets]
---

# ESK 正式付款占用快照 V1 交付证据

需求：[V1](../requirements/esk-platform-reconciliation-snapshot-v1.md)。
操作：[快照接入手册](../esk-platform-reconciliation-snapshot.md)。
本文件只记录已执行结果，不授予入账或资金操作权限。

## 当前证据

| 层次 | 已执行结果 |
| --- | --- |
| 离线桥接 | `node scripts/test-esk-platform-reconciliation.js`，41 项通过 |
| 原预演回归 | `node scripts/test-esk-paid-reconciliation.js`，80 项通过 |
| Rust Store / CLI | 独立 harness 120 项通过，含新快照 12 项及真实 Store → 实际 Node CLI |
| 生产 Router / 本地 TCP | `elon-server esk_asset::platform` 43 项通过，含新增 9 项 HTTP 和真实 loopback |
| 提交 / 推送 / 后端发布 | 待完成，以后续本文件及发布回执为准 |
| 真实用户与私人读取 | 未执行 |

资产合同与 Sui 创世基础静态回归通过；不代表执行 Move 或链上发行。
4 份本次文档的 11 个本地链接已核对存在。新增实现/测试拆为独立小模块，
暂存源码体积和文档模块化检查通过；未扩大已超预算的 `AI_INDEX.md`。

Rust harness 使用官方验证入口、共享 `validation-heavy` 分区和本地依赖缓存。
验证指纹：`68f36fe6a124b57e769415cd743ddfdfd15ae79ed1bb11f873a1e0cc1ecbc3fc`。
回执：`e6c607ffb0871935cf797e9fe1b7bd5c79d9ff5a5094aac3f9a17fdddd11090a`。
缓存 doctor 健康但有磁盘空间预警，本轮未清理任何共享缓存或其他任务数据。

服务器验证指纹：`44d6695ee2b59fc54f5eefc700004d75ec8a3521975d6fddce483c0548d6d414`。
回执：`5814ce0911b6e54984d5d85b9dc46add44bc6d27855234be35245bce95e0a529`。
官方测试命令过滤到平台模块，43 通过、2333 未选中，不宣称整个服务测试集全跑。
HTTP 比较全部 ESK、用户、会话业务列，排除通用认证原有 `last_seen_at` 更新；
Store 独立测试另比较合成 SQLite 文件字节，证明快照事务自身不写数据。
发布前匿名基线：后端 `0.3.1727`，新端点返回 404；不是本轮已部署证据。

## 实现边界

同一 SQLite 只读快照验证固定政策和账本关系，导出未取消准备及已入账付款键。
10000 键上限明确失败，不截断；纯取消释放，重新准备再次占用。
离线桥接保留原人工历史完整性、来源、时效及重复检查，不把摘要当作签名。
未生成登记文件、未开启生产政策、未读取真实付款或用户、未写余额、未移动资金。

主项目沿用现有服务器与认证；本切片不发布 APK、不重启旧版兼容路线。
新版子 APK 签名发布、受保护运营访问、真实对账审批和 Sui 上链仍分别待验收。
