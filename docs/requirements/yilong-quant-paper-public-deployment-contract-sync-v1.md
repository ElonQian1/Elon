---
version_status: current
reviewed_at: 2026-09-02
implementation_status: verified
---

# 一龙量化 Paper 公开部署合同同步 V1

## 目标

把 `yilong-quant` 已验证的 Paper 公开部署合同 V9 同步到一龙主项目的集成文档和 AI 导航，让远程开发人员能准确区分“配置就绪、部署合同已验证、目标环境已部署”，并知道主项目何时可以安全配置量化 Web 入口。

量化实现提交固定为 `0b87604e9105d7b0c1e4ba0da6b8b2c3c43d6ddc`。本同步只记录跨仓库合同，不修改 Server、PC 前端、官方目录快照或运行配置；量化 Web 客户端继续显示“计划中”。

## 范围

- 在 `docs/yilong-quant-integration.md` 记录量化 V9 的编译时 Git 身份、安全 Nginx/systemd 模板和无凭据公网只读验收器。
- 明确 `configuration_ready`、`deployment_contract_verified` 与 `environment_deployed` 三个状态不可互相替代。
- 明确主项目只有在批准的量化 HTTPS origin 通过真实网络验收后，才能配置 Paper Web URL；当前 HTTP 主站或未知 HTTPS 主机不能用来绕过 exact-origin、CSP 或证书边界。
- 更新 `AI_INDEX.md`，让后续 AI 代理能找到本需求、量化接入边界和目标环境待办。
- 通过 Feature Registry 绑定需求、文档和测试证据，供远程协作者认领、校验漂移和接续工作。

## 非目标

- 不分配域名、申请证书、登录服务器、修改 Nginx/systemd、写入生产秘密或部署量化 PWA/API。
- 不修改主项目 grant 载荷、签名、五分钟有效期、scope、签发接口或一键启动协议。
- 不修改官方项目目录内容；Web、Windows 和 Android 客户端状态继续为 `planned`。
- 不启用 sandbox/live，不接收、托管或移动真实资金，不发行真实代币，不证明付款/KYC/钱包准入，也不承诺固定或保本收益。

## 跨仓库合同

- 量化仓库负责构建身份、loopback API、同源 `/api`、HTTPS/HSTS/CSP、Paper runtime、exact parent origin 和公网只读验收器。
- 主项目负责项目广场、短期 Paper grant 和未来经批准的量化 Web URL；它不复制量化部署模板、业务源码、数据库或秘密。
- 离线 fixture 回执固定为 `scope=offline_fixture`、`network_calls_made=false`，只能证明合同；真实目标必须由量化仓库 `scripts/check-paper-public-deployment.ps1` 验收并返回 `scope=public_https_read_only`、`network_calls_made=true` 和 `status=ready`。
- 任一 HTTPS、Git SHA、安全响应头、Paper-only、资金、live 或 parent origin 检查失败时，主项目入口保持失败关闭。

## 验收标准

1. 主项目接入文档记录量化提交 `0b87604e9105d7b0c1e4ba0da6b8b2c3c43d6ddc`、V9 三态语义及真实公网验收前置门禁。
2. 文档明确量化离线合同已验证，但没有批准的 HTTPS origin/服务器权限，不能标记 `environment_deployed`。
3. `AI_INDEX.md` 能定位本需求和接入文档；官方目录、Server、PC 前端与运行配置保持不变。
4. 文档模块化、来源体积和量化跨仓库文案合同检查通过；Feature Registry 需求与实现证据零漂移。
5. 提交推送主项目 `main`，不触发 Server、PC、Android 或量化环境发布。

## 回滚

回退本次文档提交即可移除主项目的 V9 同步说明；不会改变量化仓库实现、主项目运行状态、数据库、grant 或用户资产。回滚后不得把较旧文档理解为量化环境已经部署。
