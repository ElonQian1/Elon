import type { UploadedAttachment } from '../conversation/AttachmentButton'
import type { MemberConversationTarget } from '../conversation/memberConversationApi'

const PROJECT_COMPOSER_DRAFT_KEY = 'elon.pc.projectComposerDraft.v1'
const AI_COMPOSER_DRAFT_KEY = 'elon.pc.aiComposerDraft.v1'
const MAX_DRAFT_AGE_MS = 24 * 60 * 60 * 1000

export interface ProjectComposerDraft {
  userId?: string
  input: string
  attachments: UploadedAttachment[]
  draftConversationId: string
  activeProjectId: string
  activeChannelId: string
  sessionView: string | 'new' | null
  conversationTarget: MemberConversationTarget | null
  updatedAt: number
}

export interface AiComposerDraft {
  userId?: string
  input: string
  activeConvId: string
  updatedAt: number
}

export function readProjectComposerDraft(userId?: string): ProjectComposerDraft | null {
  const draft = readDraft<ProjectComposerDraft>(PROJECT_COMPOSER_DRAFT_KEY)
  if (!draft) return null
  if (userId && draft.userId && draft.userId !== userId) return null
  return draft
}

export function saveProjectComposerDraft(draft: Omit<ProjectComposerDraft, 'updatedAt'>) {
  writeDraft(PROJECT_COMPOSER_DRAFT_KEY, { ...draft, updatedAt: Date.now() })
}

export function clearProjectComposerDraft() {
  removeDraft(PROJECT_COMPOSER_DRAFT_KEY)
}

export function readAiComposerDraft(userId?: string): AiComposerDraft | null {
  const draft = readDraft<AiComposerDraft>(AI_COMPOSER_DRAFT_KEY)
  if (!draft) return null
  if (userId && draft.userId && draft.userId !== userId) return null
  return draft
}

export function saveAiComposerDraft(draft: Omit<AiComposerDraft, 'updatedAt'>) {
  writeDraft(AI_COMPOSER_DRAFT_KEY, { ...draft, updatedAt: Date.now() })
}

export function clearAiComposerDraft() {
  removeDraft(AI_COMPOSER_DRAFT_KEY)
}

function readDraft<T extends { updatedAt?: number }>(key: string): T | null {
  try {
    const raw = safeSessionStorage()?.getItem(key)
    if (!raw) return null
    const draft = JSON.parse(raw) as T
    if (Date.now() - Number(draft.updatedAt || 0) > MAX_DRAFT_AGE_MS) {
      removeDraft(key)
      return null
    }
    return draft
  } catch {
    return null
  }
}

function writeDraft<T>(key: string, draft: T) {
  try {
    safeSessionStorage()?.setItem(key, JSON.stringify(draft))
  } catch {
    // Storage can be disabled or full. Draft persistence is best effort.
  }
}

function removeDraft(key: string) {
  try {
    safeSessionStorage()?.removeItem(key)
  } catch {
    // ignore
  }
}

function safeSessionStorage(): Storage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.sessionStorage
  } catch {
    return null
  }
}
