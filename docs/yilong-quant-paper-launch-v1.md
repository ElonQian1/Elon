# 一龙量化 Paper 一键进入 V1

状态：代码接入中，默认关闭；未配置公开量化 Web 地址和双方签名/来源参数前，不得描述为已部署。

## 用户体验

用户加入“一龙量化交易”后，在项目主页看到独立的 `PAPER / SIMULATED` 卡片。主页先检查配置；只有签名材料和量化 Web 地址都有效时才允许点击“安全进入”。点击后：

1. 主项目用当前登录会话签发最多五分钟的量化专用 grant。
2. 主项目加载量化 PWA，URL 不包含 grant、participant、主项目 bearer 或账号资料。
3. 量化页面生成随机 nonce，通过 exact-origin `postMessage` 请求授权。
4. 主项目同时校验量化 origin 和当前 iframe window，只发送一次 grant 并立即清空主页面引用。
5. 量化页面只在 React 内存中使用 grant 查询本人模拟仓位与 ESK 模拟 binding，刷新或关闭后必须重新签发。
6. 用户明确选择一笔 `submitted` ESK 申请时，主项目才附带同次 grant 的 `yeqa1` 单申请授权；量化端 accepted/released 后返回独立签名的 `yqar1` 回执，主项目验签成功后才推进申请状态。

项目广场的通用加入/打开逻辑不变；只有官方项目 ID `yilong-quant` 和净化后的 `yilong.quant.paper_launch.v1` 字段同时存在时才显示该入口。

## 接口与契约

- `GET /api/me/quant/paper-launch`：已登录用户读取 readiness，不返回密钥或 grant。
- `POST /api/me/quant/paper-launches`：签发启动票据；返回严格量化 URL、短期 grant、有效期和 `simulated=true`。
- `POST /api/me/quant/paper-access-grants`：保留为开发/合同测试的显式 scope 签发端点，不用于一键页面传参。
- 双方合同：`contracts/quant/paper-launch-v1.schema.json`。

所有未知 schema、协议、来源、窗口、nonce、attempt、过期时间和额外字段都失败关闭。iframe 只允许 `allow-scripts allow-same-origin`，不授予顶层跳转、弹窗、下载、支付、摄像头、麦克风或地理位置。

## 主项目配置

| 环境变量 | 用途 | 规则 |
|---|---|---|
| `YILONG_QUANT_PAPER_WEB_URL` | 量化 PWA 页面 | HTTPS；仅 loopback 开发可 HTTP；禁止 userinfo、query、fragment |
| `YILONG_QUANT_PAPER_SIGNING_KEY_ID` | 当前 Ed25519 key ID | 3–64 字符稳定标识 |
| `YILONG_QUANT_PAPER_SIGNING_SEED_BASE64URL` | 32 字节 Ed25519 seed | 只在主项目密钥域，base64url 无 padding |
| `YILONG_QUANT_PAPER_SUBJECT_SECRET_BASE64URL` | 32 字节 participant 脱敏 HMAC secret | 只在主项目密钥域；不得进入量化项目 |
| `YILONG_QUANT_ESK_RECEIPT_KEYRING_JSON` | 量化 ESK 回执验签公钥环 | `contracts/quant/esk-allocation-receipt-keyring-v1.schema.json`；只含量化端公钥，支持 active/retiring/revoked |

任一项缺失或非法时 readiness 必须显示 `configuration_required` 或 `configuration_invalid`，签发返回 503；不得回退到 URL token、长期 token 或共享 Cookie。

## 量化项目配置

