import { resolveApiUrl } from '../../../api/runtime'
import type { PrivateUploadFile } from '../../user-browser/privateAttachmentUpload'

export interface GroupAiAttachment {
  message_id: string; attachment_id: string; name: string; mime_type: string
  size_bytes: number; download_path: string; sha256?: string | null
}

export function groupAttachmentFiles(manifest: GroupAiAttachment[]): PrivateUploadFile[] {
  return manifest.map(file => {
    const path = file.download_path
    if (!/^\/api\/user\/[^/]+\/chat-attachments\/[^/]+\/[^/?#]+$/.test(path)
      || /%2f|%5c|%00|%25/i.test(path) || path.includes('\\') || path.split('/').some(p => p === '.' || p === '..'))
      throw new Error('附件地址无效，未发送给 AI')
    return { name: file.name, type: file.mime_type, size: file.size_bytes, sha256: file.sha256,
      async load() {
        const controller = new AbortController(), timer = setTimeout(() => controller.abort(), 45000)
        try {
          // Never forward app tokens, cookies or a server-supplied origin to an attachment.
          const response = await fetch(resolveApiUrl(path), { signal: controller.signal, redirect: 'error', credentials: 'omit' })
          if (!response.ok || !response.body) throw new Error('无法读取所选附件，未发送给 AI')
          const reader = response.body.getReader(), parts: Uint8Array<ArrayBuffer>[] = []
          let length = 0
          try {
            for (;;) {
              const { done, value } = await reader.read()
              if (done) break
              length += value.length
              if (length > file.size_bytes || length > 8388608) throw new Error('附件大小已变化，未发送给 AI')
              parts.push(new Uint8Array(value))
            }
          } finally { await reader.cancel().catch(() => {}); reader.releaseLock() }
          if (length !== file.size_bytes) throw new Error('附件未完整下载，未发送给 AI')
          return new Blob(parts, { type: file.mime_type })
        } finally { clearTimeout(timer) }
      },
    }
  })
}
