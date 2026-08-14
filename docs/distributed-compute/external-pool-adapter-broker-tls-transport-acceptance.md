---
title: 外部矿池 Adapter 服务端 Broker TLS 传输验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_windows_source_and_local_tls
---

# 外部矿池 Adapter 服务端 Broker TLS 传输验收边界

## 本批状态

V264 已实现 server-owned fresh DNS、公网单播全答案门卫、选定地址直连 TCP、TLS 1.3-only、
WebPKI hostname/time/chain 验证、leaf SPKI SHA-256 constant-time pin，以及 V258 Store 前后
currentness 和完整 installation binding 复验。生产 `cargo check` 通过；最终定向测试
`11 passed / 0 failed`。

测试只连接 Windows loopback 上的本地 TLS fixture。没有访问真实 upstream、使用生产
Secret、发送应用字节、激活 Provider、写经济状态或部署服务。

## 已执行验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- check --manifest-path server\Cargo.toml

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server\Cargo.toml --bin elon-server external_pool_adapter_broker_tls
```

验证统一使用 Rust 共享缓存分区；未建立任务专属 Cargo cache，未使用 Docker。初次测试命令
误带 `--lib`，因本包无 library target 在编译前退出；随后固定到 `--bin elon-server`。测试
fixture 的 Windows 主动断开最初返回 `10053`，最终按“未收到应用字节”的 EOF/timeout/reset
同义边界修正后全量通过。

## 动态与源码合同矩阵

1. 公网地址分类：接受真实公共 IPv4/IPv6，拒绝 private、loopback、link-local、CGNAT、
   documentation、benchmark、mapped-private、multicast 与特殊地址；
2. DNS 全答案门卫：mixed public/private、端口漂移和原始答案超限均失败，合法答案确定性
   去重排序；
3. 正向 TLS：本地测试 CA 签发 `localhost` leaf，TLS 1.3、WebPKI hostname/time/chain 和
   exact SPKI pin 全部通过；
4. 身份失败：hostname mismatch、untrusted CA 和 exact SPKI mismatch 均不形成 channel；
5. 协议失败：TLS 1.2-only server 被 TLS 1.3 client 拒绝；
6. 连接失败：本机拒绝连接不形成 channel；
7. 零应用字节：测试服务端在 channel Drop 前只等待读取，正向和 pin-failure 路径均观测
   零应用 payload；
8. Store 合同：网络 await 位于 preflight transaction 与 postflight `Immediate` transaction
   之间，两份 Prepared handle 独立，完整 installation binding 与 exact target 二次比较；
9. 无副作用：源码合同拒绝应用写入、HTTP、持久化、Provider activation、market、usage、
   settlement 和链上 effect；V258 target 与 V260 session 模块没有反向导入 broker network。

## 验证回执

| 证据 | 指纹或结果 |
|---|---|
| production Rust check | `d2abbd7f6f0f01752c5e98c644fea65f2ccff9e4cc2bba5af1c313bf32e2b629` |
| final broker TLS tests | `acc1163dd5eb4a994eda3c77f975872890aa715b53841e25d921868fab0a24cd` |
| test result | `11 passed / 0 failed / 1951 filtered out` |
| dynamic network scope | local loopback TLS fixture only |

最终测试回执位于机器级 Rust validation evidence 目录，不属于产品状态或发布工件。测试
证书仅为 2026-2046 有效的固定本地 CA/localhost fixture，不进入生产 trust roots。

## 未验收与禁止声明

- 未对生产 DNS、真实公网地址、真实矿池 CA/SPKI、代理环境、IPv4/IPv6 多地址故障转移、
  握手超时和长时连接做目标环境验收；
- 未验证 Linux 生产构建、生产网络 namespace/cgroup 与 server broker 的联合运行；
- 未发送 Adapter-generated no-work request，未读取 response 或生成可持久化 observation；
- 未把 V263 Secret delivery 与 V264 TLS channel 组合到同一真实 Adapter 生命周期；
- 未创建 route/service actor、Provider readiness/activation、market admission、usage、settlement
  或 Sui effect；
- 未开放 HTTP/MCP/PC/APK，未发布服务器或安装包。

因此只能声明 V264 的服务端 TLS transport core 已完成 Windows 源码和本地动态认证验收，
不能声明生产外部矿池已接通、Adapter 能领取任务或算力交易链路已完成。后继 V265 已另行
完成固定本地 fixture 的 bounded no-work seam，见
[`external-pool-adapter-authenticated-no-work-probe-acceptance.md`](external-pool-adapter-authenticated-no-work-probe-acceptance.md)；它不回写或扩大本验收结论。
