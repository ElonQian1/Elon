import { AttachmentPreviewDialog } from './AttachmentPreviewDialog'
import ImageAnnotationEditor from './ImageAnnotationEditor'
import type { UploadedAttachment } from './AttachmentButton'
import type { ComposerImageEditItem } from './useComposerAttachments'

interface ComposerAttachmentDialogsProps {
  attachmentPreview: UploadedAttachment | null
  imageEditItem: ComposerImageEditItem | null
  imageEditQueueCount: number
  attachmentUploading: boolean
  attachmentError: string
  onCloseAttachmentPreview: () => void
  onApplyImageEdit: (itemId: string, file: File) => Promise<void>
  onSendOriginalImage: (itemId: string) => Promise<void>
  onDiscardImageEdit: (itemId: string) => void
}

export default function ComposerAttachmentDialogs({
  attachmentPreview,
  imageEditItem,
  imageEditQueueCount,
  attachmentUploading,
  attachmentError,
  onCloseAttachmentPreview,
  onApplyImageEdit,
  onSendOriginalImage,
  onDiscardImageEdit,
}: ComposerAttachmentDialogsProps) {
  return (
    <>
      {attachmentPreview && <AttachmentPreviewDialog attachment={attachmentPreview} onClose={onCloseAttachmentPreview} />}
      {imageEditItem && (
        <ImageAnnotationEditor
          key={imageEditItem.id}
          file={imageEditItem.file}
          queueIndex={0}
          queueCount={imageEditQueueCount}
          uploading={attachmentUploading}
          error={attachmentError}
          onApply={(file) => onApplyImageEdit(imageEditItem.id, file)}
          onSendOriginal={() => onSendOriginalImage(imageEditItem.id)}
          onDiscard={() => onDiscardImageEdit(imageEditItem.id)}
        />
      )}
    </>
  )
}
