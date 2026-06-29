curl -X POST 'https://tokenhub.tencentmaas.com/v1/chat/completions' \
  -H 'Authorization: Bearer sk-iJJBNVOtF0ksg10HuOGowIfqGNSexwvDzUai3jWIX1ItmncX' \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "hy3-preview",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "你好"}
    ],
    "stream": false
  }'

curl -X POST 'https://tokenhub.tencentmaas.com/v1/responses' \
  -H 'Authorization: Bearer sk-iJJBNVOtF0ksg10HuOGowIfqGNSexwvDzUai3jWIX1ItmncX' \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "hy3-preview",
    "instructions": "You are a helpful assistant.",
    "input": "你好",
    "stream": false
  }'



步骤 1：安装 Claude Code（需要 Node.js 18+）
MacOS/Linux
npm install -g @anthropic-ai/claude-code
claude --version  # 验证安装
2
步骤 2：编辑配置文件 ~/.claude/settings.json
文件操作说明：
如果 ~/.claude/ 目录不存在，请先创建：mkdir -p ~/.claude
如果 settings.json 文件不存在，直接创建并写入以下完整内容即可
如果 settings.json 已存在，请将下方 env 字段合并追加到已有 JSON 中（注意保留原有配置项）
{
    "env": {
      "ANTHROPIC_BASE_URL": "https://tokenhub.tencentmaas.com",
      "ANTHROPIC_AUTH_TOKEN": "sk-iJJBNVOtF0ksg10HuOGowIfqGNSexwvDzUai3jWIX1ItmncX",
      "ANTHROPIC_MODEL": "hy3-preview",
      "ANTHROPIC_DEFAULT_OPUS_MODEL": "hy3-preview",
      "ANTHROPIC_DEFAULT_SONNET_MODEL": "hy3-preview",
      "ANTHROPIC_DEFAULT_HAIKU_MODEL": "hy3-preview",
      "CLAUDE_CODE_SUBAGENT_MODEL": "hy3-preview",
      "ENABLE_TOOL_SEARCH": false
    }
  }
字段说明：
字段
说明
ANTHROPIC_BASE_URL	API 基地址，固定为 TokenHub Anthropic 兼容地址
ANTHROPIC_AUTH_TOKEN	你的 API Key，用于鉴权认证
ANTHROPIC_MODEL	默认使用的模型，对应你在上一步选择的模型
ANTHROPIC_DEFAULT_OPUS_MODEL	当 Claude Code 需要使用 Opus 级别模型时的替代模型
ANTHROPIC_DEFAULT_SONNET_MODEL	当 Claude Code 需要使用 Sonnet 级别模型时的替代模型
ANTHROPIC_DEFAULT_HAIKU_MODEL	当 Claude Code 需要使用 Haiku 级别模型时的替代模型
CLAUDE_CODE_SUBAGENT_MODEL	子代理使用的模型（用于并行任务处理）
ENABLE_TOOL_SEARCH	是否启用工具搜索功能，设为 false 关闭
配置文件路径：MacOS/Linux: ~/.claude/settings.json | Windows: %USERPROFILE%\.claude\settings.json
3
步骤 3：验证配置
cd your-project
claude
# 启动后输入 /status 确认连接



  from openai import OpenAI

client = OpenAI(
    api_key="sk-iJJBNVOtF0ksg10HuOGowIfqGNSexwvDzUai3jWIX1ItmncX",
    base_url="https://tokenhub.tencentmaas.com/v1",
)

response = client.chat.completions.create(
    model="hy3-preview",
    messages=[
        {"role": "user", "content": "你好，请介绍一下你自己"},
    ],
)
print(response.choices[0].message.content)



import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'sk-iJJBNVOtF0ksg10HuOGowIfqGNSexwvDzUai3jWIX1ItmncX',
  baseURL: 'https://tokenhub.tencentmaas.com/v1',
});

const response = await client.chat.completions.create({
  model: 'hy3-preview',
  messages: [
    { role: 'user', content: '你好，请介绍一下你自己' },
  ],
});
console.log(response.choices[0].message.content);


