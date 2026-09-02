---
version_status: current
reviewed_at: 2026-09-02
release_status: released
supersedes_release_status: docs/esk-paper-quant-allocation-request-v2-acceptance.md
---

# ESK Paper 量化分配申请 V2 生产发布回执

本回执覆盖验收快照中的 `verified_pending_production_publish` 状态；不改写该验收快照绑定的源码、测试和隔离运行证据。

## 发布结果

- 主项目提交：`3d7b568f45d1bc3710dfa4708f142ab37d500865`。
- 服务器分配版本：`v0.3.1715`，版本号未写回 Git。
- 标准 `scripts/publish-server.ps1` 完成交叉编译、上传、服务切换和 smoke。
- 远端 `/health` 返回正常，`/api/server/version` 返回 `v0.3.1715` 和同一 Git SHA。
- PC 页面与后端 API 由同一服务器发布流程交付。

## 仍然关闭

本次发布没有导入真实用户数据，没有发行链上 ESK，没有移动资金，没有创建量化仓位、连接交易所或生成收益。量化子项目 V2 兼容代码虽已推送，但独立公网 HTTPS origin 尚未批准和部署。
