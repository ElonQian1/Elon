---
title: 外部矿池 Adapter 沙箱验证者信任根验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 沙箱验证者信任根验收

## V237 编译

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v237-check-fixed -- check --manifest-path server\Cargo.toml --bin elon-server
```

结果：通过。指纹 `8a1dfdbea20722957a6ec5664204edca72dbd0cd7b5a1e778f7b5513113a847a`。

## V237 定向测试

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v237-tests -- test --manifest-path server\Cargo.toml --bin elon-server sandbox_verifier_key_ -- --nocapture --test-threads=1
```

结果：通过。最终指纹 `4c228d770fa74fab46ac6d53c10e5d42cb92d80cc2fad51a71e594ab11b6ea42`。

覆盖：

- migration 连续执行两次后 root、transition 与 current view 唯一存在；
- 未认证和普通成员不能管理验证者密钥；
- 登记者不能自激活，另一名管理员可以激活；
- 相同登记请求精确重放，响应不泄露 PEM 和幂等材料；
- 已登记的 sandbox verifier 公钥不能再登记为漏洞 scanner；
- 激活后可追加撤销，currentness 动态派生为 `revoked`；
- source size、Rust format 和 `git diff --check` 通过。

## 未验证

- 真实 sandbox/verifier 进程、运行策略镜像、内核隔离和恶意行为探测；
- exact V233/V236 绑定、六能力测试向量、动态 transcript 和签名报告；
- HSM、透明日志、多验证者仲裁和生产密钥轮换；
- 生产数据库原位升级、真实 TCP、并发压力、MCP/PC、备份恢复和部署；
- credential verifier、Adapter 安装/采用、route、Worker/ACK、真实派发、计量和结算。

因此 V237 只证明独立沙箱验证者信任根的生命周期，不证明任何 Adapter 已通过动态验证或可生产运行。
