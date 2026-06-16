# AI Rules Bridge

本文件只做规则桥接，避免多套规则漂移。

规则权威来源：

1. `AGENTS.md`
2. `.github/copilot-instructions.md`
3. `.github/instructions/*.instructions.md`

## 通用硬边界

- 不覆盖用户或其他 AI 的未提交改动。
- 修改前先了解项目结构，不直接大范围重构。
- 新建文件要显式 `git add`。
- 不提交密钥、token、证书、签名文件。
- 不把生成目录、构建产物、依赖缓存加入版本库。
- 修改后运行最小有效验证命令。
- 最终说明改动、验证、风险和未完成项。

如本文件与 `.github/copilot-instructions.md` 冲突，以 `.github/copilot-instructions.md` 为准。
