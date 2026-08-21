---
version_status: current
reviewed_at: 2026-08-21
implementation_status: verified
---

# ERP 商户托管节点 V1 验收

## 结论

`cofficethinking` 已具备“一台电商子服务器共享一个发布版本、每个商户运行独立实例”的托管基础。咖啡店继续作为首个真实参考租户，新增商户通过配置和实例合同安装，不再复制咖啡店业务源码。

该结论只覆盖托管节点基础，不表示商户自助开通、正式账号体系、数据库自动创建、反向代理、TLS、第三方平台授权或真实生产部署已经完成。

## 已交付

主项目：

- `contracts/erp/managed-merchant-instance-v1.schema.json`：跨仓部署合同真源；
- `docs/requirements/erp-managed-hosting-node-v1.md`：目标、安全边界和验收标准；
- Feature Registry 条目 `erp-managed-hosting-node-v1`：供其他 AI 发现、认领和检查漂移。

商户模块服务器 `cofficethinking`，提交 `8c687f386db9f6dca2c178591aa2a893b55bc9ed`：

- 与主项目一致的合同 Schema 和不含秘密的实例示例；
- 可托管的通用零售 ERP 配置，保留订单、采购、库存、财务、会员、报表和开放商业能力；
- Python 合同校验器和脱敏环境渲染器；
- 默认预演、显式 `--apply` 的无 Docker systemd 安装器；
- 每实例独立系统用户、进程、端口、数据库凭据、密钥、上传目录和用户浏览器目录；
- systemd 写目录限制及基础进程加固；
- 对尚未完成商户隔离的 `automation.browser` 失败关闭。

## 验证证据

在 `cofficethinking` 独立干净 worktree 执行：

```text
python tests/test_merchant_hosting_contract.py -v
结果：10 项通过，Windows 内部 Bash 子测试按平台条件跳过 1 项

bash -n scripts/hosting/provision-merchant-instance.sh
结果：通过

主项目 Schema SHA-256 == cofficethinking Schema SHA-256
结果：1C984563A5C0C1C4F9F57C37A2DCA84EB525D6D73E0C53EE4DCE60E4394B868A
```

测试覆盖未知字段、重复 JSON 字段、公开路径绑定、必需秘密但不回显秘密、开放商业网关门禁、共享平台浏览器拒绝、两个商户运行边界不同、仓库示例有效和 systemd 写目录限制。

现有服务源码还确认读取安装器生成的 `PORT`、`WEB_STATIC_DIR`、`UPLOADS_DIR`、`MODULE_SERVER_PROFILE_PATH`、开放商业商户/门店身份、运行密钥引用、管理令牌和用户浏览器目录。

## 生产边界

| 能力 | 当前状态 |
|---|---|
| 通用 ERP 配置与复用同一发布版本 | 已完成 |
| 每商户独立进程、端口、用户、目录和秘密 | 已完成 |
| 每商户独立 PostgreSQL 合同 | 已要求，数据库仍由受控运维创建和迁移 |
| 默认脱敏预演与显式安装 | 已完成 |
| 咖啡店真实线上服务迁移 | 未执行，本批不改线上服务 |
| 商户自助开通、停用、升级和回滚控制面 | 未完成 |
| 正式登录、租户 RBAC 和审计 | 未完成 |
| Rust 原生路径路由、TLS 和健康门禁 | V2 代码已加入 ACME 与安装入口；真实 Linux、DNS、443 和 CA 签发仍未执行 |
| 平台诊断浏览器每商户 Worker | 未完成，托管配置拒绝启用 |
| 美团、抖音、京东、淘宝闪购生产授权 | 未完成 |

## 下一阶段

主项目下一步应把已存在的 Rust 原生入口接入托管控制面：生成合同、远程预演、受控建库迁移、候选版本健康验证、Runtime Binding 原子切换和单商户回滚。随后再接商户自助入口和正式账号/RBAC，不应把商户数据隔离寄托在前端传入的 `store_id` 上。
