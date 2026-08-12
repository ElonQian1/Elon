---
title: 外部矿池 Adapter Artifact 静态包格式证明验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 静态包格式证明验收

## 已验证

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v232 -- test --manifest-path server/Cargo.toml --bin elon-server artifact_package -- --nocapture
```

结果：`3 passed; 0 failed; 1735 filtered out`。

覆盖：

- v232 migration 连续执行两次，表和 current view 唯一存在；
- v222 -> v227 -> v230 -> v231 的真实 2048 位 RSA 来源链后，canonical ZIP/manifest 可生成不可变 v232 receipt；
- exact 幂等重放返回同一 receipt，不产生第二条记录；
- 未登录、普通 member、admin/owner 的 HTTP 权限边界；
- API 不泄露 manifest JSON、入口路径、文件清单、verifier 对象、签名、候选引用、幂等材料或服务器路径；
- admission revoked 后 currentness 为 `historical_only`，历史收据不被改写；
- CAS 文件删除后 GET 失败关闭；
- `../` 路径、manifest Adapter 身份漂移、大小写冲突路径和高压缩比炸弹均返回 `422`，不产生格式收据；
- source size guard、Rust format 和 `git diff --check` 通过。

最终验收指纹：`28492d2b3eff802807f8d22a049fe214be5e6dc50d4cab19ba4b9d437eb7cb92`。

证据：`D:\rust\shared\rust-cache-v2\validation-v1\evidence\28492d2b3eff802807f8d22a049fe214be5e6dc50d4cab19ba4b9d437eb7cb92\summary.json`。

## 未验证

- 真实第三方 Adapter 包和供应商构建流水线；
- fuzz/property 测试、极限并发、磁盘故障、断电和进程崩溃；
- 生产数据库原位升级、真实 TCP、部署、浏览器、MCP 与 PC 控制面；
- SBOM、安全扫描、sandbox conformance、credential verifier 运行时；
- Adapter 安装/采用、v213 route、Worker/ACK、真实派发、计量和结算。

因此当前状态是 `implementation_partially_verified`，不是生产 Adapter 可用。
