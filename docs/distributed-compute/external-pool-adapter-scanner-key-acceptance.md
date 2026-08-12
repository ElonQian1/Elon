---
title: 外部矿池 Adapter 漏洞扫描器信任根验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 漏洞扫描器信任根验收

## V235 定向验证

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v235 -- test --manifest-path server\Cargo.toml --bin elon-server scanner_key_ -- --nocapture --test-threads=1
```

结果：5 项命名测试通过，包括 migration、Store 生命周期、并发激活、双向角色隔离和 HTTP 生命周期。

验收指纹：`a0bc1096e5835e1309ffd1cb8904ef4859261c3dee4ea764640f4b399ff882ab`。

证据：`D:\rust\shared\rust-cache-v2\validation-v1\evidence\a0bc1096e5835e1309ffd1cb8904ef4859261c3dee4ea764640f4b399ff882ab\summary.json`。

覆盖：

- migration 可重复执行，文件数据库关闭后可重新打开并重建 currentness；
- registration、activation、revocation 的 canonical 摘要与精确幂等重放；
- 登记者不能自激活，两连接并发激活只有一个成功；
- root、activation 和 revocation SQL 行不可更新或删除；
- 同一 RSA key 无论先登记为供应商签名钥还是扫描器钥，第二种角色都失败关闭；
- 未认证、普通成员和错误角色被拒绝，管理响应不泄露 PEM 或幂等材料；
- source size、Rust format 与 `git diff --check` 通过。

## V230 受影响回归

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v230-regression -- test --manifest-path server\Cargo.toml --bin elon-server artifact_signing_key_ -- --nocapture --test-threads=1
```

结果：5 项 V230 命名测试通过。

回归指纹：`a7ba0663858feeecfae11654014ca749288d79bcd7bd383dda1915195a0ee556`。

证据：`D:\rust\shared\rust-cache-v2\validation-v1\evidence\a7ba0663858feeecfae11654014ca749288d79bcd7bd383dda1915195a0ee556\summary.json`。

新增反向角色隔离触发器未破坏既有供应商签名钥的登记、四眼激活、吊销、并发线性化、HTTP 权限和脱敏合同。

## 未验证

- 真实漏洞扫描器进程、依赖解析器、CVE 数据源和情报镜像；
- V236 已完成已签报告和最长 168 小时 snapshot currentness；真实情报内容、离线镜像更新和生产签名密钥托管仍未验证；
- 动态恶意行为、sandbox conformance、credential verifier runtime；
- 生产数据库原位升级、真实 TCP、并发压力、备份恢复和部署；
- MCP/PC 管理面、Adapter 安装/采用、route、Worker/ACK、真实派发和结算。

因此 V235 只能证明独立 scanner trust root 生命周期，不能证明任何 Adapter 已扫描、无漏洞或可执行。
