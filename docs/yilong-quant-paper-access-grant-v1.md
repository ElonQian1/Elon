# 一龙量化 Paper 用户授权 V1

## 结论

一龙主项目现在具备一个默认关闭的 paper 身份桥接：已登录用户可以通过 `POST /api/me/quant/paper-access-grants` 请求最多五分钟有效的 Ed25519 签名授权，量化子项目据此读取该脱敏主体自己的模拟持仓或申请模拟全额退出。

这不是 KYC、地区准入、真实 NET 锁定、基金申购或钱包证明。主项目已提供受控的一键 Paper 启动协议，但签发配置、量化验证公钥或量化 Web 部署未配置时，用户入口仍保持关闭。

## 请求

请求继续使用一龙现有 `Authorization: Bearer <main-session>`，正文必须明确列出一个或两个 scope：

- `paper.position.read`
- `paper.redemption.request`

主项目不会把原始 bearer 会话交给量化项目。重复 scope、未知 scope、未登录或非 active 账号均失败关闭。

## 签名证明

令牌格式为 `ypg1.<payload-base64url>.<ed25519-signature-base64url>`。Payload 由 `contracts/quant/paper-access-grant-v1.schema.json` 定义，固定绑定：

- `issuer=yilong-main`
- `audience=yilong-quant`
- 项目专用 `yp1_` 脱敏 participant
- 明确 scope 和 `paper-participation-risk-v1`
- 最多 300 秒有效期
- `simulated=true`

量化项目必须固定信任配置中的 key ID 和 Ed25519 公钥，不得信任令牌自带或网络临时提供的任意公钥。

## 配置

主项目只有同时配置以下三项时才签发：

- `YILONG_QUANT_PAPER_SIGNING_KEY_ID`
- `YILONG_QUANT_PAPER_SIGNING_SEED_BASE64URL`
- `YILONG_QUANT_PAPER_SUBJECT_SECRET_BASE64URL`

签名 seed 与 subject 脱敏密钥必须各自独立、均为 32 字节，不能提交 Git、打印日志或发给浏览器。量化项目只配置对应公钥与 key ID，不获得主项目私钥或 subject 密钥。

## 当前限制

- 没有多 key 重叠轮换控制面、吊销列表或公开部署验收；当前跨站自动接入仅限五分钟 Paper grant，失败时不会降级到 URL 传参。
- `participant_ref` 只证明同一主项目账号的稳定脱敏映射，不证明付款、钱包、KYC 或投资资格。
- 授权令牌属于短期 bearer，不得进入 URL、日志、数据库、localStorage 或 sessionStorage。
- 当前只能作用于子项目的 paper 账本；真实资产相关操作必须使用独立生产协议和审批。
