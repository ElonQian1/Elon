---
title: 外部矿池 Adapter Artifact 静态安全证明验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 静态安全证明验收

## V233 定向验证

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v233 -- test --manifest-path server\Cargo.toml --bin elon-server artifact_security_ -- --nocapture --test-threads=1
```

结果：`3 passed; 0 failed; 1738 filtered out`。

验收指纹：`f18c02e0cd10b3c172fd14492017912abf955b2758e5901faf91925f185c6c08`。

证据：`D:\rust\shared\rust-cache-v2\validation-v1\evidence\f18c02e0cd10b3c172fd14492017912abf955b2758e5901faf91925f185c6c08\summary.json`。

覆盖：

- V233 migration 可重复执行；
- V222 -> V227 -> V230 -> V231 -> V232 后，有效 canonical SBOM 可生成不可变 V233 receipt；
- exact 幂等重放、admin/owner 权限、普通成员拒绝及 API 脱敏；
- admission 终态后只保留 `historical_only`；
- 非允许许可证、SBOM 文件归属缺口及嵌入私钥均失败关闭且不产生收据；
- source size guard、Rust format 与 `git diff --check` 通过。

## V232 受影响回归

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v233-v232 -- test --manifest-path server\Cargo.toml --bin elon-server artifact_package_ -- --nocapture --test-threads=1
```

结果：`3 passed; 0 failed; 1738 filtered out`。

回归指纹：`8f9abe93b285ed62ec0c63c8f9ba502b84745754e5351d536d1666ef7a4e3c97`。

证据：`D:\rust\shared\rust-cache-v2\validation-v1\evidence\8f9abe93b285ed62ec0c63c8f9ba502b84745754e5351d536d1666ef7a4e3c97\summary.json`。

新增 SBOM 测试制品未破坏 V232 的 canonical ZIP、manifest、路径、身份、大小写冲突及压缩炸弹边界。

## 未验证

- 真实第三方 Adapter 包、供应商构建流水线和真实 SBOM 生成器；
- 依赖图解析、CVE/漏洞情报、情报新鲜度、签名扫描证明和离线镜像更新；
- 动态恶意行为、隔离沙箱 conformance、credential verifier 运行时；
- fuzz/property、极限并发、磁盘故障、断电及进程崩溃；
- 生产数据库原位升级、真实 TCP、部署、浏览器、MCP 与 PC 控制面；
- Adapter 安装/采用、v213 route、Worker/ACK、真实派发、计量和结算。

因此当前状态是 `implementation_partially_verified`，不是生产 Adapter 可用，也不代表整个 Goal 已完成。
