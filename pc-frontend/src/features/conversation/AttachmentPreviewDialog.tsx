import { useEffect } from 'react'
import { ExternalLink, FileText, X } from 'lucide-react'
import { isImageAttachment, type UploadedAttachment } from './AttachmentButton'
import styles from './ConversationPage.module.css'

interface AttachmentPreviewDialogProps {
  attachment: UploadedAttachment
  onClose: () => void
}

function formatAttachmentPreviewSize(sizeBytes?: number): string {
  const bytes = sizeBytes ?? 0
  if (bytes <= 0) return '0 KB'
  if (bytes >= 1024 * 1024) {
    const mb = bytes / (1024 * 1024)
    return `${mb >= 10 ? Math.round(mb) : mb.toFixed(1)} MB`
  }
  return `${Math.max(1, Math.round(bytes / 1024))} KB`
}

export function AttachmentPreviewDialog({ attachment, onClose }: AttachmentPreviewDialogProps) {
  const attachmentPreviewIsImage = isImageAttachment(attachment)
  const attachmentPreviewSize = formatAttachmentPreviewSize(attachment.size_bytes)

  useEffect(() => {
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', closeOnEscape)
    return () => window.removeEventListener('keydown', closeOnEscape)
  }, [onClose])

  return (
    <div
      className={styles.attachmentPreviewBackdrop}
      role="presentation"
      onMouseDown={(event) => {
        if (event.currentTarget === event.target) onClose()
      }}
    >
      <section
        className={styles.attachmentPreviewDialog}
        role="dialog"
        aria-modal="true"
        aria-label="附件预览"
      >
        <header className={styles.attachmentPreviewHeader}>
          <div className={styles.attachmentPreviewTitle}>
            <strong title={attachment.display_name}>{attachment.display_name}</strong>
            <span>{attachment.mime_type || '附件'} · {attachmentPreviewSize}</span>
          </div>
          <div className={styles.attachmentPreviewActions}>
            <a
              className={styles.attachmentPreviewIconBtn}
              href={attachment.url}
              target="_blank"
              rel="noreferrer"
              title="在新标签打开"
              aria-label="在新标签打开附件"
            >
              <ExternalLink size={16} aria-hidden="true" />
            </a>
            <button
              className={styles.attachmentPreviewIconBtn}
              onClick={onClose}
              type="button"
              title="关闭"
              aria-label="关闭附件预览"
            >
              <X size={17} aria-hidden="true" />
            </button>
          </div>
        </header>
        <div className={styles.attachmentPreviewBody}>
          {attachmentPreviewIsImage ? (
            <img
              className={styles.attachmentPreviewImage}
              src={attachment.url}
              alt={attachment.display_name}
            />
          ) : (
            <div className={styles.attachmentPreviewFile}>
              <FileText size={34} aria-hidden="true" />
              <strong>{attachment.display_name}</strong>
              <span>{attachment.mime_type || '附件'} · {attachmentPreviewSize}</span>
              <a href={attachment.url} target="_blank" rel="noreferrer">打开文件</a>
            </div>
          )}
        </div>
      </section>
    </div>
  )
}
