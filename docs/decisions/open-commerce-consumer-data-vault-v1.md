---
title: 开放商业消费者客户端加密数据保险箱 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-10
implementation_status: verified_local_crypto_http_migration_pc_build
---

# 开放商业消费者客户端加密数据保险箱 V1

## 背景

低敏偏好档案只允许固定结构化字段，不能承载身份、健康、财务或自由文本等敏感资料。若平台直接保存明文，用户仍需把数据控制权交给中心服务；若只支持本地文件，又缺少多设备可取得的耐久密文、修订控制和删除入口。因此 V1 提供由消费者设备加密、平台只托管不透明密文的保险箱。

## 决定

1. 明文在 PC 客户端使用 Web Crypto 加密，口令只停留在当前浏览器内存，不进入 API、数据库、审计或日志。
2. 固定使用 PBKDF2-SHA256、310000 次迭代、16 字节随机盐和 AES-256-GCM、12 字节随机数、128 位认证标签。
3. 每个密文信封通过附加认证数据绑定信封版本、记录 ID 和修订号，解密后的明文也必须包含匹配的版本、记录 ID 和修订号，防止跨记录或跨修订替换。
4. 服务端只保存用户提供的记录 ID、非敏感标签、类型、密文信封、密文 SHA-256、大小、修订和时间。列表不返回密文，只有本人按项目读取单条时返回完整信封。
5. 创建固定为修订 1；更新必须携带当前预期修订并上传绑定下一修订的新信封，冲突时失败关闭。删除需要用户确认和当前修订。
6. 每名用户在每个消费者项目最多保存 100 项，单项密文为 17 字节到 1 MiB；服务端严格校验算法参数、Base64、时间、ID 和修订，但不尝试解密。
7. 创建、更新和删除审计只记录类型、修订、密文摘要、大小以及 `server_can_decrypt=false`，不记录标签、口令或明文。
8. 记录 ID 只在同一消费者项目与同一用户范围内唯一。V220 把 V162 的全局 ID 主键升级为 `(consumer_project_id, consumer_user_id, id)`，允许隔离用户或项目安全复用可携带记录 ID。

## 可见元数据

- 标签、类型、记录 ID、密文大小、密文摘要、修订和时间对平台可见。
- 用户不得把秘密写进标签；真正敏感内容必须放在加密正文中。
- 密文摘要用于完整性与审计，不证明明文内容真实。

## 边界

- 平台不持有解密密钥，不提供密码找回、密钥托管、设备恢复或紧急解锁；口令丢失即无法解密。
- V1 不向商户、开发者 App 或 AI 自动披露保险箱内容，也没有字段级授权解密和远端代理执行。
- V1 不是端到端多设备密钥同步、硬件安全模块、跨运营方身份、完整订单迁移或外部数据删除系统。
- 浏览器实际密码学模块已通过 Node 24 Web Crypto 往返、错误口令、密文篡改、跨记录/跨修订替换、严格 Base64、时间和资源上限测试；PC TypeScript、开放商业回归和 Vite 生产构建均通过。
- 服务端专项已验证 V162 到 V220 的合成磁盘升级与重开、完整 Store 新建迁移、HTTP 鉴权和项目访问、同项目用户隔离、跨用户/项目同 ID 共存、并发修订竞争、删除确认、100 项和 1 MiB 上限以及审计脱敏。状态为 `verified_local_crypto_http_migration_pc_build`。
- 真实 Chromium 解锁、编辑和密文下载、生产历史库副本迁移、日志脱敏观察和生产部署仍未验证。

## 实现入口

- `server/src/open_commerce_consumer_vault_model.rs`
- `server/src/open_commerce_consumer_vault_service.rs`
- `server/src/store/open_commerce_consumer_vault.rs`
- `server/src/open_commerce_consumer_vault_api.rs`
- `server/src/open_commerce_consumer_vault_migration.rs`
- `pc-frontend/src/features/open-commerce/consumerDataVaultCrypto.js`
- `pc-frontend/src/features/open-commerce/consumerDataVaultCrypto.d.ts`
- `pc-frontend/src/features/open-commerce/ConsumerDataVaultPanel.tsx`
- `server/src/open_commerce_consumer_vault_service_tests.rs`
- `server/src/open_commerce_consumer_vault_api_tests.rs`
- `server/src/open_commerce_consumer_vault_api_test_support.rs`
- `server/src/open_commerce_consumer_vault_migration_tests.rs`
- `scripts/test-open-commerce-consumer-data-vault-crypto.mjs`
- `docs/open-commerce-consumer-data-vault-v1-acceptance.md`
