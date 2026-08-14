---
title: 咖啡商户公网 HTTPS 订单纵向验收
status: current
owner: backend
reviewed_at: 2026-08-14
document_type: acceptance_evidence
---

# 咖啡商户公网 HTTPS 订单纵向验收

## 验收结论

2026-08-14，一龙当前源码中的确定性消费者 AI 路径通过真实 HTTP 路由，先从当前隔离运营方目录发现 `catalog.search` 公开能力，再分别发现 `order.quote.create`、`order.commit` 和 `order.status.read` 三项授权能力；它查询真实商品、提交只包含这三项订单权限的受限授权申请，由商户侧登录会话批准限时 Grant，再经服务端动作确认和商户运行时出站安全模块，通过公网标准 443 HTTPS 调用 `cofficethinking` 真实服务器，并在该商户真实 PostgreSQL 和统一 ERP 订单接口中完成一笔未支付订单。

本次证明的是“一龙平台当前代码可让消费者 AI 通过目录发现、授权、确认和调用链连接公网商户节点并落入真实 ERP”的纵向子链。执行者仍是确定性测试身份和明确查询条件，不是完整生产平台上线、真实支付或真实 LLM 无人干预自主购买的证明。

```text
消费者 AI 测试身份
  -> 一龙真实 Axum HTTP 路由
  -> 隔离运营方目录发现
  -> 公开 catalog.search 查询真实商品
  -> 三项订单权限的受限授权申请与商户侧批准
  -> 限权 oc_live_ 生产凭据
  -> Grant 与服务端动作确认
  -> 公网标准 443 HTTPS + HMAC
  -> 咖啡商户真实运行时
  -> 真实 PostgreSQL
  -> 统一 ERP 订单 API 回读
```

## 隔离边界

- 商户端使用真实服务器、真实 Nginx、真实 TLS 和真实 PostgreSQL。
- 商户固定为 `merchant-cofficethinking-acceptance`。
- 门店固定为 `a11c0000-0000-4000-8000-000000000001`，执行前必须为 `suspended`。
- 一龙平台端使用当前源码启动真实 Axum HTTP 路由，但平台数据写入隔离的临时 SQLite，不触碰线上一龙账户或生产数据库。
- 开发者资料、域名和准入状态由既有生产凭据测试夹具建立，不代表真实工商核验或外部域名审核。
- 每次完整验收会在暂停门店新增一个验收商品、报价和未支付订单，必须显式传入确认开关；一旦准备回执取得商品身份，后续成功或失败收尾都会通过商户管理 API 下架该商品，并保持订单扣减后的实时库存不变。若远端连接在返回商品身份前中断，脚本无法凭空定位该商品，仍需按运行时 SKU 人工审计。
- 共享密钥只从咖啡服务器 `.env` 读取到当前进程，日志、回执和仓库均不保存明文。
- 本次没有调用支付、退款、结算或 Sui 适配器，所有平台计量回执固定 `funds_moved=false`。

## 公网入口

- 运行地址：`https://182.254.168.75`，使用受信任的短期 IP 证书。
- Nginx 只向公网代理 `/health`、`/commerce/v1/manifest` 和 POST `/commerce/v1/invoke`；其他路径返回 404。
- 80 端口只保留 ACME challenge；业务入口使用标准 443。
- 外部健康检查返回 `200` 和 `{"status":"ok"}`。
- Manifest 返回 `merchant_runtime.manifest.v1`、固定验收商户和 7 项能力。
- `nginx -t` 与 `certbot renew --dry-run --no-random-sleep-on-renew` 均通过；证书续期后会重新加载 Nginx。

## 已验证链路

