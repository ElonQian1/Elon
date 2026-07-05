import { useCallback, useState, type Dispatch, type SetStateAction } from 'react'
import type { UploadedAttachment } from './AttachmentButton'

export function useAttachmentPreview(
  setAttachments: Dispatch<SetStateAction<UploadedAttachment[]>>,
) {
  const [attachmentPreview, setAttachmentPreview] = useState<UploadedAttachment | null>(null)

  const closeAttachmentPreview = useCallback(() => {
    setAttachmentPreview(null)
  }, [])

  const removeComposerAttachment = useCallback((attachmentId: string) => {
    setAttachments((prev) => prev.filter((attachment) => attachment.attachment_id !== attachmentId))
    setAttachmentPreview((current) => current?.attachment_id === attachmentId ? null : current)
  }, [setAttachments])

  return {
    attachmentPreview,
    openAttachmentPreview: setAttachmentPreview,
    closeAttachmentPreview,
    removeComposerAttachment,
  }
}
