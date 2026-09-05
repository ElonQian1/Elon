---
title: "统一账号与只读资产接口 V1 接入合同"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-account-access, quant-client]
---

# 统一账号与只读资产接口 V1 接入合同

需求见[正式需求](../requirements/unified-account-asset-access-v1.md)。本文件描述接入与迁移，
不证明已经部署。机器字段以 [JSON Schema](../../contracts/assets/asset-access-v1.schema.json)
和合成向量为准，客户端细节见 [SDK](../../sdk/asset-access/README.md)。

## 身份、权限与资产

主项目 users/sessions 是唯一主身份来源，正式 ESK 账本是唯一资产来源。每个客户端独立
授权，不复制余额，不通过邮箱自动创建或合并账号。subject 在同一用户与客户端组合中稳定；
跨客户端不暴露同一个内部 user_id。服务器在一个读取事务内验证授权和主会话，并扫描完整
正式账本后输出额度与进度。客户端只负责展示、分页和期限管理，不重新计算资产业务规则。

固定客户端为 quant.android、quant.web、quant.ai。它们共享协议，但不共享 bearer。
client_id 是公开标签，持有它不证明软件签名或读取资格。Android 官方调用者由主 APK
校验系统提供的包名、组件和签名；资源访问仍需正确 PKCE 兑换出的受限 token。

| 范围 | 内容 |
|---|---|
| esk.summary.read | 必需；正式总额、占用与可申请额 |
| esk.progress.read | 可选；最多 20 条申请进度与快照游标 |
| profile.read | 可选；当前授权用户的有界昵称 |

该授权不能登记、提交或取消卖回、交易、转账或执行管理员操作。受限 token 不被原登录
认证接受。金额为非负 i64 范围的十进制整数字符串，精度固定六位；来源仍是
platform_recorded、链状态 not_deployed、funds_moved=false。

## 接口

所有接口都仅接受独立的真实 TLS 入口，响应和错误均 no-store。

| 方法与路径 | 调用身份 | 结果 |
|---|---|---|
| POST /api/me/asset-access/authorize | 主会话与明确同意 | 120 秒内一次性授权码 |
| POST /api/asset-access/token | 授权码、PKCE verifier 与原绑定 | 独立只读 token |
| GET /api/asset-access/me | 受限 token | 应用内身份、授权范围与期限 |
| GET /api/asset-access/esk | 受限 token | 同一来源的正式资产投影 |
| POST /api/asset-access/revoke | 受限 token | 撤销该应用授权 |
| GET /api/me/asset-access/grants | 主会话 | 最近 100 个授权及状态 |
| POST /api/me/asset-access/grants/:id/revoke | 主会话 | 撤销本人指定授权 |

受限资源与自撤销都要求 X-Elon-Asset-Client。身份不得从查询参数取得。GET limit 为
1–20，cursor 最长 160 字符；include_progress=false 时不能带 cursor。所有未知请求
字段或 schema 均拒绝。快照变更为 409 asset_access_snapshot_changed，客户端清旧页并
从第一页重新读一次；不能把不同快照累加。401/403、超时或协议错误都应清掉旧资产展示。

授权最长 3600 秒，主 APK 原生入口固定 900 秒。没有 refresh token。原主会话剩余期限
更短时使用更早期限；以后被缩短时客户端截止时间只能缩短，不能增加。兑换必须同时匹配
client、redirect、state 和 S256 verifier，并原子消费授权码。存储中仅有主会话、码和
token 的哈希，读取不更新 last_seen_at 或续期。

## Android 接入

两端使用独立构建变量 ELON_ASSET_ACCESS_ORIGIN，默认空；必须为无用户信息、查询和
路径的 HTTPS origin。它不回退到公开 HTTP Paper 页面或旧主服务地址。

- 量化调用组件：com.elon.quant.assets.access.AssetAccessActivity。
- 主同意组件：com.elon.app.esk.platform.access.AssetAccessConsentActivity。
- 输入 extra 为 asset_access_request，其 JSON 仅含 schema=
  yilong.asset_access.android_request.v1、state 和 code_challenge。
- 主界面自行确定 quant.android、summary/progress 两个 scope、900 秒与回调
  com.elon.quant:/asset-access/callback，不接受调用方扩大权限。
- 输出 extra 为 asset_access_approval，内容是 authorization_code.v1 响应；主 token
  不返回。量化保留内存中的 verifier，自行 HTTPS 兑换和连续读取。
- 主同意页失焦、切号、重建或超时即取消；量化进程重建后不恢复私有授权。

这是新的正式合同，旧逐页 progress 合同继续保持原行为。登记新组件不代表已经替换旧入口；
入口切换与双方 APK 发布必须协调，并核验官方签名、组件存在和部署版本。

## Web 与 AI 接入

Web 回调固定为配置的 HTTPS public_url 加 /quant/asset-access/callback。SDK 实例仅在
内存中持有 verifier，浏览器必须通过可信 popup 回调保持原实例，或在可信 BFF 内持有该
实例；回调需要精确 origin、source-window 与 state 检查。SDK 不包含主账号登录页或同意页。

AI 回调仅允许 http://127.0.0.1:端口/asset-access/callback，端口至少 1024。这个例外
仅针对本机接收回调，授权 API 和资产请求仍必须 HTTPS。运行时持有凭据，只把经校验的
身份/资产结果作为工具输出；不得把主 token、受限 token、兑换码和 verifier 放入模型文本。
Web 页面、AI 宿主的实际登录同意与回调接入属于各自接入验收，SDK 通过不等于这些入口可用。

## TLS 与部署

服务端复用现有 node_endpoint_transport 的 rustls 监听器。只有真实 TLS 握手之后，
secure_router 才为新路由注入不可由 HTTP 字段构造的传输标记；不消费节点单次认证证据。
普通 HTTP 返回 426，伪造 Forwarded/X-Forwarded-Proto 无效。客户端必须在发送凭据之前
拒绝 HTTP，不能依赖服务器收到凭据后的拒绝来保护网络传输。

监听器证书、私钥路径和监听地址通过现有 NODE_ENDPOINT_DIRECT_TLS_* 配置注入。
本次不自动开启节点凭据、节点会话或 owner bootstrap 开关。PUBLIC_URL 用来绑定 Web
回调和浏览器 Origin，它不是传输加密证明。普通反向代理若在 HTTP 入口终止 TLS，不能
仅靠转发头调用新 API；需要连接到真实 TLS 入口或另立受信代理合同。

本批不会申请域名、生成生产证书或修改线上 TLS 配置。正式启用前需核验批准的域名、
证书链、服务版本与两端编译地址，并完成本人授权、连续读取和撤销验收。

## 撤销与留存限制

服务器上的主会话撤销、到期、换属或用户停用都会让后续读取失败。现有主 APK 的普通
退出入口只清理本地登录数据，尚不保证撤销服务器会话。因此本批不承诺“本机退出即让所有
远端授权失效”；用户应使用明确的授权撤销操作。SDK clear 也只清理本机，只有成功的
revoke 响应证明服务器撤销已完成。网络失败时不得显示“已撤销”。

迁移 V289 仅新增身份映射、授权码、授权、凭据摘要与审计记录，不修改旧身份或余额。
授权历史追加保留并通过外键引用 users，因此未来物理注销需要单独定义匿名化或留存流程。
当前项目注销执行尚未开放，本迁移不能被当作注销执行能力。

回退新客户端时保留旧入口；停用新 TLS 入口会停止新读取，但不能远程收回已合法返回的数据。
