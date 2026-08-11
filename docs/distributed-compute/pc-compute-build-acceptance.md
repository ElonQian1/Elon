---
title: PC 算力工作区构建验收
status: current
reviewed_at: 2026-08-11
owners: pc-frontend, ai-economy
implementation_status: implementation_partially_verified
---

# PC 算力工作区构建验收

## 已验证范围

- `pc-frontend` 按锁文件完成干净依赖安装。
- 全前端 `npm run typecheck` 与 `npm run lint` 通过。
- `npm run build` 通过 TypeScript 检查并完成 Vite 生产构建；`/my-compute-settlement` 与 `/compute-supply` 均进入路由图并产出独立页面 chunk。
- `npm run check:bundle-budget` 通过；本次产物中 My Compute Settlement 页面约 20.42 kB（gzip 6.42 kB），Compute Supply 页面约 77.45 kB（gzip 19.27 kB）。

## 未验证边界

- 未启动真实服务端、TCP HTTP/MCP 客户端或登录会话，不能据此证明 API 联调成功。
- 未运行浏览器交互、视觉、窄屏、键盘或可访问性验收。
- 未发布 `pc-next-dist`，线上 `/pc` 没有因本次验收发生变化。
- 构建不证明 Provider/Pool 已验证或激活，也不证明 Offer、任务派发、用量、结算、外部付款或 Sui 链上资产真实可用。

## 验证命令

- `npm ci`
- `npm run typecheck`
- `npm run lint`
- `npm run build`
- `npm run check:bundle-budget`
