---
title: 开放商业消费者偏好档案与关系级披露 V1 验收
status: current
date: 2026-08-02
owners: backend, frontend
---

# 开放商业消费者偏好档案与关系级披露 V1 验收

## 验收范围

- 当前用户可以保存、读取和删除低敏结构化偏好档案；
- 档案按项目和用户隔离，更新增加修订号；
- 只有有效且包含 `preference.remember` 的本人关系可以生成字段级披露快照；
- 商户只读取仍有效关系的匿名披露，关系撤销或到期后失败关闭；
- 删除档案同步删除本项目内该用户的披露快照；
- HTTP、MCP 和 PC 共用领域约束，审计不保存偏好值。

## 自动验证

2026-08-02 已执行：

- `validate-rust.ps1 ... test ... open_commerce_consumer_preference_tests`：通过；
- `validate-rust.ps1 ... test ... open_commerce`：通过，覆盖既有开放商业回归；
- `validate-rust.ps1 ... check --all-targets --manifest-path server/Cargo.toml`：通过；
- `node scripts/test-open-commerce-pc-workspace.js`：通过；
- 新增和改动的开放商业 PC 文件定向 ESLint：通过；
- `npm run typecheck`：通过；
- `npm run build`：通过。

全仓 `npm run lint -- --quiet` 未形成通过结果，唯一错误来自未修改的 `pc-frontend/src/features/conversation/ConversationPage.tsx:227`：已有 `react-hooks/exhaustive-deps` 禁用注释目前没有对应问题，触发 `Unused eslint-disable directive`。本批未修改该无关文件，定向 lint 已覆盖全部新增和改动的开放商业 PC 文件。

## 人工边界复核

- 保存档案不会自动向商户披露，也不会自动改变发现请求；
- 披露只复制用户明确选择的字段，档案后续更新不会自动同步；
- 商户响应和界面不依赖消费者账号、用户 ID 或消费者项目 ID；
- 关系续期后必须由消费者针对新匿名关系再次披露；
- 当前不是敏感数据保险箱，不承诺端到端加密、跨运营方迁移或外部系统删除。
