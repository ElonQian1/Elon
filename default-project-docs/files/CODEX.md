# Codex 桥接说明

Codex 在本项目中使用 `.github/copilot-instructions.md` 作为共享规则权威来源。

## 开始任务

1. 先读取 `AGENTS.md`。
2. 再读取 `.github/copilot-instructions.md`。
3. 只按当前任务读取 `.github/instructions/*.instructions.md` 中相关文件。

## 规则维护

本文件只做 Codex 入口桥接，不复制通用规则。需要新增或修改项目规则时，优先更新 `.github/copilot-instructions.md` 或专项 instructions 文件。
