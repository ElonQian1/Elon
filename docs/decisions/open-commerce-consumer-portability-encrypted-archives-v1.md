---
title: 开放商业消费者可携带数据离线加密归档 V1
status: accepted
date: 2026-08-03
owners: sdk, pc, security
---

# 开放商业消费者可携带数据离线加密归档 V1

## 背景

消费者数据包可以下载并迁移，但普通 JSON 落盘后可被本机其他程序、同步盘或误分享直接读取。平台不应通过托管用户口令或私钥重新集中控制这些数据，因此加密和解密应发生在消费者设备本地。

## 决定

1. PC 和连接器 SDK 共同实现 `open_commerce.consumer_portability_encrypted_archive.v1`，可包装普通导出包或带运营方签名的导出包。
2. 口令限定 12–256 个字符，只在调用方内存中使用，不进入服务端 API、数据库、日志或审计。
3. 每个归档生成独立 16 字节随机盐和 12 字节随机 Nonce，使用 PBKDF2-SHA256 310000 次派生 256 位密钥，再以 AES-256-GCM 和 128 位认证标签加密。
4. 固定协议版本作为 AES-GCM Additional Authenticated Data，避免密文被静默解释为其他协议。归档同时保存明文 SHA-256，解密后再次核对。
5. 浏览器导入先在本机解密，再把原始结构发送给既有导入 API。服务端不接收口令，也不存储加密归档副本。
6. Node SDK 和 PC 使用相同字段、参数、Base64 表示和认证数据，便于运营方工具与消费者客户端互操作。

## 边界

- 当前没有口令找回、云端密钥托管、硬件密钥、多人恢复或社会恢复。
- 弱口令仍可能被离线猜测；PBKDF2 参数不是对所有未来硬件永久安全。
- 解密后的数据仍遵守隔离导入、签名信任和迁移采用边界，不自动恢复任何业务状态。
- 当前批次未进行跨端互操作、性能、内存峰值或大文件验证。

## 实现入口

- `sdk/open-commerce-connector/src/portability-archive.js`
- `pc-frontend/src/features/open-commerce/portabilityArchive.ts`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityExports.tsx`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityImports.tsx`
