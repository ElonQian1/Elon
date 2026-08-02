# 开放商业消费者关系凭证 V1 验收

## 已验证闭环

- 消费者在项目内对已发布商户建立 30/90/365 天的关系凭证，PC 默认 90 天。
- 固定范围仅表达“可关联主动提供的偏好”或“可关联商户会员标识”，数据库不保存偏好原文、联系方式或订单。
- 关系列表按当前用户隔离；同一项目的其他成员不能读取或撤销本人凭证。
- 商户项目可读取指向自己的关系历史，但响应不含消费者用户 ID、消费者项目 ID 或账号。
- 同一消费者重新建立关系时，旧凭证在同一事务内撤销，新凭证生成新的随机匿名标识。
- 过期时间异常时按过期处理；消费者撤销幂等，已撤销关系不会自动恢复。
- `pc-web` 不能冒充 `mcp-client`，普通开发者也不能注册系统保留 App ID。
- HTTP、MCP 与 PC 共用同一关系领域服务和权限规则。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -Domain open-commerce-consumer-relationships -- test `
  --manifest-path server\Cargo.toml open_commerce_relationship -- --nocapture

Set-Location pc-frontend
npm run build
npm run test:open-commerce
```

## 尚未覆盖

- 消费者偏好数据保险箱、字段级授权和数据导出。
- 订单、会员系统与匿名关系标识的真实业务绑定。
- 到期提醒和续期向导。删除请求回执及商户声明已由独立 V1 实现，但真实外部删除证明仍未完成。
- 跨运营方可验证身份、联邦关系迁移与公共信誉。
- 真实支付、链上关系对象和争议处理。

因此，本批只能描述为“消费者可撤销、商户脱敏可见的关系授权凭证 V1 已实现”，不能描述为“消费者数据网络或生产身份体系已经完成”。