1. 一龙为隔离开发者 App 签发一次性显示、数据库仅存摘要的 `oc_live_` 凭据，范围只有 `catalog.search`、`order.quote.create`、`order.commit` 和 `order.status.read`。
2. 一龙登记固定公网运行地址和密钥引用，执行 Manifest/HMAC 健康验证并得到 `runtime_status=active`。
3. 商户发布公开 `catalog.search` 和三项授权订单能力；消费者先按查询词从目录发现商品查询能力，再经真实公网调用返回本轮 SKU 对应的商品 ID，测试不把预先创建的商品 UUID 直接注入订单参数。
4. 消费者分别从目录发现 `order.quote.create`、`order.commit` 和 `order.status.read`，而不是假设同一目录响应会返回同商户的全部能力。
5. 消费者通过 HTTP 提交只包含三项订单权限的受限授权申请；商户侧使用已登录会话读取并批准申请，签发只覆盖三项订单能力、最多 3 次调用、6000 CNY 微单位和一小时有效期的 Grant。
6. 消费者应用通过一龙 HTTP 路由创建报价；`order.commit` 先准备动作确认，再由用户确认短语绑定原输入，没有确认回执不能提交。
7. 一龙经公网 HTTPS 转发已签名信封，咖啡运行时按服务端报价事务扣减库存并创建开放商业订单和统一 ERP 订单。
8. 相同提交幂等键重放返回同一 Invocation 和订单，库存不再扣减。
9. `order.status.read` 返回同一开放订单、同一统一 ERP 订单和 `unpaid`；消费者订单闭环接口返回同一商户业务回执，平台资金移动仍为 false。
10. 独立只读脚本从 PostgreSQL 反查商品、报价、开放订单、统一 ERP 订单和调用回执，并从原 ERP `/api/v1/orders` 回读同一订单。

## 本次证据

| 项目 | 结果 |
|---|---|
| 运行 ID | `1786678577266` |
| 目录发现能力数 | `4` |
| 排序策略 | `merchant_name.v1`，非付费 |
| 授权申请 | `authreq_06bef1c839e5482fa7e0bfaffaa1763c`，已批准 |
| Grant | `grant_400b74d45a9549cc99a03254329234e1` |
| 验收商品 | `d2d6aa4b-f79b-4567-8f5f-9ae3a2fadf72` |
| 开放商业订单 | `05a4ba51-f3bd-4551-96bd-ef6b4033eb05` |
| 统一 ERP 订单 | `3cdf3d00-7f31-4a6e-90de-7283ed6cfd1f` |
| 库存 | `5 -> 4` |
| 验收商品收尾 | 已下架，库存保持 `4`，未恢复为 `5` |
| ERP API 命中数 | `1` |
| 提交调用回执数 | `1` |
| 幂等重放 | 同一 Invocation，未重复扣库存 |
| 支付 | `unpaid` |
| 资金移动 | `false` |
| Rust 验证 | `CARGO_OK` |

临时 JSON 回执位于任务工作树 `.ai-tmp/`，属于一次性证据，不提交 Git。上述稳定结论与非敏感业务 ID 才进入本文档。

## 可重复入口

咖啡仓 `master` 提交 `db88720` 已固化非 Docker 公网入口：

- `scripts/configure_open_commerce_public_https.sh`：幂等配置短周期 IP 证书、受限 Nginx 反向代理和续期 reload hook；
- `docs/open_commerce_public_https.md`：记录真实服务器前置条件、最小公网路径、网络边界和验收结论。

该提交是部署方法和边界的仓库证据，不表示脚本本身替代了本次真实运行结果，也不表示咖啡仓与一龙仓已经合并为同一个代码库。

完整验收会写入一笔真实未支付订单：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\test-open-commerce-public-https-coffee.ps1 `
  -CoffeeRepo D:\rust\active-projects\cofficethinking `
  -AcknowledgeProductionWrite
```

若平台调用已经成功、仅最后核对中断，必须使用回执恢复只读验证，不能重跑下单：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\verify-open-commerce-public-https-coffee.ps1 `
  -CoffeeRepo D:\rust\active-projects\cofficethinking `
  -ReceiptPath <task-worktree>\.ai-tmp\open-commerce-public-https-<run-id>.json
```

## 尚未证明

- 一龙线上生产部署、线上生产数据库和真实平台账户共同完成同一链路；
- 消费者 AI 跨运营方、跨项目或面向全网的穷尽发现与互操作；本次只验证当前隔离运营方目录和固定查询条件；
- 真实 LLM 在无人代填查询条件、授权范围或购买参数的情况下自主做出购买决定；
- 真实企业资料、域名、主体准入和生产 App 审核；
- 支付、退款、履约、配送、结算或链上资产；
- 美团、抖音、京东、淘宝闪购等官方生产适配器；
- 域名多地址故障切换、长期压力、证书真实自动续期后的持续可用性和网络级 egress 防火墙。

下一步应先把咖啡实例正式绑定到一龙平台项目，再用可持久化测试账户走“目录发现 -> 授权 -> 确认 -> 下单 -> ERP 回读”，仍保持未支付和人工确认边界。
