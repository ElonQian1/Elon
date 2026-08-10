---
title: 开放商业消费者可携带数据离线归档 V2
status: accepted
date: 2026-08-10
owners: sdk, pc-frontend, product
---

# 开放商业消费者可携带数据离线归档 V2

## 背景

V1 已使用 PBKDF2-SHA256 和 AES-256-GCM 保护归档明文，但 AES-GCM 的附加认证数据只有固定版本字符串。归档中的创建时间、明文摘要和算法参数没有作为一个完整、固定顺序的元数据消息接受认证；Node 对 Base64 的默认解析也会宽松接受部分非规范输入。PC 与连接器 SDK 分别维护实现，缺少可重复的双向互操作验证。

## 决定

1. 新归档使用 `open_commerce.consumer_portability_encrypted_archive.v2`；V1 停止写入但保留只读解密。
2. 密钥继续由 12 至 256 字符的本地口令、16 字节随机盐、PBKDF2-SHA256 310000 次派生为 256 位 AES 密钥；口令 UTF-8 表示不得超过 1024 字节。
3. 每次归档使用独立 12 字节随机 Nonce 和 AES-256-GCM 128 位认证标签，不复用盐或 Nonce。
4. V2 AAD 按固定换行顺序绑定协议名、归档 Schema、KDF 名称、哈希、迭代数、盐、加密算法、Nonce、标签位数、明文 SHA-256 和创建时间。外部实现不得依赖 JSON 属性顺序重建 AAD。
5. Base64 必须是标准、带规范填充且往返一致的表示；盐、Nonce、密文和签名在解码前先检查编码长度，解码后再检查字节边界。
6. 归档明文最多 6 MiB；解密端在执行 PBKDF2 前拒绝未知算法、异常迭代数、错误长度、非规范 Base64、无效时间和越界密文，避免由不可信归档放大 CPU 或内存消耗。
7. PC 与连接器 SDK 必须通过双向互操作测试；任何一端改变 Schema、AAD 或参数时，必须同步修改另一端和交叉测试。

## V2 AAD 消息

```text
open_commerce.consumer_portability_archive_aad.v2
<archive schema>
PBKDF2
SHA-256
310000
<salt_base64>
AES-256-GCM
<nonce_base64>
128
<plaintext_sha256>
<created_at>
```

## 边界

- 归档加密只保护离线文件，不替代来源运营方签名、消费者授权、平台导入校验或设备安全。
- 当前没有口令找回、云端密钥托管、多设备密钥同步、硬件密钥、密钥轮换服务或恢复代理。
- V1 兼容解密不追溯增加其元数据认证能力；需要完整 V2 认证时必须重新加密为 V2。
- 归档成功解密不证明订单、支付、履约、删除动作或商户声明是真实世界事实。

## 实现入口

- `sdk/open-commerce-connector/src/portability-archive.js`
- `pc-frontend/src/features/open-commerce/portabilityArchive.js`
- `scripts/test-open-commerce-portability-crypto-interop.mjs`
- `docs/open-commerce-consumer-portability-crypto-v2-acceptance.md`
