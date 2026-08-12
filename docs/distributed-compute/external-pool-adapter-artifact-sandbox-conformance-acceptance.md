---
title: 外部矿池 Adapter 动态沙箱符合性证据验收
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 动态沙箱符合性证据验收

## V239 编译

运行 `cargo check`，结果通过。该轮用于发现并修复服务层请求所有权错误；最终定向测试随后重新编译 `elon-server` test target。

## V239 定向测试

运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-federation-v239-tests-fix1 -- test --manifest-path server\Cargo.toml --bin elon-server sandbox_conformance_ -- --nocapture --test-threads=1
```

结果：通过。最终指纹 `6831ee196129ec7b47f109f6461d8fd8c3f29b4a5a088be6d298ca8f56879555`。

专项覆盖：

- migration 连续运行两次，receipt table 和 current view 唯一存在；
- 通过真实 V222、V227-V233、V235-V237 测试链产生上游权威；
- challenge 从 admission 派生恰好六项测试计划，不采用请求方能力列表；
- 独立 V237 RSA key 对 challenge 签名，服务端验签后写入不可变 receipt；
- 精确幂等重放、响应脱敏和历史读回重验；
- 缺少一项能力或任一策略违规均失败关闭；
- verifier key 撤销后 currentness 从 `verified_current` 变为 `historical_only`；
- Rust format、`git diff --check` 和 source-size guard 通过。

## 未验证

- 真实 sandbox/verifier 进程、VM/container 内核隔离、系统调用拦截和恶意制品执行；
- observations、transcript、资源计量和禁网计数的可信采集；
- 多验证者仲裁、透明日志、HSM、远程证明和生产密钥轮换；
- credential verifier、Adapter 安装/采用、Sidecar IPC、v213 route、worker/ACK 和真实外部矿池；
- 生产数据库原位升级、真实 TCP、并发压力、MCP/PC、部署、备份恢复和真实付款。

因此 `signed_sandbox_report_verified_current` 不能表述为“服务器已经真实运行并证明该 Adapter 安全”，更不能表述为“Adapter 已上线”。
