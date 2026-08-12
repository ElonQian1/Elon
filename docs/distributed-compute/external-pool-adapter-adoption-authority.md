---
title: 外部矿池 Adapter 采用授权与撤销权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 采用授权与撤销权威

## 1. 目的与边界

V244 把一份当前 V239 动态沙箱符合性回执和一份当前 V243 凭据验证回执汇合为不可变、可撤销的 Adapter 采用授权。它证明平台管理员接受了同一份 V221 onboarding、V222 staged admission、制品实现、能力集和凭据承诺所形成的证据链，可供后续安装事务消费。

采用授权不是安装结果。V244 不修改 Provider，Provider 仍为 `registering`；不写 v213 Adapter/credential/route registry，不加载或执行制品，不读取原始凭据，不创建 worker/ACK、任务执行、计量、结算或付款能力。HTTP 响应中的 `install_effect=authorization_only` 与其它 `none` 效果用于固定这一边界。

## 2. 精确绑定

采用收据同时绑定：

- V221 application、Provider 身份、所有者、策略版本、摘要和 Adapter 配置；
- V222 admission、Adapter release、实现摘要和能力集摘要；
- V239 沙箱符合性回执 ID、摘要和报告到期时间；
- V243 凭据验证回执 ID、摘要、凭据定位符 commitment 和报告到期时间。

写事务必须重新取得 V239 和 V243 私有 current authority，并验证二者的 admission、Adapter ID、release version、实现摘要、能力集摘要和预期凭据验证器完全相同。任何摘要漂移、上游撤销或报告到期均失败关闭。原始非 Bearer 凭据定位符不进入 V244 数据库或 HTTP。

同一 application 与 admission 只能形成一张采用收据，避免同一证据链被重复包装。需要升级时必须形成新的 release/admission 和新的上游验证根。

## 3. 历史、当前性与撤销

采用收据和撤销终态均为追加式记录，禁止更新、删除和 `INSERT OR REPLACE`。历史读取会重新审计 V239、V243 的历史签名根、规范化摘要以及数据库列投影。

采用收据仅在以下条件全部满足时为 `adopted_current`：

- 精确 V239 回执仍为 `verified_current`；
- 精确 V243 回执仍为 `verified_current`；
- 未存在精确绑定的撤销终态。

任一上游报告到期、密钥/实现/Admission/Provider 被撤销或漂移，或者平台管理员追加撤销终态后，采用收据立即降级为 `historical_only`。历史保留不代表仍可安装。

## 4. 管理接口

仅平台 `admin` 或 `owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-adoptions`
- `POST /api/admin/compute/external-pool-adapter-adoptions/:receipt_id/revoke`
- `GET /api/admin/compute/external-pool-adapter-adoptions/:receipt_id/currentness`

创建与撤销均要求显式确认、服务端注入操作人、限定长度的幂等键和精确摘要。响应不返回原始凭据、签名、公钥、测试观察或幂等材料。

## 5. 后续边界

V246 单独建立惰性 Adapter installation authority：消费当前 V244 authority，从 V227 CAS 的同一已复验句柄按 V232 manifest 安全解包并登记不可变安装实例，但仍不激活 Provider、不读取凭据、不启动进程或写 v213。Provider activation、service actor/route authority、真实 Adapter worker、ACK、任务执行与结算继续作为后续独立交付，不能由 V244 或安装目录自动推导。见 [`external-pool-adapter-installation-authority.md`](external-pool-adapter-installation-authority.md)。
