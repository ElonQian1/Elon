# fb2 计费策略

## 固定口径

ASR、TTS 和上下文拉取是聊天基础体验，不是 AI 生成内容。只有模型生成回复文本才消耗 token/额度。

## 免费通道

| 通道 | 接口/位置 | 计费 |
|---|---|---|
| Android 系统 ASR | 手机系统 `SpeechRecognizer` | 免费 |
| 云端 ASR 兜底 | `POST /api/voice/asr` | 免费 |
| Android 系统 TTS | 手机系统 TTS | 免费 |
| 主项目 TTS | `POST /api/voice/tts` | 免费 |
| fb2 Context Pack 拉取 | `GET /api/main-project/context/pack` | 免费 |
| Chat bootstrap/contract | `/api/external/apps/fb2/*` | 免费 |

免费不等于无限制。仍然必须保留：

- 登录鉴权
- 服务令牌
- 文件大小限制
- 录音时长限制
- 请求频率限制
- 安全审计日志

## 扣费通道

| 通道 | 计费点 |
|---|---|
| 群聊 @AI 生成回答 | 模型调用前检查额度，按输出/模型用量记录 |
| AI 助手回答 | 模型调用前检查额度 |
| 赛事分析生成文本 | 模型调用前检查额度 |
| 用户订单/票据剖析生成文本 | 模型调用前检查额度 |
| 群聊总结帖生成 | 模型调用前检查额度或按平台策略记账 |

## fb2 试用额度

fb2 用户首次创建主项目会话时，主项目可以按配置发放 AI 回复试用额度。

当前主项目已有配置项：

```text
external_app_fb2_trial_credit_fen
```

使用原则：

- 额度只用于 AI 回复/模型调用。
- 余额为 0 时，仍允许 ASR/TTS/context fetch。
- AI 回复余额不足时，应返回明确文案，引导领取、充值或等待平台策略。
- fb2 客户端可通过 `GET /api/external/apps/fb2/chat-bootstrap` 的 `billing` 字段读取余额接口和检查点。
- 当前余额读取：`GET /api/me/balance`。
- 扣费明细读取：`GET /api/me/billing`。

## 排障判断

如果 fb2 语音转文字失败，不应先怀疑额度。优先看：

- 麦克风权限
- 系统 ASR 回调
- 录音文件大小
- `/api/voice/asr` 返回码
- MIME/format
- 主项目 token

如果 AI 没反应，再检查：

- fb2 用户是否成功换取主项目 token。
- 用户是否加入默认群。
- 群消息是否成功进入主项目。
- 是否触发 `@EL` 或 AI 回复接口。
- AI 回复层是否有试用额度或付费额度。
