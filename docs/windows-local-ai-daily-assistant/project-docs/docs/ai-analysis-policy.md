# AI 分析约束

最后更新：2026-07-09

## 一句话结论

AI 只能把截图转成结构化工作摘要，不能保存原图、不能输出敏感原文、不能绕过隐私判断。

## Provider 分层

必须至少设计三类 Provider：

- `MockVisionProvider`：开发和测试用，不上传真实截图。
- `CloudVisionProvider`：云端 Vision API。
- `LocalVisionProvider`：预留本地模型接口。

默认开发阶段使用 Mock。

## 输入

AI Analyzer 可以接收：

- 压缩后的截图。
- 前台应用名。
- 窗口标题。
- 当前时间。
- 用户配置的分类标签。

AI Analyzer 不应该接收：

- API key 明文日志。
- 不必要的历史截图。
- 数据库全量记录。
- 用户未同意上传的隐私应用截图。

## 输出 JSON

AI 输出必须校验为结构化 JSON：

```json
{
  "summary": "正在整理产品需求文档",
  "category": "文档",
  "confidence": 0.86,
  "privacy_risk": "low",
  "visible_sensitive_content": false,
  "todo_or_risk": "后续需要确认截图保存策略"
}
```

## 分类

MVP 分类：

- 开发
- 会议
- 沟通
- 文档
- 测试
- 设计
- 运维
- 数据分析
- 学习
- 管理
- 产品
- 生活/非工作
- 未识别

## Prompt 约束

Prompt 必须要求模型：

- 不输出密码、Token、验证码、身份证号、手机号等敏感原文。
- 不复述完整聊天内容。
- 只总结工作行为和任务方向。
- 给出置信度。
- 标记疑似隐私风险。
- 不确定时输出低置信度，不要编造。

## 成本控制

MVP 必须考虑：

- 默认 5 分钟采集间隔。
- 连续相同应用和标题时降频。
- 截图压缩。
- 空闲状态不上传。
- 失败重试要有限制。
