# 开放商业消费者可携带偏好字段级采用 V1 验收

## 当前状态

`verified_rust_http_pc_build`

## 已验证

- 6 项 Rust/SQLite 与进程内 Axum 专项全部通过，覆盖五种字段逐项采用、组合采用、全选、重复字段去重、空选、未知字段、未变化字段和明确确认。
- 有当前档案时只改变所选字段；无当前档案时从默认空档案建立修订 1，回滚会恢复完整旧档案或删除本次新建档案。
- 预演后的修订漂移失败关闭，不创建采用记录；采用后档案未变化时可完整回滚，旧 V141 记录继续从前后快照派生 `selected_fields`。
- HTTP 路由验证未登录、非项目成员、同项目其他用户、其他项目和采用记录回滚隔离；列表只返回当前用户自己的记录。
- 审计只记录导入 ID、修订号、所选字段和来源信任状态，不写入城市、类别或标签值。
- PC 的服务端 nullable 修订类型已与 JSON 契约统一，严格 TypeScript、开放商业契约回归和 Vite 生产构建通过。

## 验证命令

- `scripts/validate-rust.ps1 ... test --manifest-path server/Cargo.toml portability_adoption -- --nocapture`：6 passed，验证指纹 `48758c99341682622440a9430520853c405be41fb414db3e4c4bba87b8d4e323`。
- `npm run typecheck`：通过。
- `npm run test:open-commerce`：通过。
- `npm run build`：通过。

## 未完成

- 真实 TCP 监听、浏览器点击和窄屏交互、生产历史库副本及跨运营方文件传输尚未验证。
- 带共同祖先的三方历史合并、跨设备审批、关系恢复和业务数据迁移；多来源静态偏好合并由独立 V1 承接。
