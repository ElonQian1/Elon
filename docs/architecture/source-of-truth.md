# 源代码归属与唯一实现

本文是项目代码入口的唯一导航。新功能开始前先确认目标入口，避免把改动写入兼容实现或历史页面。

## 当前归属

| 产品入口 | 唯一主实现 | 允许的后端改动 | 说明 |
|---|---|---|---|
| `/pc` | `pc-frontend/` | `server/src/` 对应领域模块 | PC 主入口，所有新 PC 功能优先进入这里 |
| `/pc-next` | `pc-frontend/` | 同上 | 同源兼容别名，不得维护第二套实现 |
| `/pc-legacy` | 发布脚本从历史提交导出的快照 | 发布/回滚脚本 | 只读回退入口，禁止新增功能 |
| `/web`、`/` | `server/src/assets/web_page.html` 及其资源 | 移动 Web 对应服务端模块 | 移动网页版，不是 PC 旧代码 |
| Android App | `android/` | `server/src/` 对应 API | 原生客户端，不能按 Web 旧代码删除 |
| Windows 节点 | `server/pc-dev-runtime/`、`server/src/node_*` | 同领域节点模块 | 本机执行、节点通信和任务恢复 |

## 开发规则

1. PC 页面只改 `pc-frontend/`；只有 API、权限、数据库或服务端业务才改 `server/src/`。
2. `/pc-next` 不得出现独立页面逻辑；它只能继续指向新版构建产物。
3. `/pc-legacy` 只用于回退和对照，不接受新产品功能。
4. `server/src/assets/web_page.html`、`project_plaza.js/css`、`project_home.js/css` 属于移动 Web 运行链路。除非任务明确标注“移动 Web”，否则不要修改。
5. Android UI 改动只进入 `android/`；需要同步移动 Web 时，必须在任务说明中明确列出两个入口和各自测试。
6. `*_migration.rs`、兼容协议和旧客户端解析逻辑在完成数据/客户端迁移审计前不得删除。

## 发布前检查

- `/pc` 和 `/pc-next` 均加载同一新版 dist。
- `/pc-legacy` 未被主入口引用。
- 移动 Web 的改动没有被误认为 PC 改动。
- 新功能对应的主实现、API、测试和发布路径已记录在迁移矩阵中。

