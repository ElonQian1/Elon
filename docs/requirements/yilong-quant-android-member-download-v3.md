---
version_status: current
reviewed_at: 2026-09-12
supersedes: yilong-quant-android-public-download-v2
---

# 官方量化 APK 成员专属下载 V3

## 目标

主 APK、Win 项目广场和量化项目介绍页继续提供 `yilong-quant` 的安装与更新入口。项目没有完全开放期间，只有成功加入项目且当前仍有成员访问权的登录用户可以取得 APK。

本需求取代 V2 的匿名公共下载规则。固定 URL 只是稳定入口，不代表公开授权。

## 服务端合同

1. `GET /api/store/projects/yilong-quant/downloads/android` 必须从 `Authorization` 请求头验证登录，再调用项目成员访问检查；匿名或只在查询参数传 token 返回 `401`，非成员和失效成员返回 `403`。
2. 成员检查通过后，只返回最新通过官方量化发布门禁、位于主服务器托管目录且大小和 SHA-256 校验一致的 APK。响应使用 `Cache-Control: no-store`。
3. 公共项目列表不给非成员投影 `latest_apk_url`；公开项目空间访客也不取得最新 APK 入口。
4. 旧 `/api/user/:user_id/projects/:project_id/download/:filename` 路由处理官方量化时，认证用户必须和路径用户一致，并再次检查项目成员关系。
5. 其他项目现有公开下载和成员下载行为保持兼容。

## 客户端合同

- 主 APK 从活动主服务器构造固定下载 URL，要求本地登录 token，并通过独立、禁止重定向的客户端把 token 放入 `Authorization` 请求头。URL、文件名和错误信息中不得包含 token。
- Win 项目广场和项目介绍页对官方量化使用带认证请求取得 Blob 后下载；直接复制公开 URL 仍会由服务端拒绝。
- 访客界面显示“加入后下载”；成员资格以服务端实时结果为准，不信任前端按钮状态。
- APK 下载后继续执行既有包名、唯一证书、最低版本、源码身份和只读临时文件检查。

## 验收与回滚

自动测试必须覆盖匿名 `401`、已登录非成员 `403`、成员空发布 `404`、固定 URL 不含 token、请求头认证、禁止重定向和旧用户路径防冒用。若入口故障，可以把 Android 状态暂时改为不可用，但不得回滚为匿名下载。
