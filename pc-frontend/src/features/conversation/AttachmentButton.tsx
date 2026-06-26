/**
 * P1.4：消息附件上传组件
 *
 * 流程：
 *   1. 用户点击回形针按钮 → 触发 <input type="file">
 *   2. 选择文件后立即上传到 /api/projects/{id}/attachments
 *   3. 上传成功后在 composer 区域显示预览 chip
 *   4. 发送消息时把附件作为 markdown 图片/链接追加到 content
 */
import { useRef, useState } from 'react'
import styles from './AttachmentButton.module.css'
import { getAuthToken } from '../../api/client'

export interface UploadedAttachment {
  attachment_id: string
  kind: string
  display_name: string
  url: string
  mime_type: string
  size_bytes: number
}

interface Props {
  projectId: string
  disabled?: boolean
  onAttached: (attachment: UploadedAttachment) => void
}

const MAX_SIZE_MB = 12

export function AttachmentButton({ projectId, disabled, onAttached }: Props) {
  const inputRef = useRef<HTMLInputElement>(null)
  const [uploading, setUploading] = useState(false)
  const [error, setError] = useState('')

  async function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!inputRef.current) inputRef.current!.value = ''   // reset 允许同文件重选
    if (!file) return

    if (file.size > MAX_SIZE_MB * 1024 * 1024) {
      setError(`文件最大 ${MAX_SIZE_MB} MB`)
      return
    }
    setError('')
    setUploading(true)
    try {
      const mime = file.type || 'application/octet-stream'
      const kind = mime.startsWith('image/') ? 'image' : 'attachment'
      const url = `/api/projects/${encodeURIComponent(projectId)}/attachments`
        + `?file_name=${encodeURIComponent(file.name)}`
        + `&display_name=${encodeURIComponent(file.name)}`
        + `&mime_type=${encodeURIComponent(mime)}`
        + `&kind=${encodeURIComponent(kind)}`
      const arrayBuffer = await file.arrayBuffer()
      const token = getAuthToken()
      const res = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': mime,
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: arrayBuffer,
      })
      if (!res.ok) {
        const err = await res.json().catch(() => ({})) as Record<string, unknown>
        throw new Error(String(err.error ?? err.message ?? `HTTP ${res.status}`))
      }
      const data = await res.json() as UploadedAttachment
      onAttached(data)
    } catch (err) {
      setError((err as Error).message ?? '上传失败')
    } finally {
      setUploading(false)
      if (inputRef.current) inputRef.current.value = ''
    }
  }

  return (
    <div className={styles.wrap}>
      <input
        ref={inputRef}
        type="file"
        className={styles.hiddenInput}
        accept="image/*,.pdf,.txt,.md,.json,.csv,.zip"
        onChange={handleFileChange}
        disabled={disabled || uploading}
      />
      <button
        className={styles.btn}
        type="button"
        title={uploading ? '上传中…' : '添加附件'}
        disabled={disabled || uploading}
        onClick={() => inputRef.current?.click()}
      >
        {uploading ? '↑' : '📎'}
      </button>
      {error && <div className={styles.error}>{error}</div>}
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
      {!isImage && <span className={styles.chipIcon}>📄</span>}
      <span className={styles.chipName}>{attachment.display_name}</span>
      <span className={styles.chipSize}>{sizeKB} KB</span>
      <button className={styles.chipRemove} onClick={onRemove} type="button" title="删除">×</button>
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
