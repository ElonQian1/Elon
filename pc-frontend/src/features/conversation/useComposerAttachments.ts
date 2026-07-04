import { useCallback, useState } from 'react'
import type { ClipboardEvent, DragEvent, Dispatch, SetStateAction } from 'react'
import { v4 as uuidv4 } from 'uuid'
import {
  MAX_ATTACHMENTS_PER_MESSAGE,
  attachmentsToMarkdown,
  uploadProjectAttachment,
} from './AttachmentButton'
import type { UploadedAttachment } from './AttachmentButton'

interface UseComposerAttachmentsParams {
  activeProjectId: string
  composerDisabled: boolean
  sessionView: string | 'new' | null
}

interface RestoreAttachmentDraft {
  attachments: UploadedAttachment[]
  draftConversationId: string
}

export function useComposerAttachments({
  activeProjectId,
  composerDisabled,
  sessionView,
}: UseComposerAttachmentsParams) {
  const [attachments, setAttachments] = useState<UploadedAttachment[]>([])
  const [attachmentUploading, setAttachmentUploading] = useState(false)
  const [attachmentDropActive, setAttachmentDropActive] = useState(false)
  const [attachmentError, setAttachmentError] = useState('')
  const [draftConversationId, setDraftConversationId] = useState('')

  const attachmentConversationId = useCallback(() => {
    if (typeof sessionView === 'string' && sessionView !== 'new') return sessionView
    const next = draftConversationId || uuidv4()
    if (!draftConversationId) setDraftConversationId(next)
    return next
  }, [draftConversationId, sessionView])

  const uploadComposerFiles = useCallback(async (incomingFiles: File[] | FileList) => {
    if (!activeProjectId || composerDisabled || attachmentUploading) return
    const files = Array.from(incomingFiles).filter((file) => file.size > 0)
    if (files.length === 0) return

    const openSlots = MAX_ATTACHMENTS_PER_MESSAGE - attachments.length
    if (openSlots <= 0) {
      setAttachmentError(`一次最多添加 ${MAX_ATTACHMENTS_PER_MESSAGE} 个附件`)
      return
    }

    const selectedFiles = files.slice(0, openSlots)
    setAttachmentError(files.length > openSlots ? `已添加前 ${openSlots} 个附件` : '')
    setAttachmentUploading(true)
    try {
      const conversationId = attachmentConversationId()
      for (const file of selectedFiles) {
        const uploaded = await uploadProjectAttachment(activeProjectId, file, { conversationId })
        setAttachments((prev) => {
          if (prev.some((item) => item.attachment_id === uploaded.attachment_id)) return prev
          if (prev.length >= MAX_ATTACHMENTS_PER_MESSAGE) return prev
          return [...prev, uploaded]
        })
      }
    } catch (err) {
      setAttachmentError((err as { message?: string }).message ?? '附件上传失败')
    } finally {
      setAttachmentUploading(false)
    }
  }, [activeProjectId, attachmentConversationId, attachmentUploading, attachments.length, composerDisabled])

  function handleComposerPaste(e: ClipboardEvent<HTMLTextAreaElement>) {
    const files = Array.from(e.clipboardData.files ?? [])
    if (files.length === 0) return
    e.preventDefault()
    uploadComposerFiles(files).catch(() => {})
  }

  function handleComposerDragEnter(e: DragEvent<HTMLFormElement>) {
    if (!dataTransferHasFiles(e.dataTransfer) || composerDisabled) return
    e.preventDefault()
    setAttachmentDropActive(true)
  }

  function handleComposerDragOver(e: DragEvent<HTMLFormElement>) {
    if (!dataTransferHasFiles(e.dataTransfer) || composerDisabled) return
    e.preventDefault()
    e.dataTransfer.dropEffect = 'copy'
    setAttachmentDropActive(true)
  }

  function handleComposerDragLeave(e: DragEvent<HTMLFormElement>) {
    const nextTarget = e.relatedTarget
    if (nextTarget instanceof Node && e.currentTarget.contains(nextTarget)) return
    setAttachmentDropActive(false)
  }

  function handleComposerDrop(e: DragEvent<HTMLFormElement>) {
    if (!dataTransferHasFiles(e.dataTransfer) || composerDisabled) return
    e.preventDefault()
    setAttachmentDropActive(false)
    uploadComposerFiles(e.dataTransfer.files).catch(() => {})
  }

  const clearAttachmentDraft = useCallback(() => {
    setAttachments([])
    setAttachmentError('')
    setDraftConversationId('')
  }, [])

  const restoreAttachmentDraft = useCallback((draft: RestoreAttachmentDraft) => {
    setAttachments(draft.attachments)
    setDraftConversationId(draft.draftConversationId)
  }, [])

  return {
    attachments,
    setAttachments,
    attachmentUploading,
    attachmentDropActive,
    attachmentError,
    draftConversationId,
    clearAttachmentDraft,
    restoreAttachmentDraft,
    uploadComposerFiles,
    handleComposerPaste,
    handleComposerDragEnter,
    handleComposerDragOver,
    handleComposerDragLeave,
    handleComposerDrop,
  } satisfies {
    attachments: UploadedAttachment[]
    setAttachments: Dispatch<SetStateAction<UploadedAttachment[]>>
    attachmentUploading: boolean
    attachmentDropActive: boolean
    attachmentError: string
    draftConversationId: string
    clearAttachmentDraft: () => void
    restoreAttachmentDraft: (draft: RestoreAttachmentDraft) => void
    uploadComposerFiles: (files: File[] | FileList) => Promise<void>
    handleComposerPaste: (e: ClipboardEvent<HTMLTextAreaElement>) => void
    handleComposerDragEnter: (e: DragEvent<HTMLFormElement>) => void
    handleComposerDragOver: (e: DragEvent<HTMLFormElement>) => void
    handleComposerDragLeave: (e: DragEvent<HTMLFormElement>) => void
    handleComposerDrop: (e: DragEvent<HTMLFormElement>) => void
  }
}

export function buildComposerContent(text: string, attachments: UploadedAttachment[]): string {
  if (attachments.length === 0) return text
  const markdown = attachmentsToMarkdown(attachments).trimStart()
  return text ? `${text}\n\n${markdown}` : markdown
}

export function attachmentTitleFromAttachments(attachments: UploadedAttachment[]): string {
  const first = attachments[0]?.display_name?.trim()
  if (!first) return '附件会话'
  if (attachments.length === 1) return first
  return `${first} 等 ${attachments.length} 个附件`
}

function dataTransferHasFiles(dataTransfer: DataTransfer): boolean {
  if (dataTransfer.files.length > 0) return true
  return Array.from(dataTransfer.types ?? []).includes('Files')
}
