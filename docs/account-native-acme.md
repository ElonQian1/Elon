# Rust 原生 IP 证书管理

状态：代码实现与本地专项测试通过，生产签发与切换待验收。模块 `server/src/account_security/https/acme/`，与商业边缘网关的域名证书配置独立。

服务器通过 `instant-acme 0.8.5`、rustls 和 rcgen 实现 ACME，不启动 Certbot、lego、Nginx 或其他证书进程。Let’s Encrypt 的 `shortlived` IP 证书有效期160小时；当前实现每5分钟检查，剩余48小时内续期。失败按15分钟起指数退避，最大间隔4小时；请求前保存重试时间，防止重启循环耗尽签发额度。单次申请最多10分钟。

## 配置与端口

```dotenv
ACCOUNT_HTTPS_ENABLED=true
ACCOUNT_HTTPS_LISTEN_ADDR=0.0.0.0:8443
ACCOUNT_ACME_ENABLED=true
ACCOUNT_ACME_ACCEPT_TOS=true
ACCOUNT_ACME_IP=43.139.149.158
ACCOUNT_ACME_LISTEN_ADDR=0.0.0.0:443
ACCOUNT_ACME_DATA_DIR=/var/lib/elon-account-native-acme
ACCOUNT_ACME_STAGING=false
```

443仅在TLS-ALPN-01验证期间短暂监听，无HTTP业务路由；证书机构必须能从公网访问该端口。业务HTTPS继续使用8443，原HTTP/WebSocket8080保持兼容。首次安装可以不配置旧证书路径，Rust在启动HTTPS前完成首次申请。迁移时保留既有 `ACCOUNT_HTTPS_CERTIFICATE_PATH` 和 `ACCOUNT_HTTPS_PRIVATE_KEY_PATH`，后台成功后再热切换。

证书申请需要明确接受证书机构服务条款。个人也可使用；不需要企业营业执照。HTTPS证书与网站备案是不同事项。

## 存储与恢复

`DATA_DIR/{staging|production}/{ip}/` 保存账号凭证 `account.json`、完整证书与私钥 `active.pem`、运行状态 `status.json`。Linux目录0700、文件0600；不要提交、共享或输出前两者。一个PEM同时保存证书链与私钥，写入临时文件、fsync后原子替换；读者读取同一份字节快照，避免续期时证书与私钥不匹配。

启用前校验证书链、IP SAN、有效期及公私钥匹配。生产模式额外要求公共根信任；测试环境证书永远不会加载到业务HTTPS。TLS监听器60秒检查新证书，校验失败保留上一份可用TLS配置。旧证书路径不被覆盖；设置 `ACCOUNT_ACME_ENABLED=false` 并重启可恢复旧证书来源，但必须同时恢复旧续期任务。

## 运维迁移顺序

1. 部署包含原生ACME模块的服务端，保持现有HTTPS可用。
2. 使用 `scripts/configure-account-native-acme.sh staging`；验证CA可访问443，确认测试签发成功，业务仍返回旧可信证书。
3. 使用同一脚本 `production`；确认生产签发成功、外部HTTPS证书指纹与新证书一致后，脚本才关闭旧续期定时器。保留旧工具和证书供回退。
4. 使用 `status` 读取脱敏运行状态；续期失败时保留有效证书并输出固定错误标识，不打印CA响应或账号凭证。
5. 需要回退时使用 `disable`，恢复旧证书配置来源及旧续期定时器。

脚本只配置既有服务器服务并核查状态，不安装额外软件。申请、续期和热加载全部由Rust完成。首次签发、后续定时续期和重启后恢复应分别验收，不能把单次成功当作未来续期已经通过。

依据：[Let’s Encrypt IP证书公告](https://letsencrypt.org/2026/01/15/6day-and-ip-general-availability)、[instant-acme API](https://docs.rs/instant-acme/0.8.5/instant_acme/)。
