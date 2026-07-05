import { useCallback, useEffect, useRef } from 'react'
import type { Dispatch, MutableRefObject, SetStateAction } from 'react'
import { APP_UPDATE_BEFORE_RELOAD_EVENT } from '../updates/appUpdateSession'
import {
  clearProjectComposerDraft,
  readProjectComposerDraft,
  saveProjectComposerDraft,
  type ProjectComposerDraft,
} from '../updates/composerDrafts'
import type { UploadedAttachment } from './AttachmentButton'
import type { MemberConversationTarget } from './memberConversationApi'
import { sameConversationTarget } from './memberConversationApi'
import type { Message } from './types'

interface UseProjectComposerDraftPersistenceParams {
  userId?: string
  input: string
  setInput: Dispatch<SetStateAction<string>>
  attachments: UploadedAttachment[]
  draftConversationId: string
  activeProjectId: string
  activeChannelId: string
  sessionView: string | 'new' | null
  setSessionView: Dispatch<SetStateAction<string | 'new' | null>>
  activeConversationTarget: MemberConversationTarget | null
  isOwnConversationTarget: boolean
  setMemberConversationTarget: Dispatch<SetStateAction<MemberConversationTarget | null>>
  restoreAttachmentDraft: (draft: { attachments: UploadedAttachment[]; draftConversationId: string }) => void
  openConversation: (conversationId: string, options?: { force?: boolean }) => void | Promise<void>
  autoResize: () => void
  conversationLoadSeqRef: MutableRefObject<number>
  waitingForNewSession: MutableRefObject<boolean>
  setConvMessages: Dispatch<SetStateAction<Message[]>>
  setSessionTaskMessages: Dispatch<SetStateAction<Message[]>>
}

export function useProjectComposerDraftPersistence({
  userId,
  input,
  setInput,
  attachments,
  draftConversationId,
  activeProjectId,
  activeChannelId,
  sessionView,
  setSessionView,
  activeConversationTarget,
  isOwnConversationTarget,
  setMemberConversationTarget,
  restoreAttachmentDraft,
  openConversation,
  autoResize,
  conversationLoadSeqRef,
  waitingForNewSession,
  setConvMessages,
  setSessionTaskMessages,
}: UseProjectComposerDraftPersistenceParams) {
  const pendingDraftRef = useRef<ProjectComposerDraft | null>(readProjectComposerDraft())
  const draftReadyRef = useRef(false)

  const persistDraft = useCallback(() => {
    if (!activeProjectId && !input && attachments.length === 0) return
    saveProjectComposerDraft({
      userId,
      input,
      attachments,
      draftConversationId,
      activeProjectId,
      activeChannelId,
      sessionView,
      conversationTarget: isOwnConversationTarget ? null : activeConversationTarget,
    })
  }, [
    activeChannelId,
    activeConversationTarget,
    activeProjectId,
    attachments,
    draftConversationId,
    input,
    isOwnConversationTarget,
    sessionView,
    userId,
  ])

  useEffect(() => {
    if (draftReadyRef.current) return
    const draft = pendingDraftRef.current
    if (!draft) {
      draftReadyRef.current = true
      return
    }
    if (draft.userId && userId && draft.userId !== userId) {
      pendingDraftRef.current = null
      draftReadyRef.current = true
      clearProjectComposerDraft()
      return
    }
    if (draft.activeProjectId && activeProjectId !== draft.activeProjectId) return
    if (draft.conversationTarget && !sameConversationTarget(draft.conversationTarget, activeConversationTarget)) {
      setMemberConversationTarget(draft.conversationTarget)
      return
    }

    setInput(draft.input ?? '')
    restoreAttachmentDraft({
      attachments: draft.attachments ?? [],
      draftConversationId: draft.draftConversationId ?? '',
    })
    if (draft.sessionView === 'new') {
      conversationLoadSeqRef.current += 1
      setSessionView('new')
      setConvMessages([])
      setSessionTaskMessages([])
      waitingForNewSession.current = true
    } else if (draft.sessionView) {
      void openConversation(String(draft.sessionView), { force: true })
    } else {
      setSessionView(null)
    }
    pendingDraftRef.current = null
    draftReadyRef.current = true
    window.setTimeout(autoResize, 0)
  }, [
    activeConversationTarget,
    activeProjectId,
    autoResize,
    conversationLoadSeqRef,
    openConversation,
    restoreAttachmentDraft,
    setConvMessages,
    setInput,
    setMemberConversationTarget,
    setSessionTaskMessages,
    setSessionView,
    userId,
    waitingForNewSession,
  ])

  useEffect(() => {
    if (!draftReadyRef.current) return
    persistDraft()
  }, [persistDraft])

  useEffect(() => {
    function saveBeforeReload() {
      persistDraft()
    }
    window.addEventListener(APP_UPDATE_BEFORE_RELOAD_EVENT, saveBeforeReload)
    return () => window.removeEventListener(APP_UPDATE_BEFORE_RELOAD_EVENT, saveBeforeReload)
  }, [persistDraft])

  return {
    clearSavedComposerDraft: clearProjectComposerDraft,
  }
}
