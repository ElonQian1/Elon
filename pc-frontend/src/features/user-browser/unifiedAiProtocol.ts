/** 一龙原生 AI UI 与厂商适配器之间的去凭证化语义协议。 */
export const UNIFIED_AI_PROTOCOL = 'yilong.ai.ui.v1' as const

export type UnifiedAiSource = 'official_web' | 'official_api' | 'cli'
export type UnifiedAiRole = 'user' | 'assistant' | 'system' | 'tool'
export type UnifiedAiMessageState = 'pending' | 'streaming' | 'completed' | 'failed'

export interface UnifiedAiTextPart {
  type: 'text'
  text: string
}

export interface UnifiedAiImagePart {
  type: 'image'
  /** 本地对象引用或厂商公开资源引用；适配器不得在这里放鉴权参数。 */
  ref: string
  alt?: string
}

export interface UnifiedAiFilePart {
  type: 'file'
  ref: string
  name: string
  mediaType?: string
}

export interface UnifiedAiCitationPart {
  type: 'citation'
  title?: string
  url: string
}

export type UnifiedAiContentPart =
  | UnifiedAiTextPart
  | UnifiedAiImagePart
  | UnifiedAiFilePart
  | UnifiedAiCitationPart

export interface UnifiedAiMessage {
  id: string
  role: UnifiedAiRole
  state: UnifiedAiMessageState
  content: UnifiedAiContentPart[]
  createdAt?: string
}

export type UnifiedAiCapability =
  | 'streaming'
  | 'attachments'
  | 'citations'
  | 'tools'
  | 'voice'
  | 'conversation_history'

export type UnifiedAiEvent =
  | {
      type: 'adapter_ready'
      capabilities: UnifiedAiCapability[]
    }
  | {
      type: 'conversation_changed'
      conversationId: string
      title?: string
    }
  | {
      type: 'message_snapshot'
      messages: UnifiedAiMessage[]
    }
  | {
      type: 'message_delta'
      messageId: string
      text: string
    }
  | {
      type: 'status_changed'
      status: 'idle' | 'thinking' | 'streaming' | 'waiting_for_user' | 'error'
      detail?: string
    }

export interface UnifiedAiEnvelope {
  schema: typeof UNIFIED_AI_PROTOCOL
  providerId: string
  source: UnifiedAiSource
  conversationId?: string
  sequence: number
  emittedAt: string
  event: UnifiedAiEvent
}

/**
 * 协议只表达用户可见语义。厂商适配器必须在发出 envelope 前丢弃浏览器凭证、
 * 请求头和原始网络响应；首版 WebView2 宿主尚未启用适配器桥。
 */
export function isUnifiedAiEnvelope(value: unknown): value is UnifiedAiEnvelope {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const candidate = value as Partial<UnifiedAiEnvelope>
  return candidate.schema === UNIFIED_AI_PROTOCOL
    && typeof candidate.providerId === 'string'
    && typeof candidate.sequence === 'number'
    && typeof candidate.emittedAt === 'string'
    && Boolean(candidate.event && typeof candidate.event === 'object')
}
