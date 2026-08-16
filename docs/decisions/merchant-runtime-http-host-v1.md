---
title: 开放商业商户运行时 HTTP 宿主 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-16
implementation_status: implemented_locally_verified
---

# 开放商业商户运行时 HTTP 宿主 V1

## 决定

1. `@elon/open-commerce-connector/merchant-runtime-http` 提供独立 Node 宿主，不让连接器
   根入口导入或启动 HTTP 服务。商户仍显式组合运行时、幂等存储和业务处理器。
2. 默认监听 `127.0.0.1`，以可信反向代理后的 HTTP 源站为首选部署方式。直接 HTTPS
   是同一宿主的可选模式，只消费调用方提供的 Node TLS 选项，不创建证书生命周期。
3. 固定 `POST /commerce/v1/invoke` 和 `GET /healthz`。调用路由必须把原始字节与 Node
   规范化请求头直接交给 `runtime.handleInvoke`，不能解析后重新序列化签名正文。
4. 宿主在运行时前拒绝错误方法、路径、媒体类型、内容编码、声明或实际超限请求以及
   `100-continue`；Node 解析错误只返回空的通用 400。
5. 既有运行时错误信封原样作为有界 JSON 返回。注入运行时抛错、非法状态或非法正文时，
   宿主只返回稳定 `merchant_runtime.http_error.v1`，不返回异常消息和堆栈。
6. 宿主自己跟踪在途异步处理器，不能只依赖 Node `server.close()` 回调判断完成。停机先
   进入 `draining` 并停止接收连接；在途归零后关闭空闲连接，截止时销毁剩余连接。回执
   区分正常排空和强制关闭，且不把强制关闭解释为业务任务完成。
7. 健康响应只包含 `ready/draining`，不携带商户、能力、密钥、订单、目录或资金状态。

## 兼容与回滚

- 现有框架自建宿主继续可直接调用 `runtime.handleInvoke`，新子路径是可选能力。
- 默认路径与既有商户接入手册的 `/commerce/v1/invoke` 保持一致。
- 停止导入新子路径即可回滚；运行时、SQLite 幂等数据库和 ERP 数据不迁移、不改写。
- HTTP 和 HTTPS 使用相同请求处理器，切换 TLS 终止位置不改变签名信封协议。

## 非目标

本决定不提供证书签发、DNS、反向代理、系统服务、生产密钥、发布连接器、服务发现、支付、
外部平台适配器、多机幂等或公网部署验收。
