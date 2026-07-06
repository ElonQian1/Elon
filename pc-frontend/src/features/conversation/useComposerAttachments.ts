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

export interface ComposerImageEditItem {
  id: string
  file: File
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
  const [imageEditQueue, setImageEditQueue] = useState<ComposerImageEditItem[]>([])

  const attachmentConversationId = useCallback(() => {
    if (typeof sessionView === 'string' && sessionView !== 'new') return sessionView
    const next = draftConversationId || uuidv4()
    if (!draftConversationId) setDraftConversationId(next)
    return next
  }, [draftConversationId, sessionView])

  const addUploadedAttachment = useCallback((uploaded: UploadedAttachment) => {
    setAttachments((prev) => {
      if (prev.some((item) => item.attachment_id === uploaded.attachment_id)) return prev
      if (prev.length >= MAX_ATTACHMENTS_PER_MESSAGE) return prev
      return [...prev, uploaded]
    })
  }, [])

  const uploadDirectFiles = useCallback(async (files: File[]) => {
    if (files.length === 0) return
    setAttachmentUploading(true)
    try {
      const conversationId = attachmentConversationId()
      for (const file of files) {
        const uploaded = await uploadProjectAttachment(activeProjectId, file, { conversationId })
        addUploadedAttachment(uploaded)
      }
    } catch (err) {
      setAttachmentError((err as { message?: string }).message ?? '附件上传失败')
    } finally {
      setAttachmentUploading(false)
    }
  }, [activeProjectId, addUploadedAttachment, attachmentConversationId])

  const uploadComposerFiles = useCallback(async (incomingFiles: File[] | FileList) => {
    if (!activeProjectId || composerDisabled || attachmentUploading) return
    const files = Array.from(incomingFiles).filter((file) => file.size > 0)
    if (files.length === 0) return

    const openSlots = MAX_ATTACHMENTS_PER_MESSAGE - attachments.length - imageEditQueue.length
    if (openSlots <= 0) {
      setAttachmentError(`一次最多添加 ${MAX_ATTACHMENTS_PER_MESSAGE} 个附件`)
      return
    }

    const selectedFiles = files.slice(0, openSlots)
    setAttachmentError(files.length > openSlots ? `已添加前 ${openSlots} 个附件` : '')
    const imageFiles = selectedFiles.filter(isEditableImageFile)
    const directFiles = selectedFiles.filter((file) => !isEditableImageFile(file))

    if (imageFiles.length > 0) {
      setImageEditQueue((prev) => [
        ...prev,
        ...imageFiles.map((file) => ({ id: uuidv4(), file })),
      ])
    }

    await uploadDirectFiles(directFiles)
  }, [
    activeProjectId,
    attachmentUploading,
    attachments.length,
    composerDisabled,
    imageEditQueue.length,
    uploadDirectFiles,
  ])

  const uploadImageEditFile = useCallback(async (itemId: string, file: File) => {
    if (!activeProjectId || composerDisabled || attachmentUploading) return
    if (attachments.length >= MAX_ATTACHMENTS_PER_MESSAGE) {
      setAttachmentError(`一次最多添加 ${MAX_ATTACHMENTS_PER_MESSAGE} 个附件`)
      return
    }
    setAttachmentUploading(true)
    try {
      const conversationId = attachmentConversationId()
      const uploaded = await uploadProjectAttachment(activeProjectId, file, { conversationId })
      addUploadedAttachment(uploaded)
      setImageEditQueue((prev) => prev.filter((item) => item.id !== itemId))
      setAttachmentError('')
    } catch (err) {
      setAttachmentError((err as { message?: string }).message ?? '图片上传失败')
    } finally {
      setAttachmentUploading(false)
    }
  }, [
    activeProjectId,
    addUploadedAttachment,
    attachmentConversationId,
    attachmentUploading,
    attachments.length,
    composerDisabled,
  ])

  const uploadEditedImage = useCallback(async (itemId: string, file: File) => {
    await uploadImageEditFile(itemId, file)
  }, [uploadImageEditFile])

  const uploadOriginalImage = useCallback(async (itemId: string) => {
    const item = imageEditQueue.find((candidate) => candidate.id === itemId)
    if (!item) return
    await uploadImageEditFile(itemId, item.file)
  }, [imageEditQueue, uploadImageEditFile])

  const discardImageEdit = useCallback((itemId: string) => {
    setImageEditQueue((prev) => prev.filter((item) => item.id !== itemId))
  }, [])

  function handleComposerPaste(e: ClipboardEvent<HTMLElement>) {
    if (e.defaultPrevented) return
    const files = Array.from(e.clipboardData.files ?? [])
    if (files.length === 0) return
    e.preventDefault()
    uploadComposerFiles(files).catch(() => {})
  }

  function handleComposerDragEnter(e: DragEvent<HTMLElement>) {
    if (!dataTransferHasFiles(e.dataTransfer) || composerDisabled) return
    e.preventDefault()
    setAttachmentDropActive(true)
  }

  function handleComposerDragOver(e: DragEvent<HTMLElement>) {
    if (!dataTransferHasFiles(e.dataTransfer) || composerDisabled) return
    e.preventDefault()
    e.dataTransfer.dropEffect = 'copy'
    setAttachmentDropActive(true)
  }

  function handleComposerDragLeave(e: DragEvent<HTMLElement>) {
    const nextTarget = e.relatedTarget
    if (nextTarget instanceof Node && e.currentTarget.contains(nextTarget)) return
    setAttachmentDropActive(false)
  }

  function handleComposerDrop(e: DragEvent<HTMLElement>) {
    if (!dataTransferHasFiles(e.dataTransfer) || composerDisabled) return
    e.preventDefault()
    setAttachmentDropActive(false)
    uploadComposerFiles(e.dataTransfer.files).catch(() => {})
  }

  const clearAttachmentDraft = useCallback(() => {
    setAttachments([])
    setAttachmentError('')
    setDraftConversationId('')
    setImageEditQueue([])
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
    imageEditItem: imageEditQueue[0] ?? null,
    imageEditQueueCount: imageEditQueue.length,
    draftConversationId,
    clearAttachmentDraft,
    restoreAttachmentDraft,
    uploadComposerFiles,
    uploadEditedImage,
    uploadOriginalImage,
    discardImageEdit,
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
    imageEditItem: ComposerImageEditItem | null
    imageEditQueueCount: number
    draftConversationId: string
    clearAttachmentDraft: () => void
    restoreAttachmentDraft: (draft: RestoreAttachmentDraft) => void
    uploadComposerFiles: (files: File[] | FileList) => Promise<void>
    uploadEditedImage: (itemId: string, file: File) => Promise<void>
    uploadOriginalImage: (itemId: string) => Promise<void>
    discardImageEdit: (itemId: string) => void
    handleComposerPaste: (e: ClipboardEvent<HTMLElement>) => void
    handleComposerDragEnter: (e: DragEvent<HTMLElement>) => void
    handleComposerDragOver: (e: DragEvent<HTMLElement>) => void
    handleComposerDragLeave: (e: DragEvent<HTMLElement>) => void
    handleComposerDrop: (e: DragEvent<HTMLElement>) => void
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

function isEditableImageFile(file: File): boolean {
  const mime = file.type.toLowerCase()
  if (['image/jpeg', 'image/jpg', 'image/png', 'image/webp', 'image/bmp'].includes(mime)) return true
  return /\.(jpe?g|png|webp|bmp)$/i.test(file.name)
}
