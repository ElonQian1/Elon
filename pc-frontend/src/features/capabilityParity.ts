export type CapabilityParityStatus = 'app_shared' | 'app_baseline' | 'web_specialized' | 'needs_contract'

export interface CapabilityParityEntry {
  id: string
  label: string
  status: CapabilityParityStatus
  webRoute?: string
  appSurface: string
  detail: string
}

/**
 * APP is the source of truth. This registry is intentionally explicit so a
 * desktop-only UI cannot be mistaken for a second implementation of an APP
 * capability.
 */
export const CAPABILITY_PARITY: CapabilityParityEntry[] = [
  {
    id: 'provider-native-ai',
    label: '官方 AI 网页会话',
    status: 'app_shared',
    webRoute: '/user-browser',
    appSurface: 'APP ChatGPT / Google AI 同源适配器',
    detail: '网页与 APP 共用适配器资产、事件协议、能力清单和 UI 控制语义。',
  },
  {
    id: 'project-ai',
    label: '项目 AI 任务',
    status: 'app_shared',
    webRoute: '/workspace',
    appSurface: 'APP 项目任务链与 ai_development 频道',
    detail: '网页复用服务端任务创建、实时事件、trace_id 和恢复快照；界面仍是桌面布局。',
  },
  {
    id: 'platform-ai',
    label: '平台 AI 聊天',
    status: 'needs_contract',
    webRoute: '/ai',
    appSurface: 'APP ElonServerAIClient：WS → 项目 HTTP → /api/llm/chat',
    detail: '网页 /ai 当前直接进入 /api/llm/chat/stream 的首页工具编排；APP 先走气球项目 WebSocket，再走项目 HTTP chat_only，最后才以 /api/llm/chat 兜底。两端入口、上下文和降级顺序仍不同，这是当前 AI 体验差异的核心风险。',
  },
  {
    id: 'realtime-voice',
    label: '官方实时语音',
    status: 'app_baseline',
    webRoute: '/user-browser/native',
    appSurface: 'APP 官方网页适配器 voice_mode 控件',
    detail: '网页只负责桌面呈现和控制转发，能力判断与页面动作来自 APP 同源适配器。',
  },
  {
    id: 'chatkit',
    label: 'OpenAI ChatKit',
    status: 'app_shared',
    webRoute: '/chatkit',
    appSurface: 'APP ChatKit Activity / 账号入口',
    detail: '这是 APP 与网页都支持的独立官方 API 聊天面，不替代项目 AI 或官方网页会话。',
  },
  {
    id: 'tts-console',
    label: 'TTS 声音控制台',
    status: 'web_specialized',
    webRoute: '/voice',
    appSurface: 'APP 仅提供聊天内语音输入、播放和开关',
    detail: '网页控制台用于声线、情绪和 Worker 管理；它不是 APP 核心 AI 的另一套实现。',
  },
  {
    id: 'ai-work-summary',
    label: 'AI 工作摘要',
    status: 'app_shared',
    webRoute: '/ai-work-summary',
    appSurface: 'APP AiWorkSummaryActivity',
    detail: '网页已同步 APP 当前 Activity 的页面结构、摘要卡片和操作语义；APP 当前数据仍是本地静态基线，动态数据契约尚未共享。',
  },
]

export const capabilityParityStatusLabel: Record<CapabilityParityStatus, string> = {
  app_shared: 'APP 同源',
  app_baseline: 'APP 基线同步',
  web_specialized: '网页专属',
  needs_contract: '待补契约',
}
