/**
 * P1.4：消息附件上传组件
 *
 * 流程：
 *   1. 用户点击回形针按钮 → 触发 <input type="file">
 *   2. ConversationPage 统一处理点击选择、粘贴、拖拽上传
 *   3. 上传成功后在 composer 区域显示预览 chip
 *   4. 发送消息时把附件作为结构化 attachments 传给服务端
 */
import { useRef } from 'react'
import { FileText, Loader2, Paperclip, X } from 'lucide-react'
import styles from './AttachmentButton.module.css'
import { getAuthToken } from '../../api/client'

export interface UploadedAttachment {
  attachment_id: string
  kind: string
  display_name: string
  file_name?: string
  url: string
  urls?: string[]
  path?: string
  sha256?: string
  mime_type: string
  size_bytes: number
  image_width?: number
  image_height?: number
}

interface Props {
  disabled?: boolean
  uploading?: boolean
  onFilesSelected: (files: File[]) => void
}

export const MAX_ATTACHMENT_SIZE_MB = 12
export const MAX_ATTACHMENTS_PER_MESSAGE = 6

interface UploadProjectAttachmentOptions {
  conversationId?: string | null
  conversationTitle?: string | null
}

interface UploadProjectAttachmentResponse {
  attachment?: UploadedAttachment
  status?: string
}

export async function uploadProjectAttachment(
  projectId: string,
  file: File,
  options: UploadProjectAttachmentOptions = {},
): Promise<UploadedAttachment> {
  if (file.size > MAX_ATTACHMENT_SIZE_MB * 1024 * 1024) {
    throw new Error(`文件最大 ${MAX_ATTACHMENT_SIZE_MB} MB`)
  }
  const mime = file.type || 'application/octet-stream'
  const kind = mime.startsWith('image/') ? 'image' : 'attachment'
  const params = new URLSearchParams({
    file_name: file.name,
    display_name: file.name,
    mime_type: mime,
    kind,
  })
  if (options.conversationId) params.set('conversation_id', options.conversationId)
  if (options.conversationTitle) params.set('conversation_title', options.conversationTitle)

  const token = getAuthToken()
  const res = await fetch(`/api/projects/${encodeURIComponent(projectId)}/attachments?${params.toString()}`, {
    method: 'POST',
    headers: {
      'Content-Type': mime,
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: await file.arrayBuffer(),
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({})) as Record<string, unknown>
    throw new Error(String(err.error ?? err.message ?? `HTTP ${res.status}`))
  }
  const data = await res.json() as UploadProjectAttachmentResponse | UploadedAttachment
  const attachment = isUploadedAttachment(data)
    ? data
    : isUploadedAttachment(data.attachment)
      ? data.attachment
      : null
  if (!attachment) {
    throw new Error('附件上传响应缺少文件信息')
  }
  return attachment
}

function isUploadedAttachment(value: unknown): value is UploadedAttachment {
  if (!value || typeof value !== 'object') return false
  const record = value as Partial<UploadedAttachment>
  return typeof record.attachment_id === 'string' && typeof record.url === 'string'
}

export function AttachmentButton({ disabled, uploading, onFilesSelected }: Props) {
  const inputRef = useRef<HTMLInputElement>(null)

  function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const files = Array.from(e.target.files ?? [])
    if (inputRef.current) inputRef.current.value = ''
    if (files.length) onFilesSelected(files)
  }

  return (
    <div className={styles.wrap}>
      <input
        ref={inputRef}
        type="file"
        className={styles.hiddenInput}
        accept="image/*,.pdf,.txt,.md,.json,.csv,.zip"
        multiple
        onChange={handleFileChange}
        disabled={disabled || uploading}
      />
      <button
        className={styles.btn}
        data-uploading={uploading ? 'true' : 'false'}
        type="button"
        title={uploading ? '上传中…' : '添加附件'}
        disabled={disabled || uploading}
        onClick={() => inputRef.current?.click()}
      >
        {uploading ? <Loader2 size={16} aria-hidden="true" /> : <Paperclip size={16} aria-hidden="true" />}
      </button>
    </div>
  )
}

/** 已上传附件的预览 chip，可点击删除 */
interface ChipProps {
  attachment: UploadedAttachment
  onRemove: () => void
}

export function AttachmentChip({ attachment, onRemove }: ChipProps) {
  const isImage = attachment.kind === 'image' || attachment.mime_type?.startsWith('image/')
  const sizeKB = Math.round((attachment.size_bytes ?? 0) / 1024)
  return (
    <div className={styles.chip}>
      {isImage && <img src={attachment.url} alt={attachment.display_name} className={styles.chipThumb} />}
      {!isImage && <FileText className={styles.chipIcon} size={14} aria-hidden="true" />}
      <span className={styles.chipName}>{attachment.display_name}</span>
      <span className={styles.chipSize}>{sizeKB} KB</span>
      <button className={styles.chipRemove} onClick={onRemove} type="button" title="删除">
        <X size={12} aria-hidden="true" />
      </button>
    </div>
  )
}

/** 把附件列表转为追加到消息末尾的 markdown 文本 */
export function attachmentsToMarkdown(attachments: UploadedAttachment[]): string {
  return attachments
    .map((att) => {
      const isImage = att.kind === 'image' || att.mime_type?.startsWith('image/')
      return isImage
        ? `\n![${att.display_name}](${att.url})`
        : `\n[${att.display_name}](${att.url})`
    })
    .join('')
}
