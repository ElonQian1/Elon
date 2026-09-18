# 子项目轻量 AI 工作流模板

给"单代理顺序开发"的子项目（不需要多代理并发 worktree 隔离）用的最小 git 纪律工具。
如果某个子项目经常有多个 AI 代理同时改同一仓库，改用主项目 `scripts/ai-task-preflight.ps1`
+ `scripts/finish-ai-task.ps1` 那套完整 worktree 隔离机制，不要用这个轻量版。

## 包含文件

- `ai-task-sync-check.ps1`：任务开始前运行。同步 main 到 origin/main（fast-forward-only），
  报告工作区是否干净、是否落后远端。**不建 worktree，不上锁**。
- `ai-task-finish-check.ps1`：准备宣布"完成"前运行。确认工作区干净且本地 HEAD 与
  `origin/<当前分支>` 一致（即已经 push），只有看到 `FINALIZABLE=true` 才能对用户说已完成。

两个脚本都用 `git rev-parse --show-toplevel` 定位仓库根，纯 git 操作，不依赖任何
elon-cli 专属的 `.ai/workspace-policy.txt`、Rust 缓存清理、Kind 分类判定，可以直接复制到
任意子项目仓库的 `scripts/` 目录使用。

## 接入步骤（新子项目）

1. 复制这两个 `.ps1` 文件到子项目仓库的 `scripts/` 目录（保留 UTF-8 BOM 编码，Windows
   PowerShell 5.1 没有 BOM 会把中文字符串解析报错）。
2. 在子项目的 `AGENTS.md` 或 `.github/instructions/*.md` 里补一条规则：
   - 任务开始先跑 `ai-task-sync-check.ps1`；`DIRTY=True` 时先确认改动归属，不自动覆盖或丢弃。
   - 完成一个独立改动单元就立即 `git add` + `git commit` + `git push origin HEAD:main`，
     不要攒到任务结束才一次性提交。
   - 宣布完成前跑 `ai-task-finish-check.ps1`，必须看到 `FINALIZABLE=true` 才能说"已完成"。
3. 按子项目自己的验证入口（如 `scripts/validate.ps1`）补验证步骤，这两个脚本只管 git 状态，
   不做代码验证、不做发布判定。

## 参考实现

`d:\rust\active-projects\yilong-quant\scripts\ai-task-sync-check.ps1` 和
`ai-task-finish-check.ps1` 是本模板的首个落地实例，`.github\instructions\project-workflow.instructions.md`
是对应的接入文档写法参考。
