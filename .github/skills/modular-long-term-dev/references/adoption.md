# 跨项目安装与增量接入

共享来源：`ElonQian1/Elon` 仓库中的 `.github/skills/modular-long-term-dev`。整个目录自包含，不需要一龙服务、部署工具、磁盘布局或其他 Skill。

## 安装

在目标电脑的 Codex 中输入：

```text
$skill-installer 从 https://github.com/ElonQian1/Elon/tree/main/.github/skills/modular-long-term-dev 安装 skill
```

安装到个人 skills 目录后可供该电脑上的其他项目使用；GitHub 登录本身不会自动同步所有电脑。团队项目也可将完整目录版本化到 `.agents/skills/modular-long-term-dev/`，在 `AGENTS.md` 中引用，并记录来源 commit。更新前先比较本地改动，不能直接覆盖定制版本。

安装机制参考 [官方 Skills 文档](https://learn.chatgpt.com/docs/build-skills)。

## 接入现有项目

1. 检查现有 `AGENTS.md`、CI、Git hooks、源码与生成目录，只按需要合并接入，不覆盖已有流程。
2. 复制 `assets/modularity.config.json` 到仓库根 `.modularity.json`；用 `roles` 配置明确的角色路径，`exclude` 只记录生成/第三方路径及原因。
3. 执行 `audit --json`，审阅超限文件。已有工作区可能包含未提交开发，初次接入可以记录当前现场，但必须在接入文档中披露。
4. 运行 `baseline --reason "采用当前历史债务，后续只减不增"`。只记录超限路径，不记录正常文件；文件已存在时该命令拒绝覆盖。检查只读，绝不自动刷新基线。
5. 在 `AGENTS.md` 中要求开始修改前核对预算、按职责拆分、完成前执行门禁。加入项目检查命令；在已有 CI 中合并步骤，或添加独立工作流。
6. 执行 `check`、工具的 `node --test <skill-dir>/scripts/*.test.mjs`，确认门禁真实生效后交付。

CLI 全部支持 `--root <repo>`；配置固定为 `.modularity.json`，债务固定为 `.modularity-baseline.json`。输出退出码：0 通过，1 尺寸/基线违规，2 参数/文件/Git/配置错误。

## 检查模式与基线

- `check`：已跟踪文件和未被 Git 忽略的新文件的当前内容，适合开发完成前使用。
- `check --staged`：只读取 Git index，包括暂存的配置和基线，适合 pre-commit；未暂存的缩小不能掩盖即将提交的巨型文件。
- `check --base <sha>`：全仓检查，并读取该提交的基线。已建立的基线不能增加路径或额度；历史大文件也受该提交中实际尺寸的更小上限约束。首次接入可以新增经审阅的基线。
- `audit --json`：报告尺寸和债务，退出 0；它用于盘点，不作为 CI 门禁。
- `ratchet`：将已有基线的数值收紧为当前更小的值，移除已解决/删除的债务；先检查违规，拒绝扩大或新增债务。应在开始下一项开发前、或本次缩小文件后执行并审阅 diff。

物理行包含空行与注释；CRLF/LF 的字节数按 LF 归一化，避免 Windows/Linux 换行误报。UTF-8 字节预算包含非 ASCII 字符。符号链接和 Git 子模块不跟随；子模块应独立接入门禁。未知配置键、无效阈值、缺失基线引用和损坏的 UTF-8 都直接报错。

配置路径模式使用 `/`；`*` 匹配一个目录段内，`**` 匹配任意深度，`**/` 也匹配根目录。`roles` 按顺序取第一个匹配项。默认排除构建/依赖目录，手写源码禁止放到这些目录中规避检查。JSON 数据/锁文件和二进制资产不在源码门禁内；如需要大资产政策，应另行按项目用途制定。

## CI 与 hooks

Node 和 Git 可用时即可运行，不需要 `npm install`。拉取完整历史后，PR 使用目标分支 SHA，push 使用事件的 before SHA；全零 before（首次推送）只做普通检查。不要写死默认分支名。

CI 示例步骤：

```sh
node .agents/skills/modular-long-term-dev/scripts/modularity.mjs check --base "$BASE_SHA"
```

在已有 pre-commit 末尾增加 `check --staged --base HEAD`，或者用项目现有 hook 管理器接入。首次无 HEAD 的仓库省略 `--base`。不要覆盖现有 hook 或静默改变全局 `core.hooksPath`。CI 基线防放宽用于发现误操作；规则脚本和配置本身仍需要代码审查，不是针对恶意修改的安全边界。
