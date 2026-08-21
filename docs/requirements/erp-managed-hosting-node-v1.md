---
version_status: current
reviewed_at: 2026-08-21
implementation_status: verified
---

# ERP 商户托管节点 V1

## 目标

把 `cofficethinking` 从单一咖啡店项目演进为一龙开放商业网络的通用商户 ERP 参考实现，并允许同一台电商子服务器复用同一发布版本托管多个互不信任的商户。

首期采用“共享机器、隔离实例”：每个商户使用独立进程、数据库、系统用户、端口、运行配置、密钥、上传目录和浏览器 Profile。咖啡店是首个租户和行业插件，不是公共内核身份。

## 用户工作流

1. 商户在一龙项目中选择 ERP 公共内核、行业插件和主题。
2. 一龙生成不含秘密的托管实例合同。
3. 电商托管节点预演合同并报告端口、目录和身份冲突。
4. 受控运维准备独立数据库、迁移和秘密文件后显式安装实例。
5. 节点启动独立 systemd 服务，一龙为该商户登记独立 Runtime Binding。
6. 消费者 AI 继续通过目录、授权、确认、计量和签名调用访问商户能力。
7. 单个商户可以独立停用、升级或回滚，不中断其他商户。

## 安全边界

- 禁止通过前端传入不同 `store_id` 冒充租户隔离。
- 禁止互不信任的商户共享数据库账号、运行密钥、平台 Cookie、上传目录或操作系统用户。
- 部署合同不得保存数据库地址、密码、Token、Cookie 或共享密钥。
- ERP 管理 API、MCP、浏览器控制和上传目录不因实例安装而自动暴露公网。
- 平台诊断浏览器在完成每商户凭据和 Profile 隔离前必须禁用，合同校验应失败关闭。
- 当前占位登录未替换前，不得宣称共享进程内多租户管理端安全可用。
- 安装工具默认只预演；数据库创建、迁移、Nginx、TLS 和真实发布必须由独立受控流程完成。

## V1 验收标准

1. 一龙仓和商户模块服务器共享相同的 `yilong.managed-merchant-instance.v1` JSON Schema。
2. 合同拒绝未知字段、无效 UUID、越界端口、不匹配的公开路径和秘密内联字段。
3. 节点工具能够校验模块配置和秘密文件的必需键，但任何标准输出都不包含秘密值。
4. 默认运行只产生脱敏预演；只有显式 `--apply` 且以 root 运行才修改系统。
5. 两个商户合同派生不同的系统用户、端口、状态目录、秘密目标和 systemd 单元。
6. systemd 实例仅能写入本商户状态目录，读取本商户配置和秘密文件。
7. 托管配置拒绝启用尚未完成每商户隔离的共享平台诊断浏览器。
8. 现有咖啡单实例运行合同保持兼容；本批不改写真实咖啡数据库或线上服务。
9. 文档明确区分“托管节点基础完成”和“商户自助开通、正式认证、生产发布闭环完成”。

## 非目标

- 本批不把所有 ERP API 改造成共享进程内行级多租户。
- 本批不自动创建或迁移真实 PostgreSQL 数据库。
- 本批不实现美团、抖音、京东或淘宝闪购官方生产授权。
- 本批不实现支付、退款、配送、资金结算或 Sui 上链。
- 本批不把配置存在等同于第二家真实商户已经上线。

## 实现边界

一龙协议真源：

- `contracts/erp/managed-merchant-instance-v1.schema.json`

商户模块服务器实现：

- `contracts/hosting/managed-merchant-instance-v1.schema.json`
- `scripts/hosting/merchant_instance.py`
- `scripts/hosting/provision-merchant-instance.sh`
- `scripts/systemd/yilong-merchant@.service`
- `tests/test_merchant_hosting_contract.py`
- `docs/managed_merchant_hosting.md`

下一阶段由一龙增加托管节点控制面、候选发布验证、Runtime Binding 安全切换和回滚，再接入商户自助开通入口。
