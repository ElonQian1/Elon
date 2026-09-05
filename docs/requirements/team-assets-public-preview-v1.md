---
title: "量化团队资产公开汇总转发 V1"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-public-preview, quant-team-assets]
---

# 量化团队资产公开汇总转发 V1

用户要求在量化 APK 向所有用户公开团队币安资产，并明确只展示总估值与成功同步
时间、不披露具体持仓。量化仓 `docs/requirements/team-binance-assets-v1.md` 是完整
采集、快照和页面需求；本文件只负责主服务公开转发边界。

新增精确 `GET /quant/api/v1/team-assets/summary`，只转发至既有固定 loopback 量化
服务的 `/api/v1/team-assets/summary`。不接受查询参数或请求体，不转发 Cookie、
Authorization、主用户身份或任意目标 URL，不扩大个人资产、Paper 和运营路由权限。

API 返回 `yilong.quant.team_assets.summary.v1`：币安现货钱包 BTC 原始估值、明确范围、
最近成功采集时间和时效状态。它不是全账户净资产、基金 NAV、收益或可提现金额。
量化服务未配置采集器时返回不可用，不以零资产表示未接通。

当前量化公共预览通过 HTTP 提供，公开汇总不带凭据；HTTP 传输不能提供端到端
内容真实性保证，不能包装成审计证明。真实资产同步必须在配置受保护只读凭据后
单独验收；私有币安请求必须 HTTPS，不能经本公开代理发送。

验收：精确路径和 GET 允许；查询/非 GET/私有路径拒绝；保留既有响应大小、超时、
无缓存及无凭据转发边界。量化源和页面分别发布后，再核对公开入口及 APK 展示。
