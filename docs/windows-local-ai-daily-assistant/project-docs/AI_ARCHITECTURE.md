# AI Architecture Guide

最后更新：2026-07-09

## 总体分层

```text
Windows 桌面 UI
  -> 应用状态和设置
  -> Recorder 采集服务
  -> Privacy Guard 隐私保护
  -> AI Analyzer 分析队列
  -> Timeline Engine 时间线合并
  -> Report Generator 日报生成
  -> SQLite 本地存储
  -> 可替换 AI Provider
```

## 核心模块边界

| 模块 | 职责 | 不应该做什么 |
|---|---|---|
| Desktop UI | 设置、状态、时间线、日报预览 | 不直接调用系统截图 API |
| Recorder | 定时截图、前台应用、窗口标题 | 不决定是否上传云端 |
| Privacy Guard | 暂停、排除应用、敏感策略、原图保留 | 不生成日报内容 |
| AI Analyzer | 调用 AI Provider，校验结构化输出 | 不直接保存永久截图 |
| Timeline Engine | 合并记录、统计应用时长 | 不调用截图或 AI API |
| Report Generator | 生成 Markdown 日报 | 不覆盖用户手动编辑内容 |
| Storage | SQLite 读写、迁移、删除 | 不藏业务规则 |
| AI Provider | 云端、本地、Mock 模型适配 | 不绕过 Privacy Guard |

## 主要数据流

```text
用户开启记录
  -> Recorder 检查采集间隔
  -> Recorder 获取前台应用和窗口标题
  -> Privacy Guard 判断是否暂停或命中排除应用
  -> 允许时截屏并生成临时图片
  -> AI Analyzer 调用 Provider
  -> 校验 AI JSON
  -> 保存文字化 capture_event + analysis_record
  -> 删除临时截图
  -> Timeline Engine 合并时间线
  -> UI 展示
  -> 用户编辑
  -> Report Generator 生成 Markdown
```

## 数据事实来源

- `settings`：用户配置、采集间隔、AI Provider、隐私选项。
- `capture_events`：每次采集事件。
- `analysis_records`：AI 分析结果。
- `timeline_segments`：合并后的可编辑时间线。
- `reports`：用户生成和编辑过的日报。

截图原图默认不是事实来源。原图只应作为临时输入。

## 错误处理原则

- 截图失败：记录错误，不退出应用。
- 窗口标题读取失败：记录为 `unknown`。
- AI 调用失败：保留本地采集事件，标记分析失败。
- JSON 校验失败：保留原始错误摘要，不信任模型输出。
- 数据库写入失败：停止采集并提示用户。
- 命中隐私规则：跳过截图和上传，只记录跳过原因。

## 架构约束

- 采集、分析、日报生成必须解耦，不能写成一个大函数。
- 任何上传截图的路径必须经过 Privacy Guard。
- 所有 AI 输出必须结构化校验，不能直接渲染原始模型文本。
- 用户手动编辑过的时间线不得被后台自动覆盖。
- 数据删除必须可验证，不能只隐藏 UI。
- 后台任务必须可取消，退出应用时不能留下未知子进程。
