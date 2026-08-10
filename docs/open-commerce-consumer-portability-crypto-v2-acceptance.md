# 开放商业消费者可携带数据密码学 V2 验收

## 已验证闭环

- 连接器 SDK 的运营方签名主路径已真实生成并验证 RSA-SHA256 签名，修复了公钥 `KeyObject` 被重复转换而导致签名函数退出的问题。
- SDK 只接受 2048 至 8192 位 RSA 密钥，与服务端信任公钥策略一致；签名绑定来源运营方、SPKI 公钥摘要、导出版本、包 ID、来源项目、幂等键、负载 SHA-256 和创建时间。
- 签名验证对错误公钥、算法、Key ID、非规范 Base64、错误签名字节数、字段替换和畸形信封失败关闭；换行等控制字符不能进入固定签名字段造成分隔歧义。
- PC 与 SDK 新写入 V2 离线归档，使用 PBKDF2-SHA256 310000 次、16 字节随机盐、12 字节随机 Nonce 和 AES-256-GCM 128 位认证标签。
- V2 把算法参数、盐、Nonce、明文摘要和创建时间纳入固定顺序 AAD。错误口令、元数据篡改或密文篡改均无法解密。
- 两端均在派生密钥前严格检查 Schema、固定算法、迭代数、RFC3339 时间、规范 Base64、字段长度和 6 MiB 明文/密文边界。
- PC 写入的 V2 已由 SDK 解密，SDK 写入的 V2 已由 PC 解密；独立 Node AES-GCM 实现也能按文档化 AAD 解开 SDK V2。
- PC 与 SDK 均继续解密历史 V1，且不会再创建新的 V1 归档。

## 本次验证结果

```text
SDK Node tests: 65 passed, 0 failed
PC TypeScript typecheck: passed
PC open-commerce contracts and crypto interop: passed
PC production build: passed
```

执行入口：

```powershell
npm --prefix sdk/open-commerce-connector test
npm --prefix pc-frontend run typecheck
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
```

## 仍未完成

- 没有验证真实用户文件选择、浏览器下载、跨设备传输和上传后的服务端 V5 导入全链路。
- 没有口令找回、托管、同步、轮换、硬件保护或第三方恢复机制。
- 来源运营方首次公钥分发仍需消费者通过独立可信渠道核验；签名不是平台身份认证。
- 本批没有部署 PC、服务端或 SDK 包，也没有连接外部平台、真实订单、支付或链上资产。