| 环境变量 | 用途 | 规则 |
|---|---|---|
| `QUANT_PAPER_PARENT_ORIGIN` | 可信主项目来源 | HTTPS exact origin；仅 loopback 开发可 HTTP；无路径、query、fragment |
| `QUANT_PAPER_ACCESS_KEYRING_JSON` | 固定 Paper 验签 keyring | 推荐；遵循 `contracts/quant/paper-access-keyring-v1.schema.json`，只含公钥 |
| `QUANT_PAPER_ACCESS_KEY_ID` + `QUANT_PAPER_ACCESS_PUBLIC_KEY_BASE64URL` | 旧单 key 验签配置 | 继续兼容；两项成对存在且不得与 keyring 混用 |
| `QUANT_ESK_ALLOCATION_RECEIPT_SIGNING_KEY_ID` + `QUANT_ESK_ALLOCATION_RECEIPT_SIGNING_SEED_BASE64URL` | ESK accepted/released 回执签名 | 两项成对存在；独立 Ed25519 域，不得复用主项目授权 key、subject secret 或运营令牌 |
| `QUANT_WEB_ORIGIN` | 跨域调用 API 的量化 Web 来源 | 仅确需跨域时配置；生产优先同源反代 |
| `VITE_QUANT_API_URL` | PWA API 基址 | 生产优先留空并由同源 `/api` 反代 |

量化静态站点必须把 CSP `frame-ancestors` 限制到主项目 exact origin；主项目 CSP 必须把 `frame-src` 限制到量化 origin。TLS、HSTS、`X-Content-Type-Options: nosniff` 和最小第三方脚本是公开部署前门禁。

## 密钥生成、保存和轮换

- Ed25519 seed、subject secret 必须由批准的密码学随机源在受控环境生成，并直接写入秘密管理器；不要粘贴到聊天、Issue、命令行历史、日志或 Git。
- 只把 seed 派生出的公钥和 key ID交给量化部署；seed 永不离开主项目密钥域。
- subject secret 决定稳定 `yp1_` participant。没有 participant 迁移表时不得旋转，否则同一用户会映射到新模拟仓位。
- 正常轮换先在量化 keyring 加入新 `active` 公钥，把旧 key 标记为 `retiring` 并将 `issued_at_not_after_unix` 固定为主项目切换时刻；量化预检 ready 后，主项目再切换当前 key ID 与 seed。等待最长五分钟后移除旧 key。
- 回执 key 反向轮换：先在主项目回执 keyring 加入量化端新 `active` 公钥，再切换量化 signer；保留旧公钥验证历史 accepted/released 回执，确认恢复路径后才把旧 key 标记 retiring/revoked。双方签名域不得合并。
- 发现签名 key 泄露时，先在量化 keyring 把该 key 标记 `revoked` 并重启验证，再关闭或更换主项目当前签名 key。全量撤销会让量化一键入口失败关闭。当前是密钥级撤销，不保存单个 grant 吊销表。
- 回滚时先取消 `YILONG_QUANT_PAPER_WEB_URL` 或 `QUANT_PAPER_PARENT_ORIGIN`，使入口失败关闭；不删除账本、不改 participant secret。

## 本机 Paper 验收

1. 主项目 URL 指向批准的 loopback 量化 PWA；量化 parent origin 指向主项目 loopback origin。
2. 双方 key ID/公钥匹配，主项目 seed 与 subject secret 只进入当前测试进程环境。
3. 登录主项目，进入官方“一龙量化交易”项目主页，确认 readiness 为可进入。
4. 点击后验证 URL、浏览历史、DOM、浏览器 storage、日志和剪贴板均无 `ypg1.`。
5. 验证 active、redemption_pending、redeemed、not_found、过期、错误 origin、错误 window、错误 nonce 和量化页面超时。
6. 选择一笔 ESK 申请，验证 submitted → accepted → released、回执精确重放、同步失败后重新进入恢复，以及 accepted/released 占用变化；确认全过程没有量化份额、NAV、交易或资金移动。
7. 清除任一关键配置，确认对应能力不可用、无错误状态推进；普通只读入口与 ESK binding capability 分别失败关闭。

本验收只证明 Paper 身份与模拟仓位链路，不证明付款、钱包、KYC、地区准入、真实 NET、收益、托管或 live 交易。
