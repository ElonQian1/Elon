---
title: 原生模拟网格个人授权签发 V1
decision_status: accepted
version_status: current
reviewed_at: 2026-09-14
---

# 原生模拟网格个人授权签发 V1

承接量化商业 Goal G23 及量化仓库 `grid-native-access-v1`。主账号是身份真源；原生模拟网格不使用 Paper 参与份额、ESK、钱包或交易所权限。此次交付签发入口与消费者互操作，客户端同意页、长效可撤销连接及行情调度仍须后续完成，不能以签发接口代替完整产品。

## 合同

- `POST /api/me/quant/native-grid/access-grants` 仅接受真实用户 Bearer 会话；静态 owner/admin、虚拟用户、重复鉴权头、查询参数均拒绝。只在经服务实际 TLS 监听器确认的通道提供，不相信 Forwarded 头。响应与错误均 no-store，不回显凭据。
- 请求严格限定 schema `yilong.quant.native_grid_issue.v1`、environment `native_paper`、scopes、explicit_consent=true、confirmation=`授权使用原生模拟网格`。权限限 native_grid.read/create/control；非空、无重复、不推导权限。不签发 simulate，也不接受用户自报 owner、期限或 key。
- 签名严格实现量化 `yng1` 合同，覆盖 `yng1.` 加原始 base64url payload；不复用 Paper 的原始 JSON 签名格式。subject 使用独立 HMAC 域和秘密映射为 ngu_ 加40位小写十六进制，grant 为 ngg_ 加32位随机十六进制。
- 仅读取数据库中的真实 sessions/users，在同一读取事务内取得身份、状态、撤销与期限，并在取得事务后读取时钟。签发期限取300秒与原会话剩余期限的较小值。会话无效、损坏或期限不明时失败关闭；不更新登录期限、不落盘 token。
- 三项独立环境配置：`YILONG_QUANT_NATIVE_GRID_SIGNING_KEY_ID`、`YILONG_QUANT_NATIVE_GRID_SIGNING_SEED_BASE64URL`、`YILONG_QUANT_NATIVE_GRID_SUBJECT_SECRET_BASE64URL`。全部缺失为关闭；部分、空白、非规范或长度错误为配置无效。Paper 配置不能启用原生签发。subject secret 应持久保管，换签名 key 不改变同一 subject。
- 本合同为短期签发，不是长效授权或 refresh token。量化服务已有单 grant 撤销；主会话后续退出会阻止再次签发，但已经签发的离线令牌可能继续有效至其原到期时间（最多300秒）。不得宣称已经实现即时跨服务撤销；长效授权接线前须落实同步撤销及恢复合同。
- GET 同一路径返回认证后的签发能力，不返回密钥；关闭或错误不得显示为已具备可用原生网格。

## 文件与验证

- `server/src/quant_native_grid_access/{model,signer,issue,mod}.rs`：各约100–220行，独立单一责任。
- router 模块声明及真实 TLS 私有路由挂载各少量接线；不改全局登录或旧 Paper 协议。
- 定向 harness 直接编译生产签名、校验及 SQL 源码，使用真实 SQLite 和固定合成签名样例；量化验证器消费同一公开样例。再验证完整服务编译、真实 HTTP/TLS 路由及正式发布，分项记证据。
- 覆盖签名格式、过期/错用途/重复 scope/超宽权限、静态或撤销会话、停用用户、数据库故障、限于父会话期限、两用户隔离和换 key 身份稳定；样例只包含明确合成账号和测试密钥。

正式密钥配置、客户端授权、公开原生路由及手机实测未完成前，功能状态保持部分完成。
