---
title: "本人网格只读投影传输 V1"
version_status: current
status: accepted
implementation_status: in_progress
reviewed_at: 2026-09-07
owners: [private-read-bridge]
---

# 用户目标

用户已同意先将 Win 已登录币安的本人网格列表和详情送到量化 APK。
本仓承担通用私密投影传输、主账号隔离和明确只读授权；币安解析与手机网格展示由独立量化仓负责。

## 合同与边界

- 本机节点仅为当前已绑定主用户接收有界只读投影，保存持久待同步记录，再通过显式配置的可信 HTTPS 上送；不能经旧明文 WebSocket 发送私人正文。
- 云端仅依真实节点凭据、节点状态及当前所属用户决定 owner，不相信正文自报用户。绑定变更令旧本机 binding 失效，消费者不自动重绑旧资料。
- Envelope 为 yilong.private_read_projection.v1：source、connection_id、generation、revision、observed_at_ms、fresh_until_ms、status、payload。第一版唯一 source 为 binance-futures-grid，payload schema 为 yilong.quant.binance_grid_snapshot.v1。
- revision 是除自身外全部字段的递归有序对象紧凑 JSON SHA-256；存储保持同修订幂等、不同正文冲突拒绝及代次/观察时间单调，不把收到时间伪装为新观察。
- 列表响应为 yilong.private_read_projections.v1 的 snapshots 数组；不任意跨节点选择来源，数量和正文上限明确，过期/撤销/失联有状态。
- 新增精确 scope grid.snapshot.read，复用已有 PKCE、一次性码、短期 token 和撤销复核；旧 ESK 或 Paper grant 不增加任何权限，网格确认文案独立。
- 主 APK 原生同意页核验官方量化 APK 身份和新的网格 purpose；只返回一次性码，量化客户端凭据留在原生内存，不注入公共 HTTP WebView。
- 私人投影不进入 /quant 公共代理、公开团队资产、ESK/QSHARE 账本或模型日志；本批无交易写操作、签名或资金移动。
- 研究原一小时期限维持；来源过期时停止新同步，必须以新有效研究会话获取新的源证据。

## 编辑计划与验收

- server/private_read_projection*：独立协议、持久存储、认证接收与读取，各文件目标低于500行。
- server/node_agent_private_read*：本机 binding、队列、HTTPS 发送与回执；入口只添加组装。
- 既有 asset access scope/validation、TLS 路由与必要迁移：旧授权行为兼容，不扩大已有授权。
- android/grid/access：独立同意请求与主账号原生确认；量化侧列表和详情不放在主仓。
- 验证认证主体、错节点/账号、来源绑定、过期/撤销、重复/乱序、凭据字段、正文大小和 HTTPS 伪造头；构建与真实部署分别留证。
- 必须完成真实 Win 到私密接口、再到量化 APK 的验收才能宣称用户功能已接通；公网或手机尚未可用时保持进行中。
