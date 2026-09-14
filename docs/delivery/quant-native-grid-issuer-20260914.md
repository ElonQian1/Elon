---
version_status: current
reviewed_at: 2026-09-14
implementation_status: implemented
acceptance_status: partial
---

# 原生模拟网格授权签发

主项目新增个人会话签发入口 `/api/me/quant/native-grid/access-grants`，仅挂载到实际 TLS 私有路由。真实 users/sessions 是账号和期限的真源；静态 owner/admin、过期、撤销、停用、虚拟账号、重复鉴权头、查询参数和 simulate 范围均拒绝。

签名覆盖 `yng1.` 与原始 base64url payload，与旧 Paper 原始 JSON 签名分离。subject 使用独立 HMAC 域，签名密钥轮换不改变同一 subject。事务内读完身份后才取得签发时钟，期限最多300秒并受父会话剩余期限限制，不保存 token、不延长登录、不改资金和权限数据库。

## 验证

- 直接编译生产源码的独立 harness：11项真实 SQLite/签名/输入测试通过。
- 完整 elon-server 测试构建与4项实际 Store/会话/Router 测试通过，包括静态凭据、TLS证明、origin、no-store、越权/超长/重复输入、不同用户以及撤销后拒绝。它是进程内真实路由，不是生产公网 TLS 实测。
- 首次完整测试的业务断言通过，但 Windows 临时库清理失败；已释放测试 Router 的额外引用后重跑全部4项通过。未弱化清理断言，也未回避失败。
- 量化 PR192（合入 `1fe8548aa77ccd5b05e08e5b5fdcc82f7cde628e`）消费同一合成样例，生产签发器逐字节重建样例；量化21项授权、全仓285项Rust/70项前端/14项交付合同通过。GitHub任务因付款/额度未启动，本地通过不冒充远端通过。

## 交付边界

正式服务发布及真实 TLS 路由待核验；生产密钥未配置，不能声称个人原生网格已经上线。客户端同意页、长效撤销/恢复、实时调度与量化原生策略 UI 仍未完成。签发期限不是要求用户每5分钟操作授权；长期连接需要后续独立合同。

本批未更新 APK、未操作用户手机或网页登录、未创建/修改/结束交易所网格。量化钱包同意已经完成的历史证据保留，当前手机离线，新的点击无响应问题仍待设备复核。
