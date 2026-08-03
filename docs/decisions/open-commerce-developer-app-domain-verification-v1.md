---
title: 开放商业开发者 App 主页域名控制证明 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业开发者 App 主页域名控制证明 V1

## 背景

App 资料中的 HTTPS 主页只是开发者声明。平台审核前至少需要证明提交者能够控制该主页域名，避免审核完全建立在可随意填写的链接上，同时不能把域名控制证明夸大为工商主体或生产应用认证。

## 决定

1. App 所有者或项目管理员可为当前资料修订生成 24 小时 challenge。明文只返回一次，数据库仅保存规范内容的 SHA-256。
2. 开发者把规范内容发布到主页同源的 `/.well-known/yilong-open-commerce-app-verification.txt`，再由平台主动验证。
3. 验证请求仅使用 HTTPS 443，禁用重定向，连接超时 5 秒、总超时 10 秒，响应最多 4 KiB 且必须是 UTF-8 文本。
4. 出站主机必须由运营方加入 `OPEN_COMMERCE_APP_DOMAIN_ALLOWED_HOSTS` 精确白名单。白名单为空时验证失败关闭。
5. challenge 和验证结果绑定 App、项目及资料修订。资料再次编辑会清除 challenge 和旧证明，审核提交必须先通过当前修订的域名验证。
6. 域名验证只证明 challenge 验证期间能够控制主页域名，不证明组织身份、域名法律所有权、代码安全、交易资质或外部平台授权，也不签发生产凭据。

## 实现引用

- `server/src/open_commerce_developer_domain_service.rs`
- `server/src/store/open_commerce_developer_app_domains.rs`
- `server/src/open_commerce_developer_domain_migration.rs`
- `pc-frontend/src/features/open-commerce/DeveloperAppManifestPanel.tsx`
- `docs/open-commerce-developer-app-domain-verification-v1-acceptance.md`
